/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Handling of the control events that steer the audio graph.

use pipelink_common::{
    error::ExecutorError,
    events::StateEvent,
    graph::{channel::Channel, mix::Mix},
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
        if self.graph.channels.iter().any(|c| c.name == name) {
            warn!("Channel {} already exists", name);
            return Ok(());
        }

        let channel = Channel::new(name.clone());
        self.graph.channels.push(channel);

        self.pw_tx
            .send(PipewireCommand::CreateChannel { name })
            .map_err(|e| ExecutorError::Pipewire(format!("{e:?}")))?;

        Ok(())
    }

    /// Removes a channel and destroys the `PipeWire` sink backing it.
    ///
    /// The entry stays in the graph until `PipeWire` reports the node gone,
    /// because only that report confirms the removal.
    pub(super) fn on_remove_channel(&mut self, name: ChannelName) -> Result<(), ExecutorError> {
        if let Some(channel) = self.graph.channels.iter().find(|c| c.name == name).cloned() {
            if channel.node.id.is_none() {
                // The sink was never created, so there is nothing to destroy.
                self.graph.channels.retain(|c| c.name != name);
                let _ = self
                    .state_tx
                    .send(StateEvent::ChannelRemoved { channel: name });
                return Ok(());
            }

            self.pw_tx
                .send(PipewireCommand::DeleteChannel { channel })
                .map_err(|e| ExecutorError::Pipewire(format!("{e:?}")))?;
        }

        Ok(())
    }

    /// Creates a mix together with the `PipeWire` loopback backing it.
    pub(super) fn on_create_mix(&mut self, name: MixName) -> Result<(), ExecutorError> {
        if self.graph.mixes.iter().any(|m| m.name == name) {
            warn!("Mix {} already exists", name);
            return Ok(());
        }

        let mix = Mix::new(name.clone());
        self.graph.mixes.push(mix);

        self.pw_tx
            .send(PipewireCommand::CreateMix { name })
            .map_err(|e| ExecutorError::Pipewire(format!("{e:?}")))?;

        Ok(())
    }

    /// Removes a mix and destroys the `PipeWire` loopback backing it.
    ///
    /// The entry stays in the graph until `PipeWire` reports the nodes gone,
    /// because only that report confirms the removal.
    pub(super) fn on_remove_mix(&mut self, name: MixName) -> Result<(), ExecutorError> {
        if let Some(mix) = self.graph.mixes.iter().find(|m| m.name == name).cloned() {
            if mix.sink.id.is_none() && mix.source.id.is_none() {
                // The loopback was never created, so there is nothing to destroy.
                self.graph.mixes.retain(|m| m.name != name);
                let _ = self.state_tx.send(StateEvent::MixRemoved { mix: name });
                return Ok(());
            }

            self.pw_tx
                .send(PipewireCommand::DeleteMix { mix })
                .map_err(|e| ExecutorError::Pipewire(format!("{e:?}")))?;
        }

        Ok(())
    }
}
