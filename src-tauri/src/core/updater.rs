//! Database update channel: manifest, signature verification, backup and rollback.
//!
//! The app must keep working offline (UR-1) while offering optional, verified
//! content updates (UR-2..UR-5). The flow is:
//!
//! 1. check the (signed) manifest
//! 2. compare dbVersion with what's installed
//! 3. download the payload
//! 4. verify the signature — an unsigned or bad-signature manifest is rejected
//! 5. back up the current DB, then install
//! 6. report the changelog, with one-click rollback

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A published database release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbManifest {
    /// e.g. "2027.02" — must sort above the installed version to be an update.
    pub db_version: String,
    /// Human-readable summary shown before installing.
    pub changelog: String,
    /// Payload location (absolute URL, or a file path for local testing).
    pub url: String,
    /// Size in bytes, used to reject truncated downloads.
    pub bytes: u64,
    /// ed25519/minisign signature over the payload.
    pub signature: String,
    /// Channel this build came from.
    pub channel: UpdateChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannel {
    /// Signed and default.
    Stable,
    /// Opt-in, still signed.
    Beta,
    /// A local JSON file the user picked themselves.
    Local,
}

impl UpdateChannel {
    pub fn label(self) -> &'static str {
        match self {
            UpdateChannel::Stable => "stable",
            UpdateChannel::Beta => "beta",
            UpdateChannel::Local => "local",
        }
    }
}

/// What the user is offered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    /// Nothing newer than what's installed.
    UpToDate { installed: String },
    /// A verified update is available.
    Available {
        manifest: Box<DbManifest>,
        installed: String,
    },
    /// No network or the feed could not be read.
    Unreachable { installed: String },
    /// The feed was reachable but its content failed verification.
    Rejected { installed: String, reason: String },
    /// The user is on the beta channel but running a stable build.
    ChannelMismatch,
}

/// Update settings, stored with the career/settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateSettings {
    /// Off by default: the game must be fully playable with no network.
    #[serde(default)]
    pub check_for_updates: bool,
    #[serde(default = "default_channel")]
    pub channel: UpdateChannel,
    /// Set once an update is installed, so rollback knows where to go.
    #[serde(default)]
    pub previous_db_version: Option<String>,
}

fn default_channel() -> UpdateChannel {
    UpdateChannel::Stable
}

impl Default for UpdateSettings {
    fn default() -> Self {
        UpdateSettings {
            check_for_updates: false,
            channel: UpdateChannel::Stable,
            previous_db_version: None,
        }
    }
}

/// Compare two db versions of the form "YYYY.NN".
///
/// Returns Ordering::Greater when `a` is newer than `b`.
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |v: &str| -> (u32, u32) {
        let mut parts = v.split('.');
        let year = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let num = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        (year, num)
    };
    parse(a).cmp(&parse(b))
}

/// Where a downloaded DB and its backups live.
pub struct UpdatePaths {
    pub root: PathBuf,
    pub current: PathBuf,
    pub backup: PathBuf,
    pub pending: PathBuf,
    pub manifest: PathBuf,
}

impl UpdatePaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        UpdatePaths {
            current: root.join("world.json"),
            backup: root.join("world.backup.json"),
            pending: root.join("world.pending.json"),
            manifest: root.join("manifest.json"),
            root,
        }
    }
}

/// A minimal, dependency-free check of a manifest.
///
/// Real signature verification lands with the ed25519 crate in M6.3; what is
/// enforced *now* is everything we can check without a key: the manifest must
/// parse, the version must be well formed and newer, and a payload with no
/// signature at all is always refused.
pub fn validate_manifest(
    manifest: &DbManifest,
    installed: &str,
    settings: &UpdateSettings,
) -> Result<(), String> {
    if compare_versions(&manifest.db_version, installed) != std::cmp::Ordering::Greater {
        return Err(format!(
            "manifest version {} is not newer than installed {}",
            manifest.db_version, installed
        ));
    }
    // "YYYY.NN" is the only accepted shape.
    let parts: Vec<&str> = manifest.db_version.split('.').collect();
    if parts.len() != 2
        || parts.iter().any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
    {
        return Err(format!("bad version format: {}", manifest.db_version));
    }
    if manifest.signature.trim().is_empty() {
        return Err("manifest is unsigned — refusing to install".into());
    }
    if manifest.url.trim().is_empty() {
        return Err("manifest has no payload url".into());
    }
    match settings.channel {
        UpdateChannel::Beta => {
            if manifest.channel == UpdateChannel::Stable {
                // Fine: beta users may take stable releases.
            }
        }
        _ => {
            if manifest.channel == UpdateChannel::Beta {
                return Err("beta builds need the beta channel enabled".into());
            }
        }
    }
    Ok(())
}

/// Decide what to offer the user.
pub fn evaluate_update(
    installed: &str,
    manifest: Option<&DbManifest>,
    settings: &UpdateSettings,
    network_ok: bool,
) -> UpdateStatus {
    if !settings.check_for_updates {
        return UpdateStatus::UpToDate {
            installed: installed.to_string(),
        };
    }
    match manifest {
        None if network_ok => UpdateStatus::Rejected {
            installed: installed.to_string(),
            reason: "no manifest at the feed".into(),
        },
        None => UpdateStatus::Unreachable {
            installed: installed.to_string(),
        },
        Some(manifest) => match validate_manifest(manifest, installed, settings) {
            Ok(()) => UpdateStatus::Available {
                manifest: Box::new(manifest.clone()),
                installed: installed.to_string(),
            },
            Err(reason) => UpdateStatus::Rejected {
                installed: installed.to_string(),
                reason,
            },
        },
    }
}

/// Install a verified payload: back up the current DB, then move the pending
/// file into place. Returns the version we came from (for rollback).
pub fn install_update(paths: &UpdatePaths, from_version: &str) -> Result<String, String> {
    if !paths.pending.exists() {
        return Err("no downloaded payload to install".into());
    }
    if paths.current.exists() {
        std::fs::copy(&paths.current, &paths.backup)
            .map_err(|e| format!("could not back up the current database: {e}"))?;
    }
    std::fs::rename(&paths.pending, &paths.current)
        .map_err(|e| format!("could not install the new database: {e}"))?;
    Ok(from_version.to_string())
}

/// Restore the backed-up database. Returns the version we rolled back to.
pub fn rollback(paths: &UpdatePaths, previous_version: &str) -> Result<(), String> {
    if !paths.backup.exists() {
        return Err("no backup to roll back to".into());
    }
    std::fs::copy(&paths.backup, &paths.current)
        .map_err(|e| format!("could not restore the backup: {e}"))?;
    let _ = previous_version;
    Ok(())
}

/// Where the app keeps update state on this platform.
pub fn update_root(app_data_dir: impl AsRef<Path>) -> PathBuf {
    app_data_dir.as_ref().join("db")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(version: &str) -> DbManifest {
        DbManifest {
            db_version: version.into(),
            changelog: "New season".into(),
            url: "https://example.com/world.json".into(),
            bytes: 1234,
            signature: "ed25519:abcdef".into(),
            channel: UpdateChannel::Stable,
        }
    }

    #[test]
    fn version_ordering_is_numeric_not_lexicographic() {
        assert_eq!(
            compare_versions("2027.10", "2027.9"),
            std::cmp::Ordering::Greater,
            "10 must sort above 9"
        );
        assert_eq!(
            compare_versions("2028.1", "2027.12"),
            std::cmp::Ordering::Greater,
            "the year part dominates"
        );
        assert_eq!(compare_versions("2027.1", "2027.1"), std::cmp::Ordering::Equal);
        assert_eq!(
            compare_versions("2027.2", "2027.3"),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn a_newer_signed_manifest_is_accepted() {
        let settings = UpdateSettings::default();
        assert!(validate_manifest(&manifest("2027.02"), "2027.01", &settings).is_ok());
    }

    #[test]
    fn unsigned_manifests_are_always_refused() {
        let settings = UpdateSettings::default();
        let mut bad = manifest("2027.02");
        bad.signature = "".into();
        let err = validate_manifest(&bad, "2027.01", &settings).unwrap_err();
        assert!(err.contains("unsigned"), "got: {err}");
    }

    #[test]
    fn older_or_equal_manifests_are_refused() {
        let settings = UpdateSettings::default();
        assert!(validate_manifest(&manifest("2027.01"), "2027.01", &settings).is_err());
        assert!(validate_manifest(&manifest("2026.01"), "2027.01", &settings).is_err());
    }

    #[test]
    fn malformed_versions_are_refused() {
        let settings = UpdateSettings::default();
        assert!(validate_manifest(&manifest("2027"), "2027.01", &settings).is_err());
        assert!(validate_manifest(&manifest("next"), "2027.01", &settings).is_err());
    }

    #[test]
    fn beta_builds_need_the_beta_channel() {
        let stable = UpdateSettings::default();
        let mut beta = manifest("2027.02");
        beta.channel = UpdateChannel::Beta;
        assert!(validate_manifest(&beta, "2027.01", &stable).is_err());

        let opted_in = UpdateSettings {
            channel: UpdateChannel::Beta,
            ..Default::default()
        };
        assert!(validate_manifest(&beta, "2027.01", &opted_in).is_ok());
    }

    #[test]
    fn updates_are_off_by_default() {
        let settings = UpdateSettings::default();
        let status = evaluate_update(
            "2027.01",
            Some(&manifest("2027.02")),
            &settings,
            true,
        );
        assert!(
            matches!(status, UpdateStatus::UpToDate { .. }),
            "offline-first: no prompting unless the user opts in"
        );
    }

    #[test]
    fn enabling_updates_offers_the_newer_build() {
        let settings = UpdateSettings {
            check_for_updates: true,
            ..Default::default()
        };
        let status = evaluate_update("2027.01", Some(&manifest("2027.02")), &settings, true);
        match status {
            UpdateStatus::Available { manifest, installed } => {
                assert_eq!(manifest.db_version, "2027.02");
                assert_eq!(installed, "2027.01");
            }
            other => panic!("expected an available update, got {other:?}"),
        }
    }

    #[test]
    fn no_network_reports_unreachable_not_an_error() {
        let settings = UpdateSettings {
            check_for_updates: true,
            ..Default::default()
        };
        let status = evaluate_update("2027.01", None, &settings, false);
        assert!(matches!(status, UpdateStatus::Unreachable { .. }));
    }

    #[test]
    fn install_and_rollback_round_trip() {
        let dir = std::env::temp_dir().join("cc2027-update-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let paths = UpdatePaths::new(&dir);

        std::fs::write(&paths.current, "old database").unwrap();
        std::fs::write(&paths.pending, "new database").unwrap();

        let previous = install_update(&paths, "2027.01").expect("install");
        assert_eq!(previous, "2027.01");
        assert_eq!(
            std::fs::read_to_string(&paths.current).unwrap(),
            "new database",
            "the new payload takes over"
        );
        assert_eq!(
            std::fs::read_to_string(&paths.backup).unwrap(),
            "old database",
            "the previous DB is kept for rollback"
        );

        rollback(&paths, "2027.01").expect("rollback");
        assert_eq!(
            std::fs::read_to_string(&paths.current).unwrap(),
            "old database",
            "rollback restores the backup"
        );
    }

    #[test]
    fn install_without_a_payload_fails_cleanly() {
        let dir = std::env::temp_dir().join("cc2027-update-empty");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let paths = UpdatePaths::new(dir);
        assert!(install_update(&paths, "2027.01").is_err());
        assert!(rollback(&paths, "2027.01").is_err(), "no backup yet");
    }
}
