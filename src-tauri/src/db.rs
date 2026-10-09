//! Save storage: SQLite-backed, atomic, db-version pinned.
//!
//! Stores career state as JSON inside SQLite. Saves record the `db_version`
//! they started with, so a DB update can never silently break an old save
//! (specs UR-5). WAL mode + single-statement commits keep writes atomic.

use crate::core::scheduler::{Fixture, FixtureResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    Json(serde_json::Error),
    Io(std::io::Error),
    NotFound(i64),
    Invalid(&'static str),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Sqlite(e) => write!(f, "database error: {e}"),
            StoreError::Json(e) => write!(f, "save data corrupt: {e}"),
            StoreError::Io(e) => write!(f, "io error: {e}"),
            StoreError::NotFound(id) => write!(f, "save {id} not found"),
            StoreError::Invalid(msg) => write!(f, "invalid save: {msg}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::Sqlite(e)
    }
}
impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        StoreError::Json(e)
    }
}
impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// A news line shown on the career home screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewsItem {
    pub day: u32,
    pub headline: String,
}

/// Full career state, serialized as JSON inside SQLite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CareerState {
    pub id: Option<i64>,
    pub name: String,
    pub player_team_id: u32,
    pub player_team_name: String,
    pub nation_id: u32,
    pub season_day: u32,
    pub db_version: String,
    /// Competition the player's team plays in.
    pub competition_id: u32,
    pub fixtures: Vec<Fixture>,
    pub news: Vec<NewsItem>,
    /// Player's XI per fixture (fixture_id -> selection). Falls back to auto-pick.
    #[serde(default)]
    pub selections: std::collections::BTreeMap<u32, crate::match_::TeamSelection>,
    /// The career's own copy of the players, so development persists across saves.
    #[serde(default)]
    pub players: Vec<crate::core::types::Player>,
    /// Club funds available for wages/bids in the transfer window.
    #[serde(default)]
    pub budget: Option<i64>,
    /// Which season of this career we are in (1 = first).
    #[serde(default = "default_season")]
    pub season: u32,
    pub seed: u64,
}

fn default_season() -> u32 {
    1
}

/// Runs the schema migrations on an existing connection.
pub fn open_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;

        CREATE TABLE IF NOT EXISTS saves (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            team_name   TEXT NOT NULL,
            season_day  INTEGER NOT NULL,
            db_version  TEXT NOT NULL,
            state       TEXT NOT NULL,
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_saves_updated ON saves(updated_at);
        "#,
    )?;
    Ok(())
}

/// Open (and migrate) the saves database at `path`.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))?;
    let conn = Connection::open(path)?;
    open_migrations(&conn)?;
    Ok(conn)
}

/// Insert or update a save slot (atomic per SQLite's transaction semantics).
pub fn save(conn: &Connection, state: &CareerState) -> Result<i64> {
    let json = serde_json::to_string(state)?;
    match state.id {
        Some(id) => {
            let changed = conn.execute(
                "UPDATE saves SET name = ?2, team_name = ?3, season_day = ?4,
                 db_version = ?5, state = ?6, updated_at = datetime('now') WHERE id = ?1",
                params![
                    id,
                    state.name,
                    state.player_team_name,
                    state.season_day,
                    state.db_version,
                    json
                ],
            )?;
            if changed == 0 {
                return Err(StoreError::NotFound(id));
            }
            Ok(id)
        }
        None => {
            conn.execute(
                "INSERT INTO saves (name, team_name, season_day, db_version, state)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    state.name,
                    state.player_team_name,
                    state.season_day,
                    state.db_version,
                    json
                ],
            )?;
            Ok(conn.last_insert_rowid())
        }
    }
}

pub fn load(conn: &Connection, id: i64) -> Result<CareerState> {
    let json: Option<String> = conn
        .query_row("SELECT state FROM saves WHERE id = ?1", params![id], |r| {
            r.get(0)
        })
        .optional()?;
    let json = json.ok_or(StoreError::NotFound(id))?;
    let mut state: CareerState = serde_json::from_str(&json)?;
    state.id = Some(id);
    Ok(state)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveSummary {
    pub id: i64,
    pub name: String,
    pub team_name: String,
    pub season_day: u32,
    pub db_version: String,
    pub updated_at: String,
}

pub fn list(conn: &Connection) -> Result<Vec<SaveSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, team_name, season_day, db_version, updated_at
         FROM saves ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(SaveSummary {
            id: r.get(0)?,
            name: r.get(1)?,
            team_name: r.get(2)?,
            season_day: r.get(3)?,
            db_version: r.get(4)?,
            updated_at: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    let changed = conn.execute("DELETE FROM saves WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(StoreError::NotFound(id));
    }
    Ok(())
}

/// Records a completed fixture into the career and returns the news it produced.
pub fn record_result(state: &mut CareerState, fixture_id: u32, result: FixtureResult) -> Vec<NewsItem> {
    let summary = result.summary.clone();
    let is_player_match = state
        .fixtures
        .iter()
        .find(|f| f.id == fixture_id)
        .map(|f| f.home_id == state.player_team_id || f.away_id == state.player_team_id)
        .unwrap_or(false);

    if let Some(fixture) = state.fixtures.iter_mut().find(|f| f.id == fixture_id) {
        fixture.result = Some(result.clone());
    }

    let mut news = Vec::new();
    if is_player_match {
        let headline = match result.winner_id {
            Some(id) if id == state.player_team_id => format!("Win! {}", summary),
            Some(_) => format!("Defeat — {}", summary),
            None => format!("Tie — {}", summary),
        };
        news.push(NewsItem {
            day: state.season_day,
            headline,
        });
    }
    state.news.splice(0..0, news.clone());
    news
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Format;

    fn career() -> CareerState {
        CareerState {
            id: None,
            name: "Test career".into(),
            player_team_id: 1,
            player_team_name: "Port Vale First XI".into(),
            nation_id: 1,
            season_day: 1,
            db_version: "2027.01".into(),
            competition_id: 1,
            fixtures: vec![Fixture {
                id: 10,
                competition_id: 1,
                home_id: 1,
                away_id: 2,
                format: Format::T20,
                ground_id: 1,
                day: 1,
                round: 0,
                result: None,
            }],
            news: vec![],
            selections: Default::default(),
            players: vec![],
            budget: None,
            season: 1,
            seed: 1,
        }
    }

    fn temp_db(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cc2027-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("saves.db")
    }

    #[test]
    fn save_and_load_round_trip() {
        let conn = open(&temp_db("save1")).expect("db opens");
        let mut state = career();
        let id = save(&conn, &state).expect("save");
        state.id = Some(id);

        let loaded = load(&conn, id).expect("load");
        assert_eq!(loaded.name, "Test career");
        assert_eq!(loaded.player_team_name, "Port Vale First XI");
        assert_eq!(load(&conn, id + 999).unwrap_err().to_string().contains("not found"), true);

        let summaries = list(&conn).expect("list");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].team_name, "Port Vale First XI");
    }

    #[test]
    fn save_updates_existing_slot() {
        let conn = open(&temp_db("save2")).expect("db opens");
        let mut state = career();
        let id = save(&conn, &state).expect("save");
        state.id = Some(id);
        state.name = "Renamed career".into();
        state.season_day = 9;
        save(&conn, &state).expect("update");

        let loaded = load(&conn, id).expect("load");
        assert_eq!(loaded.name, "Renamed career");
        assert_eq!(loaded.season_day, 9);
        assert_eq!(list(&conn).expect("list").len(), 1, "no duplicate rows");
        delete(&conn, id).expect("delete");
        assert!(list(&conn).expect("list").is_empty());
        // Deleting a missing save reports not found rather than succeeding.
        assert!(delete(&conn, id)
            .unwrap_err()
            .to_string()
            .contains("not found"));
    }

    #[test]
    fn record_result_news_only_for_player_matches() {
        let mut state = career();
        let result = FixtureResult {
            home_runs: 180,
            away_runs: 150,
            home_wickets: 4,
            away_wickets: 10,
            winner_id: Some(1),
            summary: "Port Vale First XI win by 30 runs".into(),
        };
        let news = record_result(&mut state, 10, result);
        assert_eq!(news.len(), 1);
        assert!(news[0].headline.contains("Win!"));

        // A match not involving the player produces no career news.
        let mut other = career();
        other.fixtures.push(Fixture {
            id: 11,
            competition_id: 1,
            home_id: 3,
            away_id: 4,
            format: Format::T20,
            ground_id: 1,
            day: 1,
            round: 0,
            result: None,
        });
        let quiet = record_result(&mut other, 11, FixtureResult {
            home_runs: 1,
            away_runs: 2,
            home_wickets: 10,
            away_wickets: 10,
            winner_id: Some(4),
            summary: "Away win".into(),
        });
        assert!(quiet.is_empty(), "neutral matches should not spam news");
    }
}
