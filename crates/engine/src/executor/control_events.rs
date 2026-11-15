/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Handling of the control events that steer the audio graph.

use pipelink_common::{
    error::ExecutorError,
    events::StateEvent,
    graph::audio_graph::ChannelMix,
    types::{ChannelName, MixName},
};
use pipelink_jack::JackError;
use pipelink_pw::Command as PipewireCommand;
use tracing::warn;

use crate::executor::Executor;

impl Executor {
    /// Creates a channel together with its `JACK` input ports and `PipeWire` sink.
    ///
    /// The channel enters the graph optimistically, before `PipeWire` confirms
    /// it, and is taken out again on `ChannelCreationFailed`.
    pub(super) fn on_create_channel(&mut self, name: ChannelName) -> Result<(), ExecutorError> {
        if let Err(err) = self.graph.add_channel(name.clone()) {
            warn!("{err}");
            let _ = self.state_tx.send(StateEvent::ChannelCreationFailed {
                channel: name,
                error: err.to_string(),
            });
            return Ok(());
        }

        if let Err(err) = self.jack.create_channel(&name, self.graph.gains_snapshot()) {
            self.graph.remove_channel(&name);
            if matches!(err, JackError::ChannelClosed) {
                return Err(ExecutorError::Jack(err.to_string()));
            }
            warn!("Failed to create JACK channel '{name}': {err}");
            let _ = self.state_tx.send(StateEvent::ChannelCreationFailed {
                channel: name,
                error: err.to_string(),
            });
            return Ok(());
        }

        if let Err(err) = self
            .pw_tx
            .send(PipewireCommand::CreateChannel { name: name.clone() })
        {
            self.graph.remove_channel(&name);
            let _ = self.jack.remove_channel(&name, self.graph.gains_snapshot());
            return Err(ExecutorError::Pipewire(format!("{err:?}")));
        }

        self.config.ensure_channel(&name);

        Ok(())
    }

    /// Removes a channel and destroys the `PipeWire` sink backing it.
    ///
    /// The entry stays in the graph until `PipeWire` reports the node gone,
    /// because only that report confirms the removal. A channel that does
    /// not exist is reported as a failed removal, otherwise the client would
    /// wait for an event that never arrives.
    pub(super) fn on_remove_channel(&mut self, name: ChannelName) -> Result<(), ExecutorError> {
        let channels = self.graph.channels();

        let Some(channel) = channels.iter().find(|c| c.name == name).cloned() else {
            let _ = self.state_tx.send(StateEvent::ChannelRemovalFailed {
                channel: name,
                error: "there is no channel with this name".to_string(),
            });
            return Ok(());
        };

        if channel.node.id.is_none() {
            self.graph.remove_channel(&name);
            self.clear_channel_app_routes(&name);
            if let Err(err) = self.jack.remove_channel(&name, self.graph.gains_snapshot()) {
                warn!("Failed to remove JACK channel '{name}': {err}");
            }
            let _ = self
                .state_tx
                .send(StateEvent::ChannelRemoved { channel: name });
            return Ok(());
        }

        self.pw_tx
            .send(PipewireCommand::DeleteChannel { channel })
            .map_err(|e| ExecutorError::Pipewire(format!("{e:?}")))?;

        Ok(())
    }

    /// Creates a mix together with its `JACK` output ports and `PipeWire` loopback.
    pub(super) fn on_create_mix(&mut self, name: MixName) -> Result<(), ExecutorError> {
        if let Err(err) = self.graph.add_mix(name.clone()) {
            warn!("{err}");
            let _ = self.state_tx.send(StateEvent::MixCreationFailed {
                mix: name,
                error: err.to_string(),
            });
            return Ok(());
        }

        if let Err(err) = self.jack.create_mix(&name, self.graph.gains_snapshot()) {
            self.graph.remove_mix(&name);
            if matches!(err, JackError::ChannelClosed) {
                return Err(ExecutorError::Jack(err.to_string()));
            }
            warn!("Failed to create JACK mix '{name}': {err}");
            let _ = self.state_tx.send(StateEvent::MixCreationFailed {
                mix: name,
                error: err.to_string(),
            });
            return Ok(());
        }

        if let Err(err) = self
            .pw_tx
            .send(PipewireCommand::CreateMix { name: name.clone() })
        {
            self.graph.remove_mix(&name);
            let _ = self.jack.remove_mix(&name, self.graph.gains_snapshot());
            return Err(ExecutorError::Pipewire(format!("{err:?}")));
        }

        Ok(())
    }

    /// Removes a mix and destroys the `PipeWire` loopback backing it.
    ///
    /// The entry stays in the graph until `PipeWire` reports the nodes gone,
    /// because only that report confirms the removal. A mix that does not
    /// exist is reported as a failed removal, otherwise the client would
    /// wait for an event that never arrives.
    pub(super) fn on_remove_mix(&mut self, name: MixName) -> Result<(), ExecutorError> {
        let mixes = self.graph.mixes();

        let Some(mix) = mixes.iter().find(|m| m.name == name).cloned() else {
            let _ = self.state_tx.send(StateEvent::MixRemovalFailed {
                mix: name,
                error: "there is no mix with this name".to_string(),
            });
            return Ok(());
        };

        if mix.sink.id.is_none() && mix.source.id.is_none() {
            // The loopback was never created, so there is nothing to destroy.
            self.graph.remove_mix(&name);
            if let Err(err) = self.jack.remove_mix(&name, self.graph.gains_snapshot()) {
                warn!("Failed to remove JACK mix '{name}': {err}");
            }
            let _ = self.state_tx.send(StateEvent::MixRemoved { mix: name });
            return Ok(());
        }

        self.pw_tx
            .send(PipewireCommand::DeleteMix { mix })
            .map_err(|e| ExecutorError::Pipewire(format!("{e:?}")))?;

        Ok(())
    }

    /// Activates the route from `channel` into `mix` and updates the JACK mixer.
    pub(super) fn on_add_channel_mix(
        &mut self,
        channel: ChannelName,
        mix: MixName,
    ) -> Result<(), ExecutorError> {
        if self
            .update_route_and_sync(&channel, &mix, |route| route.active = true)?
            .is_some()
        {
            let _ = self
                .state_tx
                .send(StateEvent::MixChannelAdded { channel, mix });
        }
        Ok(())
    }

    /// Deactivates the route from `channel` into `mix` and silences it in JACK.
    pub(super) fn on_remove_channel_mix(
        &mut self,
        channel: ChannelName,
        mix: MixName,
    ) -> Result<(), ExecutorError> {
        if self
            .update_route_and_sync(&channel, &mix, |route| route.active = false)?
            .is_some()
        {
            let _ = self
                .state_tx
                .send(StateEvent::MixChannelRemoved { channel, mix });
        }
        Ok(())
    }

    /// Updates the gain (in percent) of `(channel, mix)` and pushes the linear gain to
    /// JACK.
    pub(super) fn on_set_channel_gain(
        &mut self,
        channel: ChannelName,
        mix: MixName,
        percent: f32,
    ) -> Result<(), ExecutorError> {
        let percent = normalize_percent(percent);
        let gain_linear = percent_to_linear(percent);

        if self
            .update_route_and_sync(&channel, &mix, |route| route.gain = gain_linear)?
            .is_some()
        {
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
    pub(super) fn on_set_channel_mute(
        &mut self,
        channel: ChannelName,
        mix: MixName,
        muted: bool,
    ) -> Result<(), ExecutorError> {
        if self
            .update_route_and_sync(&channel, &mix, |route| route.muted = muted)?
            .is_some()
        {
            let _ = self.state_tx.send(StateEvent::MuteChanged {
                channel,
                mix,
                muted,
            });
        }
        Ok(())
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

/// Upper limit for user-configured channel-to-mix gain percentages (+12 dB).
const MAX_GAIN_PERCENT: f32 = 400.0;

/// Clamps non-finite or negative percentages to `0.0` and caps at `MAX_GAIN_PERCENT`.
#[inline]
fn normalize_percent(percent: f32) -> f32 {
    if !percent.is_finite() || percent <= 0.0 {
        0.0
    } else {
        percent.min(MAX_GAIN_PERCENT)
    }
}

/// Converts a normalized percentage (`0.0` = silence, `100.0` = unity) to a linear
/// multiplier.
#[inline]
fn percent_to_linear(percent: f32) -> f32 {
    normalize_percent(percent) / 100.0
}

#[cfg(test)]
mod tests {
    use super::percent_to_linear;

    #[test]
    fn test_percent_to_linear_handles_unity_attenuation_and_silence() {
        assert!((percent_to_linear(100.0) - 1.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(50.0) - 0.5).abs() < f32::EPSILON);
        assert!((percent_to_linear(150.0) - 1.5).abs() < f32::EPSILON);
        assert!((percent_to_linear(400.0) - 4.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(500.0) - 4.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(-10.0) - 0.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(f32::NAN) - 0.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(f32::INFINITY) - 0.0).abs() < f32::EPSILON);
    }
}
