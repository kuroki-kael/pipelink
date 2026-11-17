/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Make-before-break channel renaming across `PipeWire`, `JACK`, `AudioGraph`, and
//! config.
//!
//! Rename first spawns and links the new `PipeWire` sink and `JACK` ports with all mix
//! gains and master volume copied, migrates active app routes and default-sink status
//! onto the new node once it is ready, and only then retires the old `PipeWire` node
//! after `WirePlumber` has time to switch active streams without interrupting playback.

use pipelink_common::{
    error::ExecutorError,
    events::StateEvent,
    types::{ChannelName, MixName, NodeName},
};
use pipelink_jack::JackError;
use pipelink_pw::Command as PipewireCommand;
use tracing::{info, warn};

use super::OLD_NODE_RETIRE_DELAY;
use crate::executor::{DeferredChannelDestroy, Executor, PendingChannelRename};

struct ChannelSnapshot {
    idx: usize,
    volume_percent: f32,
    muted: bool,
    was_default: bool,
    routes: Vec<(MixName, bool, f32, bool)>,
}

impl Executor {
    /// Begins a make-before-break rename from `old_name` to `new_name`.
    pub(in crate::executor) fn on_rename_channel(
        &mut self,
        old_name: &ChannelName,
        new_name: &ChannelName,
    ) -> Result<(), ExecutorError> {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            let _ = self.state_tx.send(StateEvent::ChannelOperationFailed {
                channel: old_name.clone(),
                error: "new channel name cannot be empty".to_string(),
            });
            return Ok(());
        }

        let Some(old_canonical) = self.graph.resolve_channel_name(old_name) else {
            let _ = self.state_tx.send(StateEvent::ChannelOperationFailed {
                channel: old_name.clone(),
                error: "there is no channel with this name".to_string(),
            });
            return Ok(());
        };

        let new_channel = ChannelName::from(trimmed);
        if old_canonical == new_channel {
            let _ = self.state_tx.send(StateEvent::ChannelRenamed {
                old_name: old_canonical,
                new_name: new_channel,
            });
            return Ok(());
        }

        if let Some(error) = self.validate_rename_target(&old_canonical, &new_channel) {
            let _ = self.state_tx.send(StateEvent::ChannelOperationFailed {
                channel: old_canonical,
                error,
            });
            return Ok(());
        }

        let snapshot = self.capture_channel_snapshot(&old_canonical);
        self.flush_deferred_channel_destroy(&new_channel)?;

        if let Err(err) = self.graph.add_channel(new_channel.clone()) {
            let _ = self.state_tx.send(StateEvent::ChannelOperationFailed {
                channel: old_canonical,
                error: err.to_string(),
            });
            return Ok(());
        }
        self.apply_snapshot_to_channel(&new_channel, &snapshot);

        if let Err(err) = self
            .jack
            .create_channel(&new_channel, self.graph.gains_snapshot())
        {
            self.graph.remove_channel(&new_channel);
            if matches!(err, JackError::ChannelClosed) {
                return Err(ExecutorError::Jack(err.to_string()));
            }
            let _ = self.state_tx.send(StateEvent::ChannelOperationFailed {
                channel: old_canonical,
                error: format!("failed to create JACK ports for renamed channel: {err}"),
            });
            return Ok(());
        }

        if let Err(err) = self.pw_tx.send(PipewireCommand::CreateChannel {
            name: new_channel.clone(),
        }) {
            self.graph.remove_channel(&new_channel);
            let _ = self
                .jack
                .remove_channel(&new_channel, self.graph.gains_snapshot());
            return Err(ExecutorError::Pipewire(format!("{err:?}")));
        }

        self.pending_channel_renames.insert(
            new_channel,
            PendingChannelRename {
                old_name: old_canonical,
                old_idx: snapshot.idx,
                was_default: snapshot.was_default,
            },
        );
        Ok(())
    }

    /// Completes a pending make-before-break rename once `new_channel`'s `PipeWire` node
    /// and stereo `JACK` links are ready to pass audio.
    pub(in crate::executor) fn finish_pending_channel_rename(
        &mut self,
        new_channel: &ChannelName,
    ) -> bool {
        let Some(rename) = self.pending_channel_renames.remove(new_channel) else {
            return false;
        };

        self.config.rename_channel(&rename.old_name, new_channel);
        self.storage.save_config(&self.config);

        for app_node in &mut self.graph.apps {
            if app_node.channel.as_ref() == Some(&rename.old_name) {
                app_node.channel = Some(new_channel.clone());
            }
        }
        self.sync_channel_app_routes(new_channel);

        if rename.was_default
            && let Err(err) = self.pw_tx.send(PipewireCommand::SetDefaultSink {
                node_name: NodeName::from(format!("pipelink.channel.{new_channel}")),
            })
        {
            warn!("Failed to migrate default sink to '{new_channel}': {err:?}");
        }

        if let Some(old_ch) = self.graph.remove_channel(&rename.old_name) {
            self.deferred_channel_destroys.push(DeferredChannelDestroy {
                channel: old_ch,
                retire_at: tokio::time::Instant::now() + OLD_NODE_RETIRE_DELAY,
            });
        }
        if let Err(err) = self
            .jack
            .remove_channel(&rename.old_name, self.graph.gains_snapshot())
        {
            warn!(
                "Failed to retire old JACK channel '{}': {err}",
                rename.old_name
            );
        }

        let _ = self.graph.move_channel(new_channel, rename.old_idx);
        if let Err(err) =
            self.jack
                .move_channel(new_channel, rename.old_idx, self.graph.gains_snapshot())
        {
            warn!("Failed to restore channel order for '{new_channel}': {err}");
        }

        self.storage.mark_state_dirty();
        info!(
            "Renamed channel '{}' -> '{}' (make-before-break)",
            rename.old_name, new_channel
        );
        let _ = self.state_tx.send(StateEvent::ChannelRenamed {
            old_name: rename.old_name,
            new_name: new_channel.clone(),
        });
        true
    }

    /// Immediately destroys any pending retired `PipeWire` node matching `name` before
    /// spawning a fresh sink with that same name.
    pub(in crate::executor) fn flush_deferred_channel_destroy(
        &mut self,
        name: &ChannelName,
    ) -> Result<(), ExecutorError> {
        self.flush_deferred_where(|entry| entry.channel.name.eq_ignore_ascii_case(name))
    }

    pub(super) fn flush_deferred_where(
        &mut self,
        predicate: impl Fn(&DeferredChannelDestroy) -> bool,
    ) -> Result<(), ExecutorError> {
        let pending = std::mem::take(&mut self.deferred_channel_destroys);
        for entry in pending {
            if predicate(&entry) {
                self.pw_tx
                    .send(PipewireCommand::DeleteChannel {
                        channel: entry.channel,
                    })
                    .map_err(|e| ExecutorError::Pipewire(format!("{e:?}")))?;
            } else {
                self.deferred_channel_destroys.push(entry);
            }
        }
        Ok(())
    }

    fn validate_rename_target(
        &self,
        old_canonical: &ChannelName,
        new_channel: &ChannelName,
    ) -> Option<String> {
        if self.graph.channel_index(new_channel).is_some() {
            return Some(format!("a channel named '{new_channel}' already exists"));
        }
        if self.pending_channel_renames.contains_key(new_channel)
            || self.pending_channel_renames.values().any(|pending| {
                pending.old_name.eq_ignore_ascii_case(old_canonical)
                    || pending.old_name.eq_ignore_ascii_case(new_channel)
            })
        {
            return Some("another channel rename is already in progress".to_string());
        }
        None
    }

    fn capture_channel_snapshot(&self, old_canonical: &ChannelName) -> ChannelSnapshot {
        let idx = self.graph.channel_index(old_canonical).unwrap_or(0);
        let (volume_percent, muted) = self
            .graph
            .channels()
            .get(idx)
            .map_or((100.0, false), |ch| (ch.volume_percent, ch.muted));

        let expected_old_node = format!("pipelink.channel.{old_canonical}");
        let was_default = self.graph.default_sink.as_ref().is_some_and(|ds| {
            ds.eq_ignore_ascii_case(&expected_old_node) || ds.eq_ignore_ascii_case(old_canonical)
        });

        let routes = self
            .graph
            .mixes()
            .iter()
            .filter_map(|mix| {
                self.graph
                    .get_route(old_canonical, &mix.name)
                    .map(|route| (mix.name.clone(), route.active, route.gain, route.muted))
            })
            .collect();

        ChannelSnapshot {
            idx,
            volume_percent,
            muted,
            was_default,
            routes,
        }
    }

    fn apply_snapshot_to_channel(&mut self, channel: &ChannelName, snapshot: &ChannelSnapshot) {
        if let Some(ch) = self
            .graph
            .channels_mut()
            .iter_mut()
            .find(|c| &c.name == channel)
        {
            ch.volume_percent = snapshot.volume_percent;
            ch.muted = snapshot.muted;
        }
        for &(ref mix, active, gain, muted) in &snapshot.routes {
            if let Some(cell) = self.graph.get_route_mut(channel, mix) {
                cell.active = active;
                cell.gain = gain;
                cell.muted = muted;
            }
        }
    }
}
