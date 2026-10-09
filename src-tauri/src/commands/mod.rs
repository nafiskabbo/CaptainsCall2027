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
    state
        .world
        .players
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
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CareerView {
    pub state: CareerState,
    pub table: Vec<TableRow>,
    pub next_fixture_id: Option<u32>,
    pub season_complete: bool,
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
pub fn list_saves(app: tauri::AppHandle, state: SharedState<'_>) -> Result<Vec<SaveSummary>, String> {
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
    let loaded = db::load(&conn, id).map_err(|e| e.to_string())?;
    let career = Career {
        state: loaded,
        live: None,
        last_snapshot: None,
    };
    let view = build_view(&career, &state.world);
    *state.career.lock().map_err(|e| e.to_string())? = Some(career);
    Ok(view)
}

#[tauri::command]
pub fn delete_save(app: tauri::AppHandle, state: SharedState<'_>, id: i64) -> Result<(), String> {
    let conn = state.saves(&app)?;
    db::delete(&conn, id).map_err(|e| e.to_string())
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

// ---------------- helpers ----------------

fn build_view(career: &Career, world: &World) -> CareerView {
    CareerView {
        state: career.state.clone(),
        table: career.table(world),
        next_fixture_id: career.next_fixture().map(|f| f.id),
        season_complete: career.season_complete(),
    }
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
