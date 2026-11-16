/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Handling of the control events that steer the audio graph.

mod routes;

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

        self.pending_channel_creates.insert(name.clone());
        self.config.ensure_channel(&name);
        self.storage.save_config(&self.config);

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

        let Some(channel) = channels
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(&name))
            .cloned()
        else {
            let _ = self.state_tx.send(StateEvent::ChannelRemovalFailed {
                channel: name,
                error: "there is no channel with this name".to_string(),
            });
            return Ok(());
        };

        let canonical = channel.name.clone();
        if channel.node.id.is_none() {
            self.graph.remove_channel(&canonical);
            self.clear_channel_app_routes(&canonical, true);
            if let Err(err) = self
                .jack
                .remove_channel(&canonical, self.graph.gains_snapshot())
            {
                warn!("Failed to remove JACK channel '{canonical}': {err}");
            }
            let _ = self
                .state_tx
                .send(StateEvent::ChannelRemoved { channel: canonical });
            return Ok(());
        }

        self.pending_channel_removals.insert(canonical);
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

        self.pending_mix_creates.insert(name.clone());
        self.maybe_auto_select_active_mix(&name);
        self.config.ensure_mix(&name);
        self.storage.save_config(&self.config);

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

        let Some(mix) = mixes
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(&name))
            .cloned()
        else {
            let _ = self.state_tx.send(StateEvent::MixRemovalFailed {
                mix: name,
                error: "there is no mix with this name".to_string(),
            });
            return Ok(());
        };

        let canonical = mix.name.clone();
        if mix.sink.id.is_none() && mix.source.id.is_none() {
            // The loopback was never created, so there is nothing to destroy.
            self.graph.remove_mix(&canonical);
            if self.config.remove_mix(&canonical) {
                self.storage.save_config(&self.config);
            }
            if let Err(err) = self
                .jack
                .remove_mix(&canonical, self.graph.gains_snapshot())
            {
                warn!("Failed to remove JACK mix '{canonical}': {err}");
            }
            let _ = self
                .state_tx
                .send(StateEvent::MixRemoved { mix: canonical });
            self.ensure_active_mix();
            return Ok(());
        }

        self.pending_mix_removals.insert(canonical);
        self.pw_tx
            .send(PipewireCommand::DeleteMix { mix })
            .map_err(|e| ExecutorError::Pipewire(format!("{e:?}")))?;

        Ok(())
    }
}
