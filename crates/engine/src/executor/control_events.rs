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
use pipelink_pw::Command as PipewireCommand;
use tracing::warn;

use crate::executor::Executor;

impl Executor {
    /// Creates a channel together with the `PipeWire` sink backing it.
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

        self.pw_tx
            .send(PipewireCommand::CreateChannel { name })
            .map_err(|e| ExecutorError::Pipewire(format!("{e:?}")))?;

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

    /// Creates a mix together with the `PipeWire` loopback backing it.
    pub(super) fn on_create_mix(&mut self, name: MixName) -> Result<(), ExecutorError> {
        if let Err(err) = self.graph.add_mix(name.clone()) {
            warn!("{err}");
            let _ = self.state_tx.send(StateEvent::MixCreationFailed {
                mix: name,
                error: err.to_string(),
            });
            return Ok(());
        }

        self.pw_tx
            .send(PipewireCommand::CreateMix { name })
            .map_err(|e| ExecutorError::Pipewire(format!("{e:?}")))?;

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
            let _ = self.state_tx.send(StateEvent::MixRemoved { mix: name });
            return Ok(());
        }

        self.pw_tx
            .send(PipewireCommand::DeleteMix { mix })
            .map_err(|e| ExecutorError::Pipewire(format!("{e:?}")))?;

        Ok(())
    }
}
