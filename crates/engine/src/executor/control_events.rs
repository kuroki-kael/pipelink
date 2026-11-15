/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Handling of the control events that steer the audio graph.

use pipelink_common::{
    error::ExecutorError,
    events::StateEvent,
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

        if let Err(err) = self.jack.create_channel(&name) {
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

        if let Err(err) = self.pw_tx.send(PipewireCommand::CreateChannel {
            name: name.clone(),
        }) {
            self.graph.remove_channel(&name);
            let _ = self.jack.remove_channel(&name);
            return Err(ExecutorError::Pipewire(format!("{err:?}")));
        }

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
            if let Err(err) = self.jack.remove_channel(&name) {
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

        if let Err(err) = self.jack.create_mix(&name) {
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

        if let Err(err) = self.pw_tx.send(PipewireCommand::CreateMix {
            name: name.clone(),
        }) {
            self.graph.remove_mix(&name);
            let _ = self.jack.remove_mix(&name);
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
            if let Err(err) = self.jack.remove_mix(&name) {
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
        let Some(route) = self.graph.get_route_mut(&channel, &mix) else {
            warn!("Cannot add route ({channel} -> {mix}): channel or mix not found");
            return Ok(());
        };
        route.active = true;

        self.sync_jack_route(&channel, &mix)?;
        let _ = self
            .state_tx
            .send(StateEvent::MixChannelAdded { channel, mix });
        Ok(())
    }

    /// Deactivates the route from `channel` into `mix` and silences it in JACK.
    pub(super) fn on_remove_channel_mix(
        &mut self,
        channel: ChannelName,
        mix: MixName,
    ) -> Result<(), ExecutorError> {
        let Some(route) = self.graph.get_route_mut(&channel, &mix) else {
            warn!("Cannot remove route ({channel} -> {mix}): channel or mix not found");
            return Ok(());
        };
        route.active = false;

        self.sync_jack_route(&channel, &mix)?;
        let _ = self
            .state_tx
            .send(StateEvent::MixChannelRemoved { channel, mix });
        Ok(())
    }

    /// Updates the gain (in percent) of `(channel, mix)` and pushes the linear gain to JACK.
    pub(super) fn on_set_channel_gain(
        &mut self,
        channel: ChannelName,
        mix: MixName,
        percent: f32,
    ) -> Result<(), ExecutorError> {
        let gain_linear = percent_to_linear(percent);
        let Some(route) = self.graph.get_route_mut(&channel, &mix) else {
            warn!("Cannot set gain on ({channel} -> {mix}): channel or mix not found");
            return Ok(());
        };
        route.gain = gain_linear;

        self.sync_jack_route(&channel, &mix)?;
        let _ = self.state_tx.send(StateEvent::GainChanged {
            channel,
            mix,
            gain_linear,
            percent,
        });
        Ok(())
    }

    /// Updates the mute flag of `(channel, mix)` and pushes the effective gain to JACK.
    pub(super) fn on_set_channel_mute(
        &mut self,
        channel: ChannelName,
        mix: MixName,
        muted: bool,
    ) -> Result<(), ExecutorError> {
        let Some(route) = self.graph.get_route_mut(&channel, &mix) else {
            warn!("Cannot set mute on ({channel} -> {mix}): channel or mix not found");
            return Ok(());
        };
        route.muted = muted;

        self.sync_jack_route(&channel, &mix)?;
        let _ = self.state_tx.send(StateEvent::MuteChanged {
            channel,
            mix,
            muted,
        });
        Ok(())
    }

    fn sync_jack_route(
        &mut self,
        channel: &ChannelName,
        mix: &MixName,
    ) -> Result<(), ExecutorError> {
        let Some(route) = self.graph.get_route(channel, mix) else {
            return Ok(());
        };
        self.jack
            .set_route(channel, mix, route.effective_gain())
            .map_err(|err| ExecutorError::Jack(err.to_string()))
    }
}

/// Converts a percentage (`0.0` = silence, `100.0` = unity) to a linear gain multiplier.
#[inline]
fn percent_to_linear(percent: f32) -> f32 {
    if !percent.is_finite() || percent <= 0.0 {
        0.0
    } else {
        percent / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::percent_to_linear;

    #[test]
    fn test_percent_to_linear_handles_unity_attenuation_and_silence() {
        assert!((percent_to_linear(100.0) - 1.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(50.0) - 0.5).abs() < f32::EPSILON);
        assert!((percent_to_linear(150.0) - 1.5).abs() < f32::EPSILON);
        assert!((percent_to_linear(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(-10.0) - 0.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(f32::NAN) - 0.0).abs() < f32::EPSILON);
    }
}
