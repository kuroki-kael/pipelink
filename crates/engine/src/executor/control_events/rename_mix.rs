/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Make-before-break mix renaming across `PipeWire`, `JACK`, `AudioGraph`, and config.
//!
//! Spawns and links the replacement mix loopback and `JACK` output ports with all channel
//! route states and gains copied, switches active output routing to the new mix once
//! ready, restores the original mix column position, and delays unloading the old
//! loopback cleanly.

use pipelink_common::{
    error::ExecutorError,
    events::StateEvent,
    types::{ChannelName, MixName},
};
use pipelink_jack::JackError;
use pipelink_pw::Command as PipewireCommand;
use tracing::{info, warn};

use super::OLD_NODE_RETIRE_DELAY;
use crate::executor::{DeferredMixDestroy, Executor, PendingMixRename};

struct MixSnapshot {
    idx: usize,
    was_active: bool,
    routes: Vec<(ChannelName, bool, f32, bool)>,
}

impl Executor {
    /// Begins a make-before-break rename from `old_name` to `new_name`.
    pub(in crate::executor) fn on_rename_mix(
        &mut self,
        old_name: &MixName,
        new_name: &MixName,
    ) -> Result<(), ExecutorError> {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            let _ = self.state_tx.send(StateEvent::MixOperationFailed {
                mix: old_name.clone(),
                error: "new mix name cannot be empty".to_string(),
            });
            return Ok(());
        }

        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            let _ = self.state_tx.send(StateEvent::MixOperationFailed {
                mix: old_name.clone(),
                error: "mix name may only contain ASCII letters, digits, '-' and '_'".to_string(),
            });
            return Ok(());
        }

        let Some(old_canonical) = self.graph.resolve_mix_name(old_name) else {
            let _ = self.state_tx.send(StateEvent::MixOperationFailed {
                mix: old_name.clone(),
                error: "there is no mix with this name".to_string(),
            });
            return Ok(());
        };

        let new_mix = MixName::from(trimmed);
        if old_canonical == new_mix {
            let _ = self.state_tx.send(StateEvent::MixRenamed {
                old_name: old_canonical,
                new_name: new_mix,
            });
            return Ok(());
        }

        if let Some(error) = self.validate_mix_rename_target(&old_canonical, &new_mix) {
            let _ = self.state_tx.send(StateEvent::MixOperationFailed {
                mix: old_canonical,
                error,
            });
            return Ok(());
        }

        let snapshot = self.capture_mix_snapshot(&old_canonical);
        self.flush_deferred_mix_destroy(&new_mix)?;

        if let Err(err) = self.graph.add_mix(new_mix.clone()) {
            let _ = self.state_tx.send(StateEvent::MixOperationFailed {
                mix: old_canonical,
                error: err.to_string(),
            });
            return Ok(());
        }
        self.apply_snapshot_to_mix(&new_mix, &snapshot);

        if let Err(err) = self.jack.create_mix(&new_mix, self.graph.gains_snapshot()) {
            self.graph.remove_mix(&new_mix);
            if matches!(err, JackError::ChannelClosed) {
                return Err(ExecutorError::Jack(err.to_string()));
            }
            let _ = self.state_tx.send(StateEvent::MixOperationFailed {
                mix: old_canonical,
                error: format!("failed to create JACK ports for renamed mix: {err}"),
            });
            return Ok(());
        }

        if let Err(err) = self.pw_tx.send(PipewireCommand::CreateMix {
            name: new_mix.clone(),
        }) {
            self.graph.remove_mix(&new_mix);
            let _ = self.jack.remove_mix(&new_mix, self.graph.gains_snapshot());
            return Err(ExecutorError::Pipewire(format!("{err:?}")));
        }

        self.pending_mix_renames.insert(
            new_mix,
            PendingMixRename {
                old_name: old_canonical,
                old_idx: snapshot.idx,
                was_active: snapshot.was_active,
            },
        );
        Ok(())
    }

    /// Completes a pending make-before-break mix rename once `new_mix`'s loopback and
    /// stereo `JACK` links are ready.
    pub(in crate::executor) fn finish_pending_mix_rename(&mut self, new_mix: &MixName) -> bool {
        let Some(rename) = self.pending_mix_renames.remove(new_mix) else {
            return false;
        };

        self.config.rename_mix(&rename.old_name, new_mix);
        self.storage.save_config(&self.config);

        if rename.was_active {
            self.set_active_mix(Some(new_mix.clone()));
            self.sync_active_mix_output();
        }

        if let Some(old_mix) = self.graph.remove_mix(&rename.old_name) {
            self.deferred_mix_destroys.push(DeferredMixDestroy {
                mix: old_mix,
                retire_at: tokio::time::Instant::now() + OLD_NODE_RETIRE_DELAY,
            });
        }
        if let Err(err) = self
            .jack
            .remove_mix(&rename.old_name, self.graph.gains_snapshot())
        {
            warn!("Failed to retire old JACK mix '{}': {err}", rename.old_name);
        }

        let _ = self.graph.move_mix(new_mix, rename.old_idx);
        if let Err(err) = self
            .jack
            .move_mix(new_mix, rename.old_idx, self.graph.gains_snapshot())
        {
            warn!("Failed to restore mix order for '{new_mix}': {err}");
        }

        self.storage.mark_state_dirty();
        info!(
            "Renamed mix '{}' -> '{}' (make-before-break)",
            rename.old_name, new_mix
        );
        let _ = self.state_tx.send(StateEvent::MixRenamed {
            old_name: rename.old_name,
            new_name: new_mix.clone(),
        });
        true
    }

    /// Immediately unloads any retiring `PipeWire` loopback matching `name` before
    /// spawning a fresh loopback with that same name.
    pub(in crate::executor) fn flush_deferred_mix_destroy(
        &mut self,
        name: &MixName,
    ) -> Result<(), ExecutorError> {
        self.flush_deferred_mixes_where(|entry| entry.mix.name.eq_ignore_ascii_case(name))
    }

    pub(super) fn flush_deferred_mixes_where(
        &mut self,
        predicate: impl Fn(&DeferredMixDestroy) -> bool,
    ) -> Result<(), ExecutorError> {
        let pending = std::mem::take(&mut self.deferred_mix_destroys);
        for entry in pending {
            if predicate(&entry) {
                self.pw_tx
                    .send(PipewireCommand::DeleteMix { mix: entry.mix })
                    .map_err(|e| ExecutorError::Pipewire(format!("{e:?}")))?;
            } else {
                self.deferred_mix_destroys.push(entry);
            }
        }
        Ok(())
    }

    fn validate_mix_rename_target(
        &self,
        old_canonical: &MixName,
        new_mix: &MixName,
    ) -> Option<String> {
        if self.graph.mix_index(new_mix).is_some() {
            return Some(format!("a mix named '{new_mix}' already exists"));
        }
        if self.pending_mix_renames.contains_key(new_mix)
            || self.pending_mix_renames.values().any(|pending| {
                pending.old_name.eq_ignore_ascii_case(old_canonical)
                    || pending.old_name.eq_ignore_ascii_case(new_mix)
            })
        {
            return Some("another mix rename is already in progress".to_string());
        }
        None
    }

    fn capture_mix_snapshot(&self, old_canonical: &MixName) -> MixSnapshot {
        let idx = self.graph.mix_index(old_canonical).unwrap_or(0);
        let was_active = self.graph.active_mix.as_ref() == Some(old_canonical);
        let routes = self
            .graph
            .channels()
            .iter()
            .filter_map(|ch| {
                self.graph
                    .get_route(&ch.name, old_canonical)
                    .map(|route| (ch.name.clone(), route.active, route.gain, route.muted))
            })
            .collect();

        MixSnapshot {
            idx,
            was_active,
            routes,
        }
    }

    fn apply_snapshot_to_mix(&mut self, mix: &MixName, snapshot: &MixSnapshot) {
        for &(ref channel, active, gain, muted) in &snapshot.routes {
            if let Some(cell) = self.graph.get_route_mut(channel, mix) {
                cell.active = active;
                cell.gain = gain;
                cell.muted = muted;
            }
        }
    }
}
