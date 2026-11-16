/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::{collections::HashSet, fmt, time::Duration};

use pipelink_common::{
    error::ExecutorError,
    events::{ControlEvent, ControlReply, StateEvent},
    graph::audio_graph::AudioGraph,
    types::{ChannelName, MixName},
};
use pipelink_jack::JackClient;
use pipelink_pw::{Command as PipewireCommand, PipewireEvent, PipewireService};
use pipelink_resolver::service::ProcessResolver;
use pipewire::channel::Sender as ChannelSender;
use tokio::sync::{
    broadcast::Sender as Broadcast,
    mpsc::{Receiver, UnboundedReceiver},
    oneshot::Sender,
    watch,
};

use crate::config::{
    state::RuntimeState,
    storage::{PersistenceStore, StoragePaths},
    topology::ConfigGraph,
};

mod app_routes;
mod bootstrap;
mod control_events;
mod output_routing;
mod pipewire_events;
mod query_events;

/// Execution coordinator for the audio graph
pub struct Executor {
    config: ConfigGraph,
    graph: AudioGraph,
    storage: PersistenceStore,
    pending_channel_creates: HashSet<ChannelName>,
    pending_mix_creates: HashSet<MixName>,
    pending_channel_removals: HashSet<ChannelName>,
    pending_mix_removals: HashSet<MixName>,
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
            .field("storage", &self.storage)
            .field("control_rx", &self.control_rx)
            .field("state_tx", &self.state_tx)
            .field("pw_service", &self.pw_service)
            .field("resolver", &self.resolver)
            .finish_non_exhaustive()
    }
}

impl Executor {
    /// Creates a new `Executor` instance from a static topology definition
    /// without disk persistence.
    ///
    /// # Errors
    /// Returns an [`ExecutorError`] if initializing the `PipeWire` service fails.
    pub fn new(
        config: ConfigGraph,
        control_rx: Receiver<Envelope>,
        state_tx: Broadcast<StateEvent>,
    ) -> Result<Self, ExecutorError> {
        Self::with_persistence(config, &RuntimeState::default(), None, control_rx, state_tx)
    }

    /// Creates a new `Executor` instance backed by persistent storage and restores
    /// the saved topology and runtime mixer state on startup.
    ///
    /// # Errors
    /// Returns an [`ExecutorError`] if initializing `PipeWire`/`JACK` or restoring
    /// persisted channels and mixes fails.
    pub fn with_persistence(
        config: ConfigGraph,
        initial_state: &RuntimeState,
        storage_paths: Option<StoragePaths>,
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

        let mut executor = Self {
            config,
            graph: AudioGraph::default(),
            storage: PersistenceStore::new(storage_paths),
            pending_channel_creates: HashSet::new(),
            pending_mix_creates: HashSet::new(),
            pending_channel_removals: HashSet::new(),
            pending_mix_removals: HashSet::new(),
            control_rx,
            state_tx,
            pw_service: Some(pw_service),
            pw_tx,
            pw_rx: pw_event_rx,
            resolver: ProcessResolver::new(),
            jack,
        };
        executor.bootstrap(initial_state)?;
        Ok(executor)
    }

    /// Primary execution loop handling control inputs, `PipeWire` events,
    /// debounced state flushes, and graceful shutdown signals.
    ///
    /// # Errors
    /// Returns an [`ExecutorError`] if sending a command fails.
    pub async fn run(&mut self, mut shutdown: watch::Receiver<bool>) -> Result<(), ExecutorError> {
        const RECLAIM_POLL_INTERVAL: Duration = Duration::from_millis(10);
        let mut reclaim_interval = tokio::time::interval(RECLAIM_POLL_INTERVAL);
        reclaim_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        if !*shutdown.borrow() {
            loop {
                let flush_deadline = self.storage.state_flush_deadline();
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
                    () = sleep_until_optional(flush_deadline), if flush_deadline.is_some() => {
                        self.storage.flush_state_if_dirty(&self.graph);
                    }
                    res = shutdown.changed() => {
                        if res.is_err() || *shutdown.borrow() {
                            break;
                        }
                    }
                    else => {
                        break;
                    }
                }
            }
        }

        self.storage.flush_state_if_dirty(&self.graph);
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
            ControlEvent::RouteApp { app, channel } => {
                self.on_route_app(app, &channel);
            },
            ControlEvent::UnrouteApp { app } => {
                self.on_unroute_app(app);
            },
            ControlEvent::SelectGlobalActiveLayer { mix } => {
                self.on_select_active_mix(mix);
            },
            ControlEvent::SetOutput { output } => {
                self.on_set_output(&output);
            },
            ControlEvent::SelectChannelActiveLayer { .. } | _ => {},
        }

        Ok(())
    }

    fn handle_pipewire_event(&mut self, event: PipewireEvent) {
        match event {
            PipewireEvent::NodeAppeared {
                id,
                name,
                media_class,
                serial,
                description,
                nick,
                is_hardware,
                ..
            } => {
                self.on_node_appeared(
                    id,
                    &name,
                    &media_class,
                    serial,
                    description,
                    nick,
                    is_hardware,
                );
            },
            PipewireEvent::MetadataAppeared { .. } => {
                self.on_metadata_appeared();
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

async fn sleep_until_optional(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(target) => tokio::time::sleep_until(target).await,
        None => std::future::pending().await,
    }
}
