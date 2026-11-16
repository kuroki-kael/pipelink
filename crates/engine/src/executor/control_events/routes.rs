/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Control handlers for channel-to-mix route activation, gain, and mute controls.

use pipelink_common::{
    error::ExecutorError,
    events::StateEvent,
    graph::audio_graph::ChannelMix,
    types::{ChannelName, MixName},
};
use pipelink_jack::JackError;
use tracing::warn;

use crate::{
    config::state::{normalize_percent, percent_to_linear},
    executor::Executor,
};

impl Executor {
    /// Activates the route from `channel` into `mix` and updates the JACK mixer.
    pub(in crate::executor) fn on_add_channel_mix(
        &mut self,
        channel: ChannelName,
        mix: MixName,
    ) -> Result<(), ExecutorError> {
        let (channel, mix) = self.resolve_route_names(channel, mix);
        if self
            .update_route_and_sync(&channel, &mix, |route| route.active = true)?
            .is_some()
        {
            self.config.set_channel_mix(&channel, &mix);
            self.storage.save_config(&self.config);
            let _ = self
                .state_tx
                .send(StateEvent::MixChannelAdded { channel, mix });
        }
        Ok(())
    }

    /// Deactivates the route from `channel` into `mix` and silences it in JACK.
    pub(in crate::executor) fn on_remove_channel_mix(
        &mut self,
        channel: ChannelName,
        mix: MixName,
    ) -> Result<(), ExecutorError> {
        let (channel, mix) = self.resolve_route_names(channel, mix);
        if self
            .update_route_and_sync(&channel, &mix, |route| route.active = false)?
            .is_some()
        {
            self.config.remove_channel_mix(&channel, &mix);
            self.storage.save_config(&self.config);
            let _ = self
                .state_tx
                .send(StateEvent::MixChannelRemoved { channel, mix });
        }
        Ok(())
    }

    /// Updates the gain (in percent) of `(channel, mix)` and pushes the linear gain to
    /// JACK.
    pub(in crate::executor) fn on_set_channel_gain(
        &mut self,
        channel: ChannelName,
        mix: MixName,
        percent: f32,
    ) -> Result<(), ExecutorError> {
        let (channel, mix) = self.resolve_route_names(channel, mix);
        let percent = normalize_percent(percent);
        let gain_linear = percent_to_linear(percent);

        if self
            .update_route_and_sync(&channel, &mix, |route| route.gain = gain_linear)?
            .is_some()
        {
            self.storage.mark_state_dirty();
            let _ = self.state_tx.send(StateEvent::GainChanged {
                channel,
                mix,
                gain_linear,
                percent,
            });
        }
        Ok(())
    }

    /// Updates the mute flag of `(channel, mix)` and pushes the effective gain to JACK.
    pub(in crate::executor) fn on_set_channel_mute(
        &mut self,
        channel: ChannelName,
        mix: MixName,
        muted: bool,
    ) -> Result<(), ExecutorError> {
        let (channel, mix) = self.resolve_route_names(channel, mix);
        if self
            .update_route_and_sync(&channel, &mix, |route| route.muted = muted)?
            .is_some()
        {
            self.storage.mark_state_dirty();
            let _ = self.state_tx.send(StateEvent::MuteChanged {
                channel,
                mix,
                muted,
            });
        }
        Ok(())
    }

    fn resolve_route_names(&self, channel: ChannelName, mix: MixName) -> (ChannelName, MixName) {
        let channel = self.graph.resolve_channel_name(&channel).unwrap_or(channel);
        let mix = self.graph.resolve_mix_name(&mix).unwrap_or(mix);
        (channel, mix)
    }

    /// Computes the updated route, pushes its effective linear gain to JACK,
    /// and commits the mutation in `AudioGraph` only once JACK accepts it.
    fn update_route_and_sync<F>(
        &mut self,
        channel: &ChannelName,
        mix: &MixName,
        update: F,
    ) -> Result<Option<ChannelMix>, ExecutorError>
    where
        F: FnOnce(&mut ChannelMix),
    {
        let Some(mut next) = self.graph.get_route(channel, mix).cloned() else {
            warn!("Cannot update route ({channel} -> {mix}): channel or mix not found");
            let _ = self.state_tx.send(StateEvent::RouteOperationFailed {
                channel: channel.clone(),
                mix: mix.clone(),
                error: "channel or mix not found".to_string(),
            });
            return Ok(None);
        };

        update(&mut next);
        let gains = self
            .graph
            .gains_with_override(channel, mix, next.effective_gain());

        match self.jack.sync_gains(gains) {
            Ok(()) => {
                if let Some(slot) = self.graph.get_route_mut(channel, mix) {
                    *slot = next.clone();
                }
                Ok(Some(next))
            },
            Err(JackError::ChannelClosed) => {
                Err(ExecutorError::Jack(JackError::ChannelClosed.to_string()))
            },
            Err(err) => {
                warn!("Failed to update JACK route ({channel} -> {mix}): {err}");
                let _ = self.state_tx.send(StateEvent::RouteOperationFailed {
                    channel: channel.clone(),
                    mix: mix.clone(),
                    error: err.to_string(),
                });
                Ok(None)
            },
        }
    }
}
