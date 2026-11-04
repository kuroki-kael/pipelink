/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::fmt;

use pipelink_common::{
    error::ExecutorError,
    events::{ControlEvent, StateChangeEvent},
    graph::{
        app_node::AppNode,
        channel::Channel,
        node::{NativeNode, Node as PipewireNode},
    },
};
use pipelink_pw::{Command as PipewireCommand, PipewireEvent, PipewireService};
use pipelink_resolver::{is_process_alive, service::ProcessResolver};
use pipewire::channel::Sender as ChannelSender;
use tokio::sync::{broadcast::Sender as Broadcast, mpsc::Receiver};
use tracing::{info, warn};

use crate::{AudioGraph, config::topology::ConfigGraph};

/// Execution coordinator for the audio graph
pub struct Executor {
    config: ConfigGraph,
    graph: AudioGraph,
    control_rx: Receiver<ControlEvent>,
    state_tx: Broadcast<StateChangeEvent>,
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
            .finish_non_exhaustive()
    }
}

impl Executor {
    /// Creates a new `Executor` instance.
    ///
    /// # Errors
    /// Returns an [`ExecutorError`] if initializing the `PipeWire` service fails.
    pub fn new(
        config: ConfigGraph,
        control_rx: Receiver<ControlEvent>,
        state_tx: Broadcast<StateChangeEvent>,
    ) -> Result<Self, ExecutorError> {
        // tokio mpsc for incoming pipewire events
        let (pw_event_tx, pw_event_rx) = tokio::sync::mpsc::channel::<PipewireEvent>(100);

        // start pipewire event loop in a dedicated background thread
        let pw_service = PipewireService::start(pw_event_tx)
            .map_err(|e| ExecutorError::Pipewire(e.to_string()))?;

        let pw_cmd_tx = pw_service.command_sender();

        let resolver = ProcessResolver::default();

        Ok(Self {
            config,
            graph: AudioGraph::new(),
            control_rx,
            state_tx,
            pw_service: Some(pw_service),
            pw_tx: pw_cmd_tx,
            pw_rx: pw_event_rx,
            resolver,
        })
    }

    /// Primary execution loop handling control inputs and pipewire events
    ///
    /// # Errors
    /// Returns an [`ExecutorError`] if handling any event fails
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
                    info!("All event channels closed, executor exiting");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_control_event(&mut self, ctrl: ControlEvent) -> Result<(), ExecutorError> {
        match ctrl {
            ControlEvent::CreateChannel { name } => {
                if self.graph.channels.iter().any(|c| c.get_name() == name) {
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
            },
            ControlEvent::RemoveChannel { name } => {
                if let Some(pos) = self
                    .graph
                    .channels
                    .iter()
                    .position(|c| c.get_name() == name)
                {
                    let channel = self.graph.channels.remove(pos);

                    self.pw_tx
                        .send(PipewireCommand::DeleteChannel { channel })
                        .map_err(|e| ExecutorError::PipewireChannel(format!("{e:?}")))?;

                    let _ = self
                        .state_tx
                        .send(StateChangeEvent::ChannelRemoved { channel: name });
                } else {
                    warn!("Channel {} not found", name);
                }
            },

            ControlEvent::SetChannelGain { .. }
            | ControlEvent::SetChannelMute { .. }
            | ControlEvent::CreateMix { .. }
            | ControlEvent::RemoveMix { .. }
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
                if let Some(channel_name) = name.strip_prefix("pipelink.channel.")
                    && let Some(ch) = self
                        .graph
                        .channels
                        .iter_mut()
                        .find(|c| c.get_name() == channel_name)
                {
                    ch.set_id(Some(id));
                    ch.get_node_mut().set_serial(serial);
                    info!("Channel '{channel_name}' has the id {id}");
                }
            },
            PipewireEvent::PortAppeared {
                id,
                node_id,
                direction,
                channel,
                node_name,
            } => {
                if let Some(channel_name) = node_name.strip_prefix("pipelink.channel.")
                    && let Some(ch) = self
                        .graph
                        .channels
                        .iter_mut()
                        .find(|c| c.get_name() == channel_name)
                    && ch.on_port_appeared(id, node_id, &direction, &channel)
                    && ch.is_ready()
                {
                    info!("Channel '{channel_name}' is ready");
                }
            },
            PipewireEvent::NodeInfo { id, pid, nick, .. } => {
                if let Some(pid) = pid
                    && let Some(app) = self.resolver.resolve(pid)
                {
                    info!(
                        "Resolved stream node {id} to app '{}' ({})",
                        app.display_name, app.app_name
                    );

                    let mut node = PipewireNode::default();
                    node.set_id(Some(id));
                    if let Some(nick_str) = nick {
                        node.set_name(&nick_str);
                    } else {
                        node.set_name(&app.app_name);
                    }

                    if let Some(existing) = self
                        .graph
                        .apps
                        .iter_mut()
                        .find(|a| a.node.get_id() == Some(id))
                    {
                        existing.app = app;
                        existing.node = node;
                    } else {
                        self.graph.apps.push(AppNode::new(app, node));
                    }
                }
            },
            PipewireEvent::GlobalRemoved { id } => {
                if let Some(pos) = self
                    .graph
                    .apps
                    .iter()
                    .position(|a| a.node.get_id() == Some(id))
                {
                    let removed = self.graph.apps.remove(pos);
                    let pid = removed.app.pid;
                    let other_active = self.graph.apps.iter().any(|a| a.app.pid == pid);
                    if !other_active && !is_process_alive(pid) {
                        self.resolver.invalidate(pid);
                        info!("Process {pid} exited, evicted from resolver cache");
                    }
                }
            },
            _ => {},
        }
    }
}
