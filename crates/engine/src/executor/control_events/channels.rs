/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Channel null-sink master gain/mute and reordering handlers.

use pipelink_common::{error::ExecutorError, events::StateEvent, types::ChannelName};
use pipelink_jack::JackError;
use pipelink_pw::Command as PipewireCommand;
use tracing::warn;

use crate::{
    config::state::{normalize_channel_percent, pw_linear_to_percent, pw_percent_to_linear},
    executor::Executor,
};

impl Executor {
    /// Sets master gain percentage on a channel's `PipeWire` null-sink (`0.0..=200.0`).
    pub(in crate::executor) fn on_set_channel_master_volume(
        &mut self,
        channel: &ChannelName,
        percent: f32,
    ) {
        let clamped = normalize_channel_percent(percent);
        self.apply_channel_master_update(channel, Some(clamped), None);
    }

    /// Sets master mute state on a channel's `PipeWire` null-sink.
    pub(in crate::executor) fn on_set_channel_master_mute(
        &mut self,
        channel: &ChannelName,
        muted: bool,
    ) {
        self.apply_channel_master_update(channel, None, Some(muted));
    }

    fn apply_channel_master_update(
        &mut self,
        channel: &ChannelName,
        next_percent: Option<f32>,
        next_muted: Option<bool>,
    ) {
        let Some(ch) = self
            .graph
            .channels_mut()
            .iter_mut()
            .find(|c| c.name.eq_ignore_ascii_case(channel))
        else {
            let _ = self.state_tx.send(StateEvent::ChannelOperationFailed {
                channel: channel.clone(),
                error: "there is no channel with this name".to_string(),
            });
            return;
        };

        if let Some(p) = next_percent {
            ch.volume_percent = p;
        }
        if let Some(m) = next_muted {
            ch.muted = m;
        }

        let canonical = ch.name.clone();
        let percent = ch.volume_percent;
        let muted = ch.muted;
        let node_id = ch.node.id;

        self.storage.mark_state_dirty();

        if let Some(id) = node_id
            && let Err(err) = self.pw_tx.send(PipewireCommand::SetChannelNodeProps {
                node_id: id,
                linear_volume: pw_percent_to_linear(percent),
                muted,
            })
        {
            warn!("Failed to send SetChannelNodeProps for '{canonical}': {err:?}");
        }

        let _ = self.state_tx.send(StateEvent::ChannelMasterChanged {
            channel: canonical,
            percent,
            muted,
        });
    }

    /// Applies an external `Props` change reported by `PipeWire` for a channel null-sink.
    pub(in crate::executor) fn on_channel_props_changed(
        &mut self,
        node_id: pipelink_common::types::NodeId,
        linear_volume: Option<f32>,
        muted: Option<bool>,
    ) {
        let Some(ch) = self
            .graph
            .channels_mut()
            .iter_mut()
            .find(|c| c.node.id == Some(node_id))
        else {
            return;
        };

        let next_percent = linear_volume.map_or(ch.volume_percent, pw_linear_to_percent);
        let next_muted = muted.unwrap_or(ch.muted);
        if (next_percent - ch.volume_percent).abs() <= 0.4 && next_muted == ch.muted {
            return;
        }

        ch.volume_percent = next_percent;
        ch.muted = next_muted;
        let canonical = ch.name.clone();
        self.storage.mark_state_dirty();

        let _ = self.state_tx.send(StateEvent::ChannelMasterChanged {
            channel: canonical,
            percent: next_percent,
            muted: next_muted,
        });
    }

    /// Reorders `channel` cleanly without unregistering JACK or `PipeWire` nodes.
    pub(in crate::executor) fn on_move_channel(
        &mut self,
        channel: &ChannelName,
        to_index: usize,
    ) -> Result<(), ExecutorError> {
        let Some(canonical) = self.graph.resolve_channel_name(channel) else {
            let _ = self.state_tx.send(StateEvent::ChannelOperationFailed {
                channel: channel.clone(),
                error: "there is no channel with this name".to_string(),
            });
            return Ok(());
        };

        let Some((from_idx, actual_to)) = self.graph.move_channel(&canonical, to_index) else {
            return Ok(());
        };
        if from_idx != actual_to {
            if let Err(err) =
                self.jack
                    .move_channel(&canonical, actual_to, self.graph.gains_snapshot())
            {
                self.graph.move_channel(&canonical, from_idx);
                if matches!(err, JackError::ChannelClosed) {
                    return Err(ExecutorError::Jack(err.to_string()));
                }
                let _ = self.state_tx.send(StateEvent::ChannelOperationFailed {
                    channel: canonical,
                    error: err.to_string(),
                });
                return Ok(());
            }

            let config_to = self.graph.channels()[..actual_to]
                .iter()
                .filter(|c| {
                    self.config
                        .channels
                        .iter()
                        .any(|cfg_ch| cfg_ch.name.eq_ignore_ascii_case(&c.name))
                })
                .count();
            if self.config.move_channel(&canonical, config_to) {
                self.storage.save_config(&self.config);
            }
            self.storage.mark_state_dirty();
        }

        let ordered = self
            .graph
            .channels()
            .iter()
            .map(|c| c.name.clone())
            .collect();
        let _ = self
            .state_tx
            .send(StateEvent::ChannelsReordered { channels: ordered });
        Ok(())
    }
}
