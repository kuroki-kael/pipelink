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
    // TODO: Add the channel to the graph only once `NodeAppeared` confirms it.
    //   That needs a pending set for the duplicate check and a delete path that
    //   resolves by name, so a request can be withdrawn before it materialises.
    pub(super) fn on_create_channel(&mut self, name: ChannelName) -> Result<(), ExecutorError> {
        if self.graph.channels.iter().any(|c| c.name == name) {
            warn!("Channel {} already exists", name);
            return Ok(());
        }

        let channel = Channel::new(name.clone());
        self.graph.channels.push(channel);

        self.pw_tx
            .send(PipewireCommand::CreateChannel { name: name.clone() })
            .map_err(|e| ExecutorError::PipewireChannel(format!("{e:?}")))?;

        let _ = self
            .state_tx
            .send(StateEvent::ChannelAdded { channel: name });

        Ok(())
    }

    /// Removes a channel and destroys the `PipeWire` sink backing it.
    pub(super) fn on_remove_channel(&mut self, name: ChannelName) -> Result<(), ExecutorError> {
        if let Some(pos) = self.graph.channels.iter().position(|c| c.name == name) {
            let channel = self.graph.channels.remove(pos);

            self.pw_tx
                .send(PipewireCommand::DeleteChannel { channel })
                .map_err(|e| ExecutorError::PipewireChannel(format!("{e:?}")))?;

            let _ = self
                .state_tx
                .send(StateEvent::ChannelRemoved { channel: name });
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
            .send(PipewireCommand::CreateMix { name: name.clone() })
            .map_err(|e| ExecutorError::PipewireChannel(format!("{e:?}")))?;

        let _ = self.state_tx.send(StateEvent::MixAdded { mix: name });

        Ok(())
    }

    /// Removes a mix and destroys the `PipeWire` loopback backing it.
    pub(super) fn on_remove_mix(&mut self, name: MixName) -> Result<(), ExecutorError> {
        if let Some(pos) = self.graph.mixes.iter().position(|m| m.name == name) {
            let mix = self.graph.mixes.remove(pos);

            self.pw_tx
                .send(PipewireCommand::DeleteMix { mix })
                .map_err(|e| ExecutorError::PipewireChannel(format!("{e:?}")))?;

            let _ = self.state_tx.send(StateEvent::MixRemoved { mix: name });
        }

        Ok(())
    }
}
