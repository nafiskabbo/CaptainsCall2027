//! End-to-end harness: exposes the real command surface over HTTP so the real
//! React UI can be driven from a browser and observed.
//!
//! Lives under `examples/` so it never ships in the app bundle.
//!
//! ```text
//! cargo run --example e2e -- 127.0.0.1:8787 target/e2e
//! ```
//!
//! The browser shim (`e2e.html` + `src/mock-e2e.ts`) turns every
//! `window.__TAURI_INTERNALS__.invoke(cmd, args)` into a POST here, so the UI
//! exercises the same engine, `CareerView` shapes and validation rules the
//! packaged app runs. Serves one request at a time; that matches how a single
//! user's UI talks to a single backend.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use captains_call_2027_lib::career::Career;
use captains_call_2027_lib::commands::AppState;
use captains_call_2027_lib::db::TrainingFocus;
use captains_call_2027_lib::world::{load_world, DB_VERSION};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let addr = args.get(1).cloned().unwrap_or_else(|| "127.0.0.1:8787".into());
    let data_dir = PathBuf::from(args.get(2).cloned().unwrap_or_else(|| "target/e2e".into()));
    let _ = std::fs::remove_dir_all(&data_dir);
    let _ = std::fs::create_dir_all(&data_dir);
    std::env::set_var("CC_E2E_DIR", &data_dir);

    let listener = std::net::TcpListener::bind(&addr).expect("bind");
    eprintln!("[e2e] listening on http://{addr} (data: {})", data_dir.display());

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let Some(req) = read_request(&mut stream) else {
            continue;
        };
        let response = if req.path == "/health" {
            br#"{"ok":true,"data":"up"}"#.to_vec()
        } else if req.path == "/invoke" {
            invoke(&req.body)
        } else {
            br#"{"ok":false,"error":"not found"}"#.to_vec()
        };
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             Access-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: content-type\r\n\
             Access-Control-Allow-Methods: POST, GET, OPTIONS\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n",
            response.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&response);
    }
}

// ------------------------------------------------------------------ shared state

static STATE: OnceLock<AppState> = OnceLock::new();

fn state() -> &'static AppState {
    STATE.get_or_init(|| AppState::new(load_world()))
}

fn world() -> &'static captains_call_2027_lib::World {
    &state().world
}

/// Career slot, guarded by the same mutex the app uses.
fn careers() -> &'static Mutex<Option<Career>> {
    &state().career
}

/// A locked career. Holds the mutex guard and derefs to `Career`, so callers
/// get a normal-looking `career` binding without leaking the borrow.
/// `MutexGuard::map` is unstable, hence the wrapper rather than a projection.
struct CareerHandle(std::sync::MutexGuard<'static, Option<Career>>);

impl std::ops::Deref for CareerHandle {
    type Target = Career;
    fn deref(&self) -> &Career {
        self.0.as_ref().expect("career checked before locking handle")
    }
}

impl std::ops::DerefMut for CareerHandle {
    fn deref_mut(&mut self) -> &mut Career {
        self.0.as_mut().expect("career checked before locking handle")
    }
}

/// Locks the career slot; `Err` when no career is running.
fn career() -> Result<CareerHandle, String> {
    let guard = careers().lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        return Err("no career in progress".into());
    }
    Ok(CareerHandle(guard))
}

fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("CC_E2E_DIR").unwrap_or_else(|_| "target/e2e".into()))
}

// ---------------------------------------------------------------------- HTTP bits

struct HttpRequest {
    path: String,
    body: Vec<u8>,
}

fn read_request(stream: &mut std::net::TcpStream) -> Option<HttpRequest> {
    let mut buf = Vec::with_capacity(8192);
    let mut chunk = [0u8; 8192];
    loop {
        let Ok(n) = stream.read(&mut chunk) else { return None };
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find(&buf, b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buf[..pos]).to_string();
            let mut lines = head.lines();
            let request_line = lines.next()?;
            let path = request_line.split_whitespace().nth(1)?.to_string();
            let content_length = lines
                .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
                .and_then(|l| l.split(':').nth(1))
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            let mut body = buf[pos + 4..].to_vec();
            while body.len() < content_length {
                let Ok(n) = stream.read(&mut chunk) else { break };
                if n == 0 {
                    break;
                }
                body.extend_from_slice(&chunk[..n]);
            }
            body.truncate(content_length);
            return Some(HttpRequest { path, body });
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

// ---------------------------------------------------------------- command routing

fn invoke(body: &[u8]) -> Vec<u8> {
    let value: serde_json::Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => return err(format!("bad request body: {e}")),
    };
    let cmd = value.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
    let args = value.get("args").cloned().unwrap_or(serde_json::json!({}));

    match dispatch(cmd, &args) {
        Ok(data) => {
            serde_json::to_vec(&serde_json::json!({ "ok": true, "data": data })).unwrap_or_default()
        }
        Err(e) => err(e),
    }
}

fn err(message: String) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({ "ok": false, "error": message })).unwrap_or_default()
}

fn arg_str(args: &serde_json::Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing string arg `{key}`"))
}

fn arg_u32(args: &serde_json::Value, key: &str) -> Result<u32, String> {
    let v = args.get(key).ok_or_else(|| format!("missing number arg `{key}`"))?;
    v.as_u64()
        .or_else(|| v.as_f64().map(|f| f as u64))
        .map(|n| n as u32)
        .ok_or_else(|| format!("arg `{key}` is not a number"))
}

fn arg_u64(args: &serde_json::Value, key: &str) -> Result<u64, String> {
    let v = args.get(key).ok_or_else(|| format!("missing number arg `{key}`"))?;
    v.as_u64()
        .or_else(|| v.as_f64().map(|f| f as u64))
        .ok_or_else(|| format!("arg `{key}` is not a number"))
}

fn arg_i64(args: &serde_json::Value, key: &str) -> Result<i64, String> {
    let v = args.get(key).ok_or_else(|| format!("missing number arg `{key}`"))?;
    v.as_i64()
        .or_else(|| v.as_f64().map(|f| f as i64))
        .ok_or_else(|| format!("arg `{key}` is not a number"))
}

fn arg_bool(args: &serde_json::Value, key: &str) -> Result<bool, String> {
    args.get(key)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| format!("missing bool arg `{key}`"))
}

fn arg_json<T: serde::de::DeserializeOwned>(args: &serde_json::Value, key: &str) -> Result<T, String> {
    args.get(key)
        .cloned()
        .ok_or_else(|| format!("missing object arg `{key}`"))
        .and_then(|v| serde_json::from_value(v).map_err(|e| format!("bad `{key}`: {e}")))
}

/// Mirrors `lib.rs`'s `generate_handler!` list 1:1. A command the UI calls but
/// this router forgets returns an error instead of hanging the UI.
fn dispatch(cmd: &str, args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let dir = data_dir();

    match cmd {
        // ---- app info ----
        "get_app_info" => Ok(serde_json::json!({
            "name": "Captain's Call 2027",
            "version": env!("CARGO_PKG_VERSION"),
            "phase": "G1 playable",
        })),
        "get_world" => Ok(serde_json::json!(world())),
        "get_db_version" => Ok(serde_json::json!(DB_VERSION.to_string())),
        "get_squad" => {
            let team_id = arg_u32(args, "teamId")?;
            Ok(serde_json::json!(squad_for(team_id)))
        }
        "auto_select" => {
            let team_id = arg_u32(args, "teamId")?;
            let format = arg_json(args, "format")?;
            let team = world()
                .teams
                .iter()
                .find(|t| t.id == team_id)
                .cloned()
                .ok_or_else(|| format!("team {team_id} not found"))?;
            Ok(serde_json::json!(
                captains_call_2027_lib::match_::TeamSelection::auto_for(
                    &team,
                    &world().players,
                    format
                )
            ))
        }

        // ---- career ----
        "start_career" => {
            let name = arg_str(args, "name")?;
            let team_id = arg_u32(args, "teamId")?;
            let seed = arg_u64(args, "seed")?;
            let first_season = arg_u32(args, "firstSeason").unwrap_or(1);
            let reset_form = args
                .get("resetForm")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let career = Career::start_with_options(
                world(),
                team_id,
                name,
                seed,
                first_season,
                reset_form,
            )?;
            let view = captains_call_2027_lib::commands::build_view(&career, world());
            *careers().lock().map_err(|e| e.to_string())? = Some(career);
            Ok(serde_json::json!(view))
        }
        "get_career" => {
            let career = career()?;
            Ok(serde_json::json!(captains_call_2027_lib::commands::build_view(
                &career, world()
            )))
        }
        "save_career" => {
            let conn = state().saves_in(&dir)?;
            let career = career()?;
            Ok(serde_json::json!(
                captains_call_2027_lib::db::save(&conn, &career.state)
                    .map_err(|e| e.to_string())?
            ))
        }
        "list_saves" => {
            let conn = state().saves_in(&dir)?;
            Ok(serde_json::json!(
                captains_call_2027_lib::db::list(&conn).map_err(|e| e.to_string())?
            ))
        }
        "load_career" => {
            let id = arg_i64(args, "id")?;
            let conn = state().saves_in(&dir)?;
            let (loaded, matches_db) =
                captains_call_2027_lib::db::load_checked(&conn, id, DB_VERSION)
                    .map_err(|e| e.to_string())?;
            let mismatch = if matches_db {
                None
            } else {
                Some(format!(
                    "This save uses database {} but {} is installed.",
                    loaded.db_version, DB_VERSION
                ))
            };
            let players = if loaded.players.is_empty() {
                world().players.clone()
            } else {
                loaded.players.clone()
            };
            let records = loaded.records.clone();
            let career = Career {
                state: loaded,
                players,
                live: None,
                last_snapshot: None,
                records,
            };
            let mut view = captains_call_2027_lib::commands::build_view(&career, world());
            view.db_version_warning = mismatch;
            *careers().lock().map_err(|e| e.to_string())? = Some(career);
            Ok(serde_json::json!(view))
        }
        "delete_save" => {
            let id = arg_i64(args, "id")?;
            let conn = state().saves_in(&dir)?;
            captains_call_2027_lib::db::delete(&conn, id).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({}))
        }
        "advance_day" => {
            let mut career = career()?;
            career.advance_day(world());
            Ok(serde_json::json!(captains_call_2027_lib::commands::build_view(
                &career, world()
            )))
        }
        "advance_one_day" => {
            let mut career = career()?;
            career.advance_one_day(world());
            Ok(serde_json::json!(captains_call_2027_lib::commands::build_view(
                &career, world()
            )))
        }
        "advance_to_my_match" => {
            let mut career = career()?;
            career.advance_to_my_next_match(world());
            Ok(serde_json::json!(captains_call_2027_lib::commands::build_view(
                &career, world()
            )))
        }
        "start_new_season" => {
            let mut career = career()?;
            let season = career.state.season;
            let report = captains_call_2027_lib::core::season::roll_season(
                &mut career.state,
                world(),
                season,
            );
            career.state.season = season + 1;
            career.players = career.state.players.clone();
            for headline in report.retired.iter().take(3) {
                career.state.news.insert(
                    0,
                    captains_call_2027_lib::db::NewsItem {
                        day: 1,
                        headline: format!("{headline} has retired"),
                    },
                );
            }
            for headline in report.intake.iter().take(3) {
                career.state.news.insert(
                    0,
                    captains_call_2027_lib::db::NewsItem {
                        day: 1,
                        headline: format!("{headline} joins from the academy"),
                    },
                );
            }
            Ok(serde_json::json!(captains_call_2027_lib::commands::build_view(
                &career, world()
            )))
        }
        "get_transfer_pool" => {
            let career = career()?;
            let seed = career.state.seed ^ ((career.state.season as u64) << 20);
            Ok(serde_json::json!(
                captains_call_2027_lib::core::transfers::build_transfer_pool(
                    world(),
                    &career.players,
                    career.state.player_team_id,
                    seed
                )
            ))
        }
        "bid_for_player" => {
            let player_id = arg_u32(args, "playerId")?;
            let offered_wage = arg_u32(args, "offeredWage")?;
            let mut career = career()?;
            let seed = career.state.seed ^ ((career.state.season as u64) << 20);
            let listing = captains_call_2027_lib::core::transfers::build_transfer_pool(
                world(),
                &career.players,
                career.state.player_team_id,
                seed,
            )
            .into_iter()
            .find(|l| l.player_id == player_id)
            .ok_or_else(|| format!("player {player_id} is not in the transfer pool"))?;
            let team_id = career.state.player_team_id;
            let season_day = career.state.season_day;
            let team_name = career.state.player_team_name.clone();
            let budget = career.state.budget.unwrap_or(1_000_000) as u64;
            let outcome = captains_call_2027_lib::core::transfers::place_bid(
                world(),
                &mut career.players,
                team_id,
                &listing,
                offered_wage,
                budget,
            );
            if matches!(
                outcome,
                captains_call_2027_lib::core::transfers::BidOutcome::Accepted { .. }
            ) {
                let wage_bill = captains_call_2027_lib::core::transfers::wage_bill(
                    &career.players,
                    team_id,
                );
                career.state.players = career.players.clone();
                career.state.budget = Some((budget as i64 - wage_bill as i64).max(0));
                let name = listing.name.clone();
                career.state.news.insert(
                    0,
                    captains_call_2027_lib::db::NewsItem {
                        day: season_day,
                        headline: format!("{} signs for {}", name, team_name),
                    },
                );
            }
            Ok(serde_json::json!(outcome))
        }
        "release_player" => {
            let player_id = arg_u32(args, "playerId")?;
            let mut career = career()?;
            let team_id = career.state.player_team_id;
            let before = captains_call_2027_lib::core::transfers::squad_size(
                &career.players,
                team_id,
            );
            if before <= 11 {
                return Err("you cannot reduce your squad below a full XI".into());
            }
            captains_call_2027_lib::core::transfers::release_player(
                &mut career.players,
                team_id,
                player_id,
            )?;
            career.state.players = career.players.clone();
            Ok(serde_json::json!({}))
        }
        "get_finances" => {
            let career = career()?;
            let team_id = career.state.player_team_id;
            let budget = career.state.budget.unwrap_or_else(|| {
                world()
                    .teams
                    .iter()
                    .find(|t| t.id == team_id)
                    .map(|t| t.budget)
                    .unwrap_or(1_000_000)
            });
            let wage_bill = captains_call_2027_lib::core::transfers::wage_bill(
                &career.players,
                team_id,
            );
            let squad_size =
                captains_call_2027_lib::core::transfers::squad_size(&career.players, team_id);
            Ok(serde_json::json!({
                "budget": budget,
                "wageBill": wage_bill,
                "squadSize": squad_size,
            }))
        }
        "set_training_focus" => {
            let focus = arg_json::<TrainingFocus>(args, "focus")?;
            let mut career = career()?;
            career.state.training_focus = focus;
            Ok(serde_json::json!(captains_call_2027_lib::commands::build_view(
                &career, world()
            )))
        }
        "get_update_state" => Ok(update_view(
            &captains_call_2027_lib::commands::load_settings_in(&dir),
        )),
        "set_update_settings" => {
            let enabled = arg_bool(args, "enabled")?;
            let channel = if args.get("channel").is_some() {
                Some(arg_str(args, "channel")?)
            } else {
                None
            };
            let mut settings = captains_call_2027_lib::commands::load_settings_in(&dir);
            settings.check_for_updates = enabled;
            if let Some(channel) = channel {
                settings.channel = match channel.as_str() {
                    "beta" => captains_call_2027_lib::core::updater::UpdateChannel::Beta,
                    "local" => captains_call_2027_lib::core::updater::UpdateChannel::Local,
                    _ => captains_call_2027_lib::core::updater::UpdateChannel::Stable,
                };
            }
            captains_call_2027_lib::commands::save_settings_in(&dir, &settings)?;
            Ok(update_view(&settings))
        }
        "check_local_update" => {
            let manifest_path = arg_str(args, "manifestPath")?;
            let settings = captains_call_2027_lib::commands::load_settings_in(&dir);
            let json_text = std::fs::read_to_string(&manifest_path)
                .map_err(|e| format!("could not read manifest: {e}"))?;
            let manifest: captains_call_2027_lib::core::updater::DbManifest =
                serde_json::from_str(&json_text)
                    .map_err(|e| format!("manifest is not valid JSON: {e}"))?;
            let status = captains_call_2027_lib::core::updater::evaluate_update(
                DB_VERSION,
                Some(&manifest),
                &settings,
                true,
            );
            Ok(serde_json::json!({
                "installed": DB_VERSION.to_string(),
                "settings": settings,
                "status": captains_call_2027_lib::commands::UpdateStatusView::from(status),
            }))
        }

        // ---- selection ----
        "get_selection" => {
            let fixture_id = arg_u32(args, "fixtureId")?;
            let career = career()?;
            let selection = career
                .selection_for(world(), fixture_id)
                .ok_or_else(|| format!("fixture {fixture_id} not found"))?;
            Ok(serde_json::json!(selection))
        }
        "set_selection" => {
            let fixture_id = arg_u32(args, "fixtureId")?;
            let selection = arg_json(args, "selection")?;
            let mut career = career()?;
            career.set_selection(world(), fixture_id, selection)?;
            Ok(serde_json::json!({}))
        }

        // ---- live match ----
        "start_fixture" => {
            let fixture_id = arg_u32(args, "fixtureId")?;
            let mut career = career()?;
            Ok(serde_json::json!(career.start_fixture(world(), fixture_id)?))
        }
        "get_match" => {
            let career = career().ok();
            Ok(serde_json::json!(career.as_deref().and_then(|c| c.live_snapshot())))
        }
        "set_match_input" => {
            let input = arg_json(args, "input")?;
            let mut career = career()?;
            if let Some(live) = career.live.as_mut() {
                live.sim.input = input;
            }
            Ok(serde_json::json!({}))
        }
        "sim_match" => {
            let count = arg_u32(args, "count")?;
            let mut career = career()?;
            Ok(serde_json::json!(career.advance(count)))
        }
        "sim_innings" => {
            let mut career = career()?;
            Ok(serde_json::json!(career.advance_innings()))
        }
        "export_scorecard" => {
            let career = career()?;
            let snapshot = career
                .live
                .as_ref()
                .map(|l| l.snapshot())
                .or_else(|| career.last_snapshot.clone())
                .ok_or("no match has been played")?;
            let html = captains_call_2027_lib::commands::almanack::scorecard_html(
                &snapshot, world(),
            );
            let out = dir.join("scorecards");
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            let path = out.join(format!("scorecard-{}.html", snapshot.match_id));
            std::fs::write(&path, html).map_err(|e| e.to_string())?;
            Ok(serde_json::json!(path.display().to_string()))
        }

        other => Err(format!("unknown command `{other}`")),
    }
}

/// Squad lookup, mirroring `get_squad`'s national-team handling.
fn squad_for(team_id: u32) -> Vec<captains_call_2027_lib::Player> {
    let players = career()
        .ok()
        .map(|c| c.players.clone())
        .unwrap_or_else(|| world().players.clone());
    let is_national = world()
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .map(|t| t.is_national)
        .unwrap_or(false);
    players
        .into_iter()
        .filter(|p| {
            if is_national {
                p.nation_id == team_id
            } else {
                p.team_id == Some(team_id)
            }
        })
        .collect()
}

fn update_view(settings: &captains_call_2027_lib::core::updater::UpdateSettings) -> serde_json::Value {
    let installed = DB_VERSION.to_string();
    serde_json::json!({
        "installed": installed,
        "settings": settings,
        "status": { "kind": "upToDate", "installed": installed },
    })
}
