//! Career mode commands (G1): start/continue a career, play fixtures, save.

use crate::career::{Career, LiveMatch};
use crate::core::scheduler::TableRow;
use crate::core::types::{Format, World};
use crate::db::{self, CareerState, SaveSummary};
use crate::match_::state::{MatchPhase, MatchSnapshot};
use crate::match_::MatchInput;
use crate::world::load_world;
use std::sync::Mutex;
use tauri::{Manager, State};

/// App state holding the world, the active career and the live match.
pub struct AppState {
    pub world: World,
    pub career: Mutex<Option<Career>>,
}

impl AppState {
    pub fn new(world: World) -> Self {
        AppState {
            world,
            career: Mutex::new(None),
        }
    }

    /// Opens the saves database under the app data dir (reopened per call;
    /// SQLite handles concurrent readers fine and this keeps lifetimes simple).
    fn saves(&self, app: &tauri::AppHandle) -> Result<rusqlite::Connection, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("app data dir unavailable: {e}"))?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
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
    state.world.clone()
}

#[tauri::command]
pub fn get_db_version() -> String {
    crate::world::DB_VERSION.to_string()
}

#[tauri::command]
pub fn get_squad(state: SharedState<'_>, team_id: u32) -> Vec<crate::core::types::Player> {
    let career_players = state
        .career
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|c| c.players.clone()));
    let source: Vec<crate::core::types::Player> =
        career_players.unwrap_or_else(|| state.world.players.clone());
    source
        .iter()
        .filter(|p| {
            if state
                .world
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
    let team = state
        .world
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .cloned()
        .ok_or_else(|| format!("team {team_id} not found"))?;
    Ok(crate::match_::TeamSelection::auto_for(
        &team,
        &state.world.players,
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
) -> Result<CareerView, String> {
    let career = Career::start(&state.world, team_id, name, seed)?;
    let view = build_view(&career, &state.world);
    *state.career.lock().map_err(|e| e.to_string())? = Some(career);
    Ok(view)
}

#[tauri::command]
pub fn get_career(state: SharedState<'_>) -> Result<CareerView, String> {
    let guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_ref().ok_or("no career in progress")?;
    Ok(build_view(career, &state.world))
}

#[tauri::command]
pub fn save_career(app: tauri::AppHandle, state: SharedState<'_>) -> Result<i64, String> {
    let _ = &state;
    let conn = state.saves(&app)?;
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    db::save(&conn, &career.state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_saves(
    app: tauri::AppHandle,
    state: SharedState<'_>,
) -> Result<Vec<SaveSummary>, String> {
    let conn = state.saves(&app)?;
    db::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_career(
    app: tauri::AppHandle,
    state: SharedState<'_>,
    id: i64,
) -> Result<CareerView, String> {
    let conn = state.saves(&app)?;
    // Warn (but still load) when the save came from a different dbVersion.
    let (loaded, matches_db) = db::load_checked(&conn, id, crate::world::DB_VERSION)
        .map_err(|e| e.to_string())?;
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
    let players = if loaded.players.is_empty() {
        state.world.players.clone()
    } else {
        loaded.players.clone()
    };
    let career = Career {
        state: loaded,
        players,
        live: None,
        last_snapshot: None,
        records: Default::default(),
    };
    let mut view = build_view(&career, &state.world);
    view.db_version_warning = mismatch;
    *state.career.lock().map_err(|e| e.to_string())? = Some(career);
    Ok(view)
}

#[tauri::command]
pub fn delete_save(app: tauri::AppHandle, state: SharedState<'_>, id: i64) -> Result<(), String> {
    let conn = state.saves(&app)?;
    db::delete(&conn, id).map_err(|e| e.to_string())
}

/// Roll into a new season: age players, retire, intake youth, reset fixtures.
#[tauri::command]
pub fn start_new_season(state: SharedState<'_>) -> Result<CareerView, String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    let season = career.state.season;
    let report = crate::core::season::roll_season(&mut career.state, &state.world, season);
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
    Ok(build_view(career, &state.world))
}

/// Transfer-window listings for the player's club.
#[tauri::command]
pub fn get_transfer_pool(
    state: SharedState<'_>,
) -> Result<Vec<crate::core::transfers::TransferListing>, String> {
    let guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_ref().ok_or("no career in progress")?;
    let seed = career.state.seed ^ ((career.state.season as u64) << 20);
    Ok(crate::core::transfers::build_transfer_pool(
        &state.world,
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
    let listing = crate::core::transfers::build_transfer_pool(
        &state.world,
        &career.players,
        career.state.player_team_id,
        seed,
    )
    .into_iter()
    .find(|l| l.player_id == player_id)
    .ok_or_else(|| format!("player {player_id} is not in the transfer pool"))?;

    let budget = career.state.budget.unwrap_or(1_000_000) as u64;
    let outcome = crate::core::transfers::place_bid(
        &state.world,
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
    let guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_ref().ok_or("no career in progress")?;
    let team_id = career.state.player_team_id;
    Ok(Finances {
        budget: career.state.budget.unwrap_or_else(|| {
            state
                .world
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

/// Skip to the next match day, simulating other fixtures.
#[tauri::command]
pub fn advance_day(state: SharedState<'_>) -> Result<CareerView, String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    career.advance_day(&state.world);
    Ok(build_view(career, &state.world))
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
    career
        .selection_for(&state.world, fixture_id)
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
    career.set_selection(&state.world, fixture_id, selection)
}

#[tauri::command]
pub fn start_fixture(state: SharedState<'_>, fixture_id: u32) -> Result<MatchSnapshot, String> {
    let mut guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_mut().ok_or("no career in progress")?;
    career.start_fixture(&state.world, fixture_id)
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

#[tauri::command]
pub fn export_scorecard(state: SharedState<'_>) -> Result<String, String> {
    let guard = state.career.lock().map_err(|e| e.to_string())?;
    let career = guard.as_ref().ok_or("no career in progress")?;
    let live = career.live.as_ref().ok_or("no match in progress")?;
    Ok(crate::commands::almanack::scorecard_html(
        &live.snapshot(),
        &state.world,
    ))
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
fn settings_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir unavailable: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("update-settings.json"))
}

fn load_settings(app: &tauri::AppHandle) -> crate::core::updater::UpdateSettings {
    let Ok(path) = settings_path(app) else {
        return Default::default();
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings(
    app: &tauri::AppHandle,
    settings: &crate::core::updater::UpdateSettings,
) -> Result<(), String> {
    let path = settings_path(app)?;
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
    let mut settings = load_settings(&app);
    settings.check_for_updates = enabled;
    if let Some(channel) = channel {
        settings.channel = match channel.as_str() {
            "beta" => crate::core::updater::UpdateChannel::Beta,
            "local" => crate::core::updater::UpdateChannel::Local,
            _ => crate::core::updater::UpdateChannel::Stable,
        };
    }
    save_settings(&app, &settings)?;
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
    let settings = load_settings(&app);
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
    let settings = load_settings(&app);
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

fn build_view(career: &Career, world: &World) -> CareerView {
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
