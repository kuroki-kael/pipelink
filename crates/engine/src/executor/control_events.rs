/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Handling of the control events that steer the audio graph.

use pipelink_common::{
    error::ExecutorError, events::StateChangeEvent, graph::channel::Channel, types::ChannelName,
};
use pipelink_pw::Command as PipewireCommand;
use tracing::warn;

use crate::executor::Executor;

impl Executor {
    /// Creates a channel together with the `PipeWire` sink backing it.
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
            .send(StateChangeEvent::ChannelAdded { channel: name });

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
                .send(StateChangeEvent::ChannelRemoved { channel: name });
        }

        Ok(())
    }
}
