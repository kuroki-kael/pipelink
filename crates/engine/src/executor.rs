/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::fmt;

use pipelink_common::{
    error::ExecutorError,
    events::{ControlEvent, StateEvent},
};
use pipelink_pw::{Command as PipewireCommand, PipewireEvent, PipewireService};
use pipelink_resolver::service::ProcessResolver;
use pipewire::channel::Sender as ChannelSender;
use tokio::sync::{broadcast::Sender as Broadcast, mpsc::Receiver};

use crate::{AudioGraph, config::topology::ConfigGraph};

mod control_events;
mod pipewire_events;

/// Execution coordinator for the audio graph
pub struct Executor {
    config: ConfigGraph,
    graph: AudioGraph,
    control_rx: Receiver<ControlEvent>,
    state_tx: Broadcast<StateEvent>,
    pw_service: Option<PipewireService>,
    pw_tx: ChannelSender<PipewireCommand>,
    pw_rx: Receiver<PipewireEvent>,
    resolver: ProcessResolver,
}

impl fmt::Debug for Executor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Executor")
            .field("config", &self.config)
            .field("graph", &self.graph)
            .field("control_rx", &self.control_rx)
            .field("state_tx", &self.state_tx)
            .field("pw_service", &self.pw_service)
            .field("resolver", &self.resolver)
            .finish_non_exhaustive()
    }
}

impl Executor {
    /// Creates a new `Executor` instance from a static topology definition.
    ///
    /// # Errors
    /// Returns an [`ExecutorError`] if initializing the `PipeWire` service fails.
    pub fn new(
        config: ConfigGraph,
        control_rx: Receiver<ControlEvent>,
        state_tx: Broadcast<StateEvent>,
    ) -> Result<Self, ExecutorError> {
        // tokio mpsc for incoming pipewire events
        let (pw_event_tx, pw_event_rx) = tokio::sync::mpsc::channel(128);

        // start pipewire event loop in a dedicated background thread
        let pw_service = PipewireService::start(pw_event_tx)
            .map_err(|e| ExecutorError::Pipewire(e.to_string()))?;
        let pw_tx = pw_service.command_sender();

        Ok(Self {
            config,
            graph: AudioGraph::default(),
            control_rx,
            state_tx,
            pw_service: Some(pw_service),
            pw_tx,
            pw_rx: pw_event_rx,
            resolver: ProcessResolver::new(),
        })
    }

    /// Primary execution loop handling control inputs and pipewire events
    ///
    /// # Errors
    /// Returns an [`ExecutorError`] if sending a command fails.
    pub async fn run(&mut self) -> Result<(), ExecutorError> {
        loop {
            tokio::select! {
                Some(ctrl) = self.control_rx.recv() => {
                    self.handle_control_event(ctrl)?;
                }
                Some(event) = self.pw_rx.recv() => {
                    self.handle_pipewire_event(event);
                }
                else => {
                    break;
                }
            }
        }

        let _ = self.pw_service.take();

        Ok(())
    }

    fn handle_control_event(&mut self, ctrl: ControlEvent) -> Result<(), ExecutorError> {
        match ctrl {
            ControlEvent::CreateChannel { name } => self.on_create_channel(name)?,
            ControlEvent::RemoveChannel { name } => self.on_remove_channel(name)?,
            ControlEvent::CreateMix { name } => self.on_create_mix(name)?,
            ControlEvent::RemoveMix { name } => self.on_remove_mix(name)?,
            ControlEvent::SetChannelGain { .. }
            | ControlEvent::SetChannelMute { .. }
            | ControlEvent::SelectGlobalActiveLayer { .. }
            | ControlEvent::SelectChannelActiveLayer { .. }
            | ControlEvent::AddChannelMix { .. }
            | ControlEvent::RemoveChannelMix { .. } => {},
        }

        Ok(())
    }

    fn handle_pipewire_event(&mut self, event: PipewireEvent) {
        match event {
            PipewireEvent::NodeAppeared {
                id, name, serial, ..
            } => {
                self.on_node_appeared(id, &name, serial);
            },
            PipewireEvent::PortAppeared {
                id,
                node_id,
                direction,
                channel,
            } => {
                self.on_port_appeared(id, node_id, direction, channel);
            },
            PipewireEvent::NodeInfo {
                id, node_name, pid, ..
            } => {
                self.on_node_info(id, node_name, pid);
            },
            PipewireEvent::GlobalRemoved { id } => {
                self.on_global_removed(id);
            },
            PipewireEvent::ChannelCreationFailed { name, reason } => {
                self.on_channel_creation_failed(name, &reason);
            },
            PipewireEvent::MixCreationFailed { name, reason } => {
                self.on_mix_creation_failed(name, &reason);
            },
            _ => {},
        }
    }
}
