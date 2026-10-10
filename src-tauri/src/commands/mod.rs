//! Career mode commands (G1): start/continue a career, play fixtures, save.

use crate::career::{Career, LiveMatch};
use crate::core::scheduler::TableRow;
use crate::core::types::{Format, World};
use crate::db::{self, CareerState, SaveSummary};
use crate::match_::state::{MatchPhase, MatchSnapshot};
use crate::match_::MatchInput;
use crate::world::load_world;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Manager, State};

/// App state holding the world, the active career and the live match.
///
/// `world` is mutable so `load_db_pack` can swap the active database between
/// careers; every read takes a short-lived borrow.
pub struct AppState {
    pub world: std::sync::RwLock<World>,
    pub career: Mutex<Option<Career>>,
}

impl AppState {
    pub fn new(world: World) -> Self {
        AppState {
            world: std::sync::RwLock::new(world),
            career: Mutex::new(None),
        }
    }

    /// The currently loaded database.
    ///
    /// Cloned rather than borrowed so commands never hold the lock across an
    /// `await`/IPC boundary.
    pub fn world(&self) -> World {
        self.world
            .read()
            .map(|w| w.clone())
            .unwrap_or_else(|_| crate::world::load_world())
    }

    /// Replace the active database.
    pub fn set_world(&self, world: World) -> Result<(), String> {
        let mut guard = self.world.write().map_err(|e| e.to_string())?;
        *guard = world;
        Ok(())
    }

    /// Opens the saves database under `dir` (reopened per call; SQLite handles
    /// concurrent readers fine and this keeps lifetimes simple).
    ///
    /// Takes a directory rather than an `AppHandle` so the same store can be
    /// pointed at a temp dir by the e2e harness and integration tests.
    pub fn saves_in(&self, dir: &std::path::Path) -> Result<rusqlite::Connection, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let conn = rusqlite::Connection::open(dir.join("saves.db"))
            .map_err(|e| format!("cannot open saves db: {e}"))?;
        crate::db::open_migrations(&conn).map_err(|e| e.to_string())?;
        Ok(conn)
    }
}

pub type SharedState<'a> = State<'a, AppState>;

pub fn init_state() -> AppState {
    AppState::new(load_world())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub phase: String,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: "Captain's Call 2027".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        phase: "G1 playable".into(),
    }
}

#[tauri::command]
pub fn get_world(state: SharedState<'_>) -> World {
    state.world()
}

/// A selectable database, as shown on the new-game screen.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackInfo {
    /// Stable key used by `load_db_pack`.
    pub key: String,
    pub label: String,
    pub description: String,
    pub db_version: String,
    pub nations: u32,
    pub teams: u32,
    pub players: u32,
    /// True when the pack uses real player/ground names rather than the
    /// fictional defaults, so the UI can label it as third-party test data.
    pub real_names: bool,
}

/// Every database the player can start a career from, default first.
#[tauri::command]
pub fn list_db_packs() -> Vec<PackInfo> {
    let default = crate::world::load_world();
    let mut out = vec![PackInfo {
        key: "default".into(),
        label: "Captain's Call 2027 (fictional)".into(),
        description: "The shipped database: twelve invented nations and their domestic sides."
            .into(),
        db_version: default.db_version.clone(),
        nations: default.nations.len() as u32,
        teams: default.teams.len() as u32,
        players: default.players.len() as u32,
        real_names: false,
    }];
    for pack in crate::world::PACKS {
        let world = crate::world::load_pack(pack.id);
        out.push(PackInfo {
            key: pack.id.key().to_string(),
            label: pack.id.label().to_string(),
            description: pack.id.description().to_string(),
            db_version: world.db_version.clone(),
            nations: world.nations.len() as u32,
            teams: world.teams.len() as u32,
            players: world.players.len() as u32,
            real_names: matches!(pack.id, crate::world::PackId::RealInternational),
        });
    }
    out
}

/// Load a database pack into the active session, replacing the current world.
///
/// Refused while a career is in progress: a career's fixtures, squads and
/// records all reference the world it was created from, so swapping mid-career
/// would leave the save inconsistent.
#[tauri::command]
pub fn load_db_pack(state: SharedState<'_>, key: String) -> Result<World, String> {
    {
        let guard = state.career.lock().map_err(|e| e.to_string())?;
        if let Some(career) = guard.as_ref() {
            if career.state.season_day > 1 || !career.state.fixtures.is_empty() {
                return Err("finish or delete the current career before changing database".into());
            }
        }
    }
    let world = if key == "default" {
        crate::world::load_world()
    } else {
        let Some(id) = crate::world::pack_from_key(&key) else {
            return Err(format!("unknown database pack: {key}"));
        };
        crate::world::load_pack(id)
    };
    if world.teams.is_empty() {
        return Err(format!("database pack '{key}' could not be loaded"));
    }
    // A career left over from the previous database would reference squads and
    // grounds that no longer exist, so clear the slot on every successful swap.
    *state.career.lock().map_err(|e| e.to_string())? = None;
    state.set_world(world.clone())?;
    Ok(world)
}

/// Abandon the career in progress without saving.
///
/// "Main Menu" has to clear the engine's career slot, not just the UI's: while
/// a career object is live the engine treats one as in progress, which blocks
/// changing the database and starting a fresh career.
#[tauri::command]
pub fn quit_to_menu(state: SharedState<'_>) -> Result<(), String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
pub fn get_db_version() -> String {
    crate::world::DB_VERSION.to_string()
}

#[tauri::command]
pub fn get_squad(state: SharedState<'_>, team_id: u32) -> Vec<crate::core::types::Player> {
    let world_ref = state.world();
    let career_players = state
        .career
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|c| c.players.clone()));
    let source: Vec<crate::core::types::Player> =
        career_players.unwrap_or_else(|| world_ref.players.clone());
    source
        .iter()
        .filter(|p| {
            if world_ref
                .teams
                .iter()
                .find(|t| t.id == team_id)
                .map(|t| t.is_national)
                .unwrap_or(false)
            {
                p.nation_id == team_id
            } else {
                p.team_id == Some(team_id)
            }
        })
        .cloned()
        .collect()
}

#[tauri::command]
pub fn auto_select(
    state: SharedState<'_>,
    team_id: u32,
    format: Format,
) -> Result<crate::match_::TeamSelection, String> {
    let world_ref = state.world();
    let team = world_ref
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .cloned()
        .ok_or_else(|| format!("team {team_id} not found"))?;
    let world_ref = state.world();
    Ok(crate::match_::TeamSelection::auto_for(
        &team,
        &world_ref.players,
        format,
    ))
}

// ---------------- career commands ----------------

/// Career summary the UI renders on its home screen.
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CareerRecordRow {
    pub name: String,
    pub matches: u32,
    pub runs: u32,
    pub average: Option<f64>,
    pub strike_rate: f64,
    pub wickets: u32,
    pub bowling_average: Option<f64>,
    pub economy: f64,
    pub hundreds: u32,
    pub fifties: u32,
    pub five_wicket_hauls: u32,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CareerView {
    pub state: CareerState,
    pub table: Vec<TableRow>,
    pub next_fixture_id: Option<u32>,
    pub season_complete: bool,
    pub international: bool,
    pub records: Vec<CareerRecordRow>,
    /// Set when a save was created against a different database version.
    pub db_version_warning: Option<String>,
}

#[tauri::command]
pub fn start_career(
    state: SharedState<'_>,
    name: String,
    team_id: u32,
    seed: u64,
    first_season: Option<u32>,
    reset_form: Option<bool>,
) -> Result<CareerView, String> {
    let world_ref = state.world();
    let career = Career::start_with_options(
        &world_ref,
        team_id,
        name,
        seed,
        first_season.unwrap_or(1),
        reset_form.unwrap_or(false),
    )?;
    let view = build_view(&career, &world_ref);
    *state.career.lock().map_err(|e| e.to_string())? = Some(career);
    Ok(view)
}

#[tauri::command]
pub fn get_career(state: SharedState<'_>) -> Result<CareerView, String> {
    let guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_ref().ok_or("no career in progress")?;
    let world_ref = state.world();
    Ok(build_view(career, &world_ref))
}

#[tauri::command]
pub fn save_career(app: tauri::AppHandle, state: SharedState<'_>) -> Result<i64, String> {
    let _ = &state;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir unavailable: {e}"))?;
    let conn = state.saves_in(&dir)?;
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    db::save(&conn, &career.state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_saves(
    app: tauri::AppHandle,
    state: SharedState<'_>,
) -> Result<Vec<SaveSummary>, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir unavailable: {e}"))?;
    let conn = state.saves_in(&dir)?;
    db::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_career(
    app: tauri::AppHandle,
    state: SharedState<'_>,
    id: i64,
) -> Result<CareerView, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir unavailable: {e}"))?;
    let conn = state.saves_in(&dir)?;
    // Warn (but still load) when the save came from a different dbVersion.
    let (loaded, matches_db) =
        db::load_checked(&conn, id, crate::world::DB_VERSION).map_err(|e| e.to_string())?;
    let mismatch = if matches_db {
        None
    } else {
        Some(format!(
            "This save uses database {} but {} is installed. It will load with the roster it was created from.",
            loaded.db_version,
            crate::world::DB_VERSION
        ))
    };
    // Saved careers carry their own developed player list; fall back to the
    // bundled world only for older saves that predate the players field.
    let world_ref = state.world();
    let players = if loaded.players.is_empty() {
        world_ref.players.clone()
    } else {
        loaded.players.clone()
    };
    // Keep the stats the save was created with, not a blank slate.
    let records = loaded.records.clone();
    let career = Career {
        state: loaded,
        players,
        live: None,
        last_snapshot: None,
        records,
    };
    let mut view = build_view(&career, &world_ref);
    view.db_version_warning = mismatch;
    *state.career.lock().map_err(|e| e.to_string())? = Some(career);
    Ok(view)
}

#[tauri::command]
pub fn delete_save(app: tauri::AppHandle, state: SharedState<'_>, id: i64) -> Result<(), String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir unavailable: {e}"))?;
    let conn = state.saves_in(&dir)?;
    db::delete(&conn, id).map_err(|e| e.to_string())
}

/// Roll into a new season: age players, retire, intake youth, reset fixtures.
#[tauri::command]
pub fn start_new_season(state: SharedState<'_>) -> Result<CareerView, String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    let season = career.state.season;
    let world_ref = state.world();
    let report = crate::core::season::roll_season(&mut career.state, &world_ref, season);
    career.state.season = season + 1;
    career.players = career.state.players.clone();
    for headline in report.retired.iter().take(3) {
        career.state.news.insert(
            0,
            crate::db::NewsItem {
                day: 1,
                headline: format!("{} has retired", headline),
            },
        );
    }
    for headline in report.intake.iter().take(3) {
        career.state.news.insert(
            0,
            crate::db::NewsItem {
                day: 1,
                headline: format!("{} joins from the academy", headline),
            },
        );
    }
    Ok(build_view(career, &world_ref))
}

/// Transfer-window listings for the player's club.
#[tauri::command]
pub fn get_transfer_pool(
    state: SharedState<'_>,
) -> Result<Vec<crate::core::transfers::TransferListing>, String> {
    let guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_ref().ok_or("no career in progress")?;
    let seed = career.state.seed ^ ((career.state.season as u64) << 20);
    let world_ref = state.world();
    Ok(crate::core::transfers::build_transfer_pool(
        &world_ref,
        &career.players,
        career.state.player_team_id,
        seed,
    ))
}

/// Bid for a listed player.
#[tauri::command]
pub fn bid_for_player(
    state: SharedState<'_>,
    player_id: u32,
    offered_wage: u32,
) -> Result<crate::core::transfers::BidOutcome, String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    let seed = career.state.seed ^ ((career.state.season as u64) << 20);
    let world_ref = state.world();
    let listing = crate::core::transfers::build_transfer_pool(
        &world_ref,
        &career.players,
        career.state.player_team_id,
        seed,
    )
    .into_iter()
    .find(|l| l.player_id == player_id)
    .ok_or_else(|| format!("player {player_id} is not in the transfer pool"))?;

    let budget = career.state.budget.unwrap_or(1_000_000) as u64;
    let outcome = crate::core::transfers::place_bid(
        &world_ref,
        &mut career.players,
        career.state.player_team_id,
        &listing,
        offered_wage,
        budget,
    );
    if matches!(outcome, crate::core::transfers::BidOutcome::Accepted { .. }) {
        career.state.players = career.players.clone();
        let wage_bill =
            crate::core::transfers::wage_bill(&career.players, career.state.player_team_id);
        career.state.budget = Some((budget as i64 - wage_bill as i64).max(0));
        let name = listing.name.clone();
        career.state.news.insert(
            0,
            crate::db::NewsItem {
                day: career.state.season_day,
                headline: format!(
                    "{} signs for {}",
                    name,
                    career.state.player_team_name.clone()
                ),
            },
        );
    }
    Ok(outcome)
}

/// Release a player from your squad.
#[tauri::command]
pub fn release_player(state: SharedState<'_>, player_id: u32) -> Result<(), String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    let team_id = career.state.player_team_id;
    let before = crate::core::transfers::squad_size(&career.players, team_id);
    if before <= 11 {
        return Err("you cannot reduce your squad below a full XI".into());
    }
    crate::core::transfers::release_player(&mut career.players, team_id, player_id)?;
    career.state.players = career.players.clone();
    Ok(())
}

/// Club finances: budget, wage bill, squad size.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finances {
    pub budget: i64,
    pub wage_bill: u32,
    pub squad_size: usize,
}

#[tauri::command]
pub fn get_finances(state: SharedState<'_>) -> Result<Finances, String> {
    let world_ref = state.world();
    let guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_ref().ok_or("no career in progress")?;
    let team_id = career.state.player_team_id;
    Ok(Finances {
        budget: career.state.budget.unwrap_or_else(|| {
            world_ref
                .teams
                .iter()
                .find(|t| t.id == team_id)
                .map(|t| t.budget)
                .unwrap_or(1_000_000)
        }),
        wage_bill: crate::core::transfers::wage_bill(&career.players, team_id),
        squad_size: crate::core::transfers::squad_size(&career.players, team_id),
    })
}

/// Set the squad's training focus for the season.
#[tauri::command]
pub fn set_training_focus(
    state: SharedState<'_>,
    focus: crate::db::TrainingFocus,
) -> Result<CareerView, String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    career.state.training_focus = focus;
    let world_ref = state.world();
    Ok(build_view(career, &world_ref))
}

/// Skip to the next match day, simulating other fixtures.
#[tauri::command]
pub fn advance_day(state: SharedState<'_>) -> Result<CareerView, String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    let world_ref = state.world();
    career.advance_day(&world_ref);
    Ok(build_view(career, &world_ref))
}

/// Step the calendar by a single day (««): always moves, simulates that day.
#[tauri::command]
pub fn advance_one_day(state: SharedState<'_>) -> Result<CareerView, String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    let world_ref = state.world();
    career.advance_one_day(&world_ref);
    Ok(build_view(career, &world_ref))
}

/// Skip straight to the player's next fixture (»»), simulating the rounds
/// in between. Returns the view unchanged when the season has none left.
#[tauri::command]
pub fn advance_to_my_match(state: SharedState<'_>) -> Result<CareerView, String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    let world_ref = state.world();
    career.advance_to_my_next_match(&world_ref);
    Ok(build_view(career, &world_ref))
}

// ---------------- live match commands ----------------

/// Read the XI the player has saved for a fixture (or the auto-pick).
#[tauri::command]
pub fn get_selection(
    state: SharedState<'_>,
    fixture_id: u32,
) -> Result<crate::match_::TeamSelection, String> {
    let guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_ref().ok_or("no career in progress")?;
    let world_ref = state.world();
    career
        .selection_for(&world_ref, fixture_id)
        .ok_or_else(|| format!("fixture {fixture_id} not found"))
}

/// Save an XI for a fixture. Validated on the Rust side.
#[tauri::command]
pub fn set_selection(
    state: SharedState<'_>,
    fixture_id: u32,
    selection: crate::match_::TeamSelection,
) -> Result<(), String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    let world_ref = state.world();
    career.set_selection(&world_ref, fixture_id, selection)
}

#[tauri::command]
pub fn start_fixture(state: SharedState<'_>, fixture_id: u32) -> Result<MatchSnapshot, String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    let world_ref = state.world();
    career.start_fixture(&world_ref, fixture_id)
}

#[tauri::command]
pub fn get_match(state: SharedState<'_>) -> Option<MatchSnapshot> {
    state.career.lock().ok()?.as_ref()?.live_snapshot()
}

#[tauri::command]
pub fn set_match_input(state: SharedState<'_>, input: MatchInput) -> Result<(), String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    if let Some(live) = career.live.as_mut() {
        live.sim.input = input;
    }
    Ok(())
}

/// Outcome names in the engine's array order.
const NAMES: [&str; 9] = [
    "dot", "one", "two", "three", "four", "six", "out", "wide", "no_ball",
];

/// The chance of each outcome on the next delivery, for the shot picker.
///
/// Keyed by outcome name rather than returned as an array, so the UI reads
/// `odds.four` and not `odds[4]` — the engine's array order is an
/// implementation detail that should not leak into the interface.
#[tauri::command]
pub fn preview_ball(state: SharedState<'_>) -> Result<HashMap<String, f64>, String> {
    let guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_ref().ok_or("no career in progress")?;
    let live = career.live.as_ref().ok_or("no live match")?;
    let p = live.sim.preview_ball();
    Ok(NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| ((*n).to_string(), p[i]))
        .collect())
}

/// Advance N balls in the live fixture; auto-records the result at the end.
#[tauri::command]
pub fn sim_match(state: SharedState<'_>, count: u32) -> Result<MatchSnapshot, String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    Ok(career.advance(count))
}

/// Fast-forward to the end of the current innings.
#[tauri::command]
pub fn sim_innings(state: SharedState<'_>) -> Result<MatchSnapshot, String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    Ok(career.advance_innings())
}

/// Write the scorecard of the last match to disk and return its path.
///
/// Works during a match and after it, using the live snapshot or the last
/// completed one.
#[tauri::command]
pub fn export_scorecard(app: tauri::AppHandle, state: SharedState<'_>) -> Result<String, String> {
    let guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_ref().ok_or("no career in progress")?;
    let snapshot = career
        .live
        .as_ref()
        .map(|l| l.snapshot())
        .or_else(|| career.last_snapshot.clone())
        .ok_or("no match has been played")?;

    let world_ref = state.world();
    let html = crate::commands::almanack::scorecard_html(&snapshot, &world_ref);

    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir unavailable: {e}"))?;
    let dir = base.join("scorecards");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("scorecard-{}.html", snapshot.match_id));
    std::fs::write(&path, html).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

// ---------------- database update channel ----------------

/// Current update settings and the installed db version.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStateView {
    pub installed: String,
    pub settings: crate::core::updater::UpdateSettings,
    /// What the user would be offered right now, if they asked.
    pub status: UpdateStatusView,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "kind")]
pub enum UpdateStatusView {
    UpToDate {
        installed: String,
    },
    Available {
        version: String,
        changelog: String,
        bytes: u64,
        channel: String,
    },
    Unreachable {
        installed: String,
    },
    Rejected {
        installed: String,
        reason: String,
    },
}

impl From<crate::core::updater::UpdateStatus> for UpdateStatusView {
    fn from(status: crate::core::updater::UpdateStatus) -> Self {
        use crate::core::updater::UpdateStatus as S;
        match status {
            S::UpToDate { installed } => UpdateStatusView::UpToDate { installed },
            S::Available { manifest, .. } => UpdateStatusView::Available {
                version: manifest.db_version.clone(),
                changelog: manifest.changelog.clone(),
                bytes: manifest.bytes,
                channel: manifest.channel.label().to_string(),
            },
            S::Unreachable { installed } => UpdateStatusView::Unreachable { installed },
            S::Rejected { installed, reason } => UpdateStatusView::Rejected { installed, reason },
            S::ChannelMismatch => UpdateStatusView::Rejected {
                installed: String::new(),
                reason: "channel mismatch".into(),
            },
        }
    }
}

/// Update settings live next to the saves so they persist.
fn settings_path_in(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join("update-settings.json")
}

/// Settings file location for an explicit directory (harness/tests).
pub fn settings_path_for(dir: &std::path::Path) -> std::path::PathBuf {
    settings_path_in(dir)
}

/// Read update settings from an explicit directory.
pub fn load_settings_in(dir: &std::path::Path) -> crate::core::updater::UpdateSettings {
    std::fs::read_to_string(settings_path_in(dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Write update settings to an explicit directory.
pub fn save_settings_in(
    dir: &std::path::Path,
    settings: &crate::core::updater::UpdateSettings,
) -> Result<(), String> {
    let path = settings_path_in(dir);
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

/// Toggle the update check and switch channel.
#[tauri::command]
pub fn set_update_settings(
    app: tauri::AppHandle,
    enabled: bool,
    channel: Option<String>,
) -> Result<UpdateStateView, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir unavailable: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut settings = load_settings_in(&dir);
    settings.check_for_updates = enabled;
    if let Some(channel) = channel {
        settings.channel = match channel.as_str() {
            "beta" => crate::core::updater::UpdateChannel::Beta,
            "local" => crate::core::updater::UpdateChannel::Local,
            _ => crate::core::updater::UpdateChannel::Stable,
        };
    }
    save_settings_in(&dir, &settings)?;
    Ok(UpdateStateView {
        installed: crate::world::DB_VERSION.to_string(),
        settings,
        status: UpdateStatusView::UpToDate {
            installed: crate::world::DB_VERSION.to_string(),
        },
    })
}

/// Read the current state of the update channel.
#[tauri::command]
pub fn get_update_state(app: tauri::AppHandle) -> Result<UpdateStateView, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir unavailable: {e}"))?;
    let settings = load_settings_in(&dir);
    Ok(UpdateStateView {
        installed: crate::world::DB_VERSION.to_string(),
        settings,
        status: UpdateStatusView::UpToDate {
            installed: crate::world::DB_VERSION.to_string(),
        },
    })
}

/// Check a local manifest file (the offline-safe way to exercise this flow).
///
/// A feed URL fetch needs the HTTP permission and a real signature key, which
/// is M6.3; this covers the local-file channel today and validates the same
/// rules the remote path will use.
#[tauri::command]
pub fn check_local_update(
    app: tauri::AppHandle,
    manifest_path: String,
) -> Result<UpdateStateView, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir unavailable: {e}"))?;
    let settings = load_settings_in(&dir);
    let json = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("could not read manifest: {e}"))?;
    let manifest: crate::core::updater::DbManifest =
        serde_json::from_str(&json).map_err(|e| format!("manifest is not valid JSON: {e}"))?;
    let _installed = crate::world::DB_VERSION;

    let status = crate::core::updater::evaluate_update(
        crate::world::DB_VERSION,
        Some(&manifest),
        &settings,
        true,
    );
    Ok(UpdateStateView {
        installed: crate::world::DB_VERSION.to_string(),
        settings,
        status: status.into(),
    })
}

// ---------------- helpers ----------------

/// Career summary the UI renders on its home screen.
pub fn build_view(career: &Career, world: &World) -> CareerView {
    let international = world
        .teams
        .iter()
        .find(|t| t.id == career.state.player_team_id)
        .map(|t| t.is_national)
        .unwrap_or(false);
    CareerView {
        state: career.state.clone(),
        table: career.table(world),
        next_fixture_id: career.next_fixture().map(|f| f.id),
        season_complete: career.season_complete(),
        international,
        records: record_rows(career),
        db_version_warning: None,
    }
}

/// Records sorted by runs, for the career stats tab.
fn record_rows(career: &Career) -> Vec<CareerRecordRow> {
    let mut rows: Vec<CareerRecordRow> = career
        .records
        .players
        .values()
        .filter(|p| p.matches > 0)
        .map(|p| CareerRecordRow {
            name: p.name.clone(),
            matches: p.matches,
            runs: p.runs,
            average: p.batting_average(),
            strike_rate: p.strike_rate(),
            wickets: p.wickets,
            bowling_average: p.bowling_average(),
            economy: p.economy(),
            hundreds: p.hundreds,
            fifties: p.fifties,
            five_wicket_hauls: p.five_wicket_hauls,
        })
        .collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.runs));
    rows
}

/// Whether a snapshot is finished.
pub fn is_complete(snapshot: &MatchSnapshot) -> bool {
    snapshot.phase == MatchPhase::Complete
}

/// Exposed for tests: the live match wrapper.
pub fn live_match(career: &Career) -> Option<&LiveMatch> {
    career.live.as_ref()
}

pub mod almanack;
