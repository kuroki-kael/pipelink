/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! XDG-compliant loading and atomic persistence for [`ConfigGraph`] and [`RuntimeState`].

use std::{
    env,
    ffi::OsStr,
    fs::{self, DirBuilder, OpenOptions},
    io::{self, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process,
    time::Duration,
};

use pipelink_common::graph::audio_graph::AudioGraph;
use serde::{Serialize, de::DeserializeOwned};
use tokio::time::Instant;
use tracing::{debug, warn};

use super::{state::RuntimeState, topology::ConfigGraph};

const APP_DIR: &str = "pipelink";
const CONFIG_FILE_NAME: &str = "config.json";
const STATE_FILE_NAME: &str = "state.json";

/// Debounce delay before flushing volatile mixer changes to disk.
pub(crate) const STATE_DEBOUNCE_DELAY: Duration = Duration::from_millis(1500);

/// Resolved filesystem locations for persistent config and runtime state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoragePaths {
    /// Path to `config.json`.
    pub config_file: PathBuf,
    /// Path to `state.json`.
    pub state_file: PathBuf,
}

impl StoragePaths {
    /// Resolves XDG-compliant paths with optional `PIPELINK_CONFIG` / `PIPELINK_STATE`
    /// overrides.
    #[must_use]
    pub fn discover() -> Option<Self> {
        let config_file = resolve_file_path(
            "PIPELINK_CONFIG",
            "XDG_CONFIG_HOME",
            ".config",
            CONFIG_FILE_NAME,
        )?;
        let state_file = resolve_file_path(
            "PIPELINK_STATE",
            "XDG_STATE_HOME",
            ".local/state",
            STATE_FILE_NAME,
        )?;
        Some(Self {
            config_file,
            state_file,
        })
    }

    /// Loads [`ConfigGraph`] from disk, returning [`ConfigGraph::default`] when missing
    /// or recovering after backing up a malformed file.
    #[must_use]
    pub fn load_config(&self) -> ConfigGraph {
        load_json_or_default(&self.config_file)
    }

    /// Loads [`RuntimeState`] from disk, returning [`RuntimeState::default`] when missing
    /// or recovering after backing up a malformed file.
    #[must_use]
    pub fn load_state(&self) -> RuntimeState {
        load_json_or_default(&self.state_file)
    }

    /// Atomically persists [`ConfigGraph`] to `self.config_file`.
    ///
    /// # Errors
    /// Returns an [`io::Error`] if writing or renaming the file fails.
    pub fn save_config(&self, config: &ConfigGraph) -> io::Result<()> {
        write_json_atomic(&self.config_file, config)
    }

    /// Atomically persists [`RuntimeState`] to `self.state_file`.
    ///
    /// # Errors
    /// Returns an [`io::Error`] if writing or renaming the file fails.
    pub fn save_state(&self, state: &RuntimeState) -> io::Result<()> {
        write_json_atomic(&self.state_file, state)
    }
}

/// Manages immediate config writes and debounced runtime-state writes.
#[derive(Debug)]
pub(crate) struct PersistenceStore {
    paths: Option<StoragePaths>,
    state_dirty: bool,
    state_deadline: Option<Instant>,
    debounce: Duration,
}

impl Default for PersistenceStore {
    fn default() -> Self {
        Self::new(None)
    }
}

impl PersistenceStore {
    pub(crate) const fn new(paths: Option<StoragePaths>) -> Self {
        Self {
            paths,
            state_dirty: false,
            state_deadline: None,
            debounce: STATE_DEBOUNCE_DELAY,
        }
    }

    /// Persists structural topology changes immediately if storage is enabled.
    pub(crate) fn save_config(&self, config: &ConfigGraph) {
        let Some(paths) = &self.paths else {
            return;
        };
        if let Err(err) = paths.save_config(config) {
            warn!(
                "Failed to save config to '{}': {err}",
                paths.config_file.display()
            );
        } else {
            debug!("Saved config to '{}'", paths.config_file.display());
        }
    }

    /// Marks runtime state as dirty and resets the debounce flush deadline.
    pub(crate) fn mark_state_dirty(&mut self) {
        if self.paths.is_none() {
            return;
        }
        self.state_dirty = true;
        self.state_deadline = Some(Instant::now() + self.debounce);
    }

    /// Returns the next scheduled flush timestamp when dirty state is pending.
    pub(crate) const fn state_flush_deadline(&self) -> Option<Instant> {
        if self.state_dirty {
            self.state_deadline
        } else {
            None
        }
    }

    /// Flushes runtime state immediately when dirty, clearing the pending deadline.
    pub(crate) fn flush_state_if_dirty(&mut self, graph: &AudioGraph) {
        if !self.state_dirty {
            return;
        }
        self.state_dirty = false;
        self.state_deadline = None;

        let Some(paths) = &self.paths else {
            return;
        };
        let state = RuntimeState::from_graph(graph);
        if let Err(err) = paths.save_state(&state) {
            warn!(
                "Failed to save runtime state to '{}': {err}",
                paths.state_file.display()
            );
        } else {
            debug!("Saved runtime state to '{}'", paths.state_file.display());
        }
    }
}

fn resolve_file_path(
    override_env: &str,
    xdg_env: &str,
    home_fallback_rel: &str,
    file_name: &str,
) -> Option<PathBuf> {
    if let Some(path) = env::var_os(override_env).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(path));
    }
    let base_dir = env::var_os(xdg_env)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .filter(|v| !v.is_empty())
                .map(|home| PathBuf::from(home).join(home_fallback_rel))
        })?;
    Some(base_dir.join(APP_DIR).join(file_name))
}

fn load_json_or_default<T: DeserializeOwned + Default>(path: &Path) -> T {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return T::default(),
        Err(err) => {
            warn!("Failed to read '{}': {err}; using defaults", path.display());
            return T::default();
        },
    };

    match serde_json::from_slice::<T>(&bytes) {
        Ok(parsed) => parsed,
        Err(err) => {
            let backup = backup_path(path);
            if let Err(rename_err) = fs::rename(path, &backup) {
                warn!(
                    "Malformed JSON in '{}' ({err}); also failed to move backup '{}': {rename_err}",
                    path.display(),
                    backup.display()
                );
            } else {
                warn!(
                    "Malformed JSON in '{}' ({err}); moved corrupt file to '{}' and reset to defaults",
                    path.display(),
                    backup.display()
                );
            }
            T::default()
        },
    }
}

fn backup_path(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(OsStr::new(".bak"));
    PathBuf::from(os)
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
    }

    let payload = serde_json::to_vec_pretty(value)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    let tmp = temp_path_for(path);
    let write_res = (|| -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        file.write_all(&payload)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
        fs::rename(&tmp, path)?;
        Ok(())
    })();

    if write_res.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    write_res
}

fn temp_path_for(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("pipelink");
    let tmp_name = format!(".{file_name}.tmp.{}", process::id());
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(tmp_name),
        _ => PathBuf::from(tmp_name),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use pipelink_common::types::{AppDisplay, ChannelName, MixName, NodeName};

    use super::*;
    use crate::config::state::RouteState;

    fn unique_test_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("pipelink-{tag}-{}-{nanos}", process::id()))
    }

    #[test]
    fn test_storage_round_trip_and_corrupt_backup() {
        let root = unique_test_dir("storage-test");
        let paths = StoragePaths {
            config_file: root.join("config/config.json"),
            state_file: root.join("state/state.json"),
        };

        assert_eq!(paths.load_config(), ConfigGraph::default());
        assert_eq!(paths.load_state(), RuntimeState::default());

        let mut cfg = ConfigGraph::new();
        let music = ChannelName::from("Music");
        let personal = MixName::from("Personal");
        cfg.ensure_channel(&music);
        cfg.ensure_mix(&personal);
        cfg.set_channel_mix(&music, &personal);
        cfg.set_app_route(AppDisplay::from("Spotify"), &music);
        cfg.output = Some(NodeName::from("alsa_output.usb.test"));

        paths.save_config(&cfg).unwrap();
        assert_eq!(paths.load_config(), cfg);

        let state = RuntimeState {
            active_mix: Some(personal.clone()),
            routes: vec![RouteState {
                channel: music,
                mix: personal,
                gain_percent: 75.0,
                muted: true,
            }],
        };
        paths.save_state(&state).unwrap();
        assert_eq!(paths.load_state(), state);

        // Corrupt JSON resets cleanly and creates a `.bak` backup.
        fs::write(&paths.config_file, b"{not valid json").unwrap();
        assert_eq!(paths.load_config(), ConfigGraph::default());
        assert!(backup_path(&paths.config_file).exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_persistence_store_debounce_and_flush() {
        let root = unique_test_dir("store-flush");
        let paths = StoragePaths {
            config_file: root.join("config.json"),
            state_file: root.join("state.json"),
        };

        let mut store = PersistenceStore::new(Some(paths.clone()));
        assert!(store.state_flush_deadline().is_none());

        store.mark_state_dirty();
        assert!(store.state_flush_deadline().is_some());

        let mut graph = AudioGraph::new();
        graph.add_mix(MixName::from("Stream")).unwrap();
        graph.active_mix = Some(MixName::from("Stream"));

        store.flush_state_if_dirty(&graph);
        assert!(store.state_flush_deadline().is_none());
        assert_eq!(paths.load_state().active_mix, Some(MixName::from("Stream")));

        let _ = fs::remove_dir_all(&root);
    }
}
