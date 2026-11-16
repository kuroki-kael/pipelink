/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! XDG-compliant loading and atomic persistence for [`ConfigGraph`] and [`RuntimeState`].

mod io_helpers;
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;

use std::{fs, io, path::PathBuf, time::Duration};

use io_helpers::{load_json_or_default, resolve_file_path, write_json_atomic};
use pipelink_common::graph::audio_graph::AudioGraph;
use tokio::time::Instant;
use tracing::{debug, warn};

use super::{state::RuntimeState, topology::ConfigGraph};

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

    /// Loads [`ConfigGraph`] from disk, or initializes and writes
    /// [`ConfigGraph::starter_default`] when no `config.json` file exists yet.
    #[must_use]
    pub fn load_or_init_config(&self) -> ConfigGraph {
        match fs::symlink_metadata(&self.config_file) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                let starter = ConfigGraph::starter_default();
                if let Err(write_err) = self.save_config(&starter) {
                    warn!(
                        "Failed to write starter config to '{}': {write_err}",
                        self.config_file.display()
                    );
                }
                starter
            },
            Ok(_) | Err(_) => self.load_config(),
        }
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
