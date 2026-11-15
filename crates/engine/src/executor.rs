/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::{fmt, time::Duration};

use pipelink_common::{
    error::ExecutorError,
    events::{ControlEvent, ControlReply, StateEvent},
    graph::audio_graph::AudioGraph,
};
use pipelink_jack::JackClient;
use pipelink_pw::{Command as PipewireCommand, PipewireEvent, PipewireService};
use pipelink_resolver::service::ProcessResolver;
use pipewire::channel::Sender as ChannelSender;
use tokio::sync::{
    broadcast::Sender as Broadcast,
    mpsc::{Receiver, UnboundedReceiver},
    oneshot::Sender,
};

use crate::config::topology::ConfigGraph;

mod control_events;
mod pipewire_events;
mod query_events;

/// Execution coordinator for the audio graph
pub struct Executor {
    config: ConfigGraph,
    graph: AudioGraph,
    control_rx: Receiver<Envelope>,
    state_tx: Broadcast<StateEvent>,
    pw_service: Option<PipewireService>,
    pw_tx: ChannelSender<PipewireCommand>,
    pw_rx: UnboundedReceiver<PipewireEvent>,
    resolver: ProcessResolver,
    jack: JackClient,
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
        control_rx: Receiver<Envelope>,
        state_tx: Broadcast<StateEvent>,
    ) -> Result<Self, ExecutorError> {
        // tokio mpsc for incoming pipewire events
        let (pw_event_tx, pw_event_rx) = tokio::sync::mpsc::unbounded_channel();

        // start pipewire event loop in a dedicated background thread
        let pw_service = PipewireService::start(pw_event_tx)
            .map_err(|e| ExecutorError::Pipewire(e.to_string()))?;
        let pw_tx = pw_service.command_sender();

        let jack = JackClient::start().map_err(|e| ExecutorError::Jack(e.to_string()))?;

        Ok(Self {
            config,
            graph: AudioGraph::default(),
            control_rx,
            state_tx,
            pw_service: Some(pw_service),
            pw_tx,
            pw_rx: pw_event_rx,
            resolver: ProcessResolver::new(),
            jack,
        })
    }

    /// Primary execution loop handling control inputs and pipewire events
    ///
    /// # Errors
    /// Returns an [`ExecutorError`] if sending a command fails.
    pub async fn run(&mut self) -> Result<(), ExecutorError> {
        const RECLAIM_POLL_INTERVAL: Duration = Duration::from_millis(10);
        let mut reclaim_interval = tokio::time::interval(RECLAIM_POLL_INTERVAL);
        reclaim_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                Some(envelope) = self.control_rx.recv() => {
                    if let Some(event) = self.handle_query(envelope) {
                        self.handle_control_event(event)?;
                    }
                }
                Some(event) = self.pw_rx.recv() => {
                    self.handle_pipewire_event(event);
                }
                _ = reclaim_interval.tick(), if self.jack.pending_reclaims() > 0 => {
                    self.jack.reclaim();
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
            ControlEvent::AddChannelMix { channel, mix } => {
                self.on_add_channel_mix(channel, mix)?;
            },
            ControlEvent::RemoveChannelMix { channel, mix } => {
                self.on_remove_channel_mix(channel, mix)?;
            },
            ControlEvent::SetChannelGain {
                channel,
                mix,
                percent,
            } => {
                self.on_set_channel_gain(channel, mix, percent)?;
            },
            ControlEvent::SetChannelMute {
                channel,
                mix,
                muted,
            } => {
                self.on_set_channel_mute(channel, mix, muted)?;
            },
            ControlEvent::SelectGlobalActiveLayer { .. }
            | ControlEvent::SelectChannelActiveLayer { .. }
            | _ => {},
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
                name,
                direction,
                channel,
            } => {
                self.on_port_appeared(id, node_id, name.as_deref(), direction, channel);
            },
            PipewireEvent::LinkAppeared {
                id,
                out_node,
                in_node,
                out_port,
                in_port,
            } => {
                self.on_link_appeared(id, out_node, in_node, out_port, in_port);
            },
            PipewireEvent::LinkRemoved { id } => {
                self.on_link_removed(id);
            },
            PipewireEvent::NodeInfo {
                id, node_name, pid, ..
            } => {
                self.on_node_info(id, node_name, pid);
            },
            PipewireEvent::GlobalRemoved { id } => {
                self.on_global_removed(id);
            },
            PipewireEvent::GlobalRemovalFailed { id, reason } => {
                self.on_global_removal_failed(id, &reason);
            },
            PipewireEvent::ChannelCreationFailed { name, reason } => {
                self.on_channel_creation_failed(name, &reason);
            },
            PipewireEvent::MixCreationFailed { name, reason } => {
                self.on_mix_creation_failed(name, &reason);
            },
            PipewireEvent::MixRemovalFailed { name, reason } => {
                self.on_mix_removal_failed(name, &reason);
            },
            _ => {},
        }
    }

    /// Sends the answer back to the client that asked for it.
    fn reply(reply_tx: Option<Sender<ControlReply>>, msg: ControlReply) {
        if let Some(reply_tx) = reply_tx {
            let _ = reply_tx.send(msg);
        }
    }
}

/// A control event together with the channel its answer goes back through.
#[derive(Debug)]
pub struct Envelope {
    /// The requested state change or query.
    pub event: ControlEvent,
    /// Where the answer goes, `None` if the event asks for no answer.
    pub reply: Option<Sender<ControlReply>>,
}
