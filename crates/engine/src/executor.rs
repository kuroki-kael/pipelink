/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::{collections::HashSet, fmt, time::Duration};

use pipelink_common::{
    error::ExecutorError,
    events::{ControlReply, StateEvent},
    graph::audio_graph::AudioGraph,
    types::{ChannelName, MixName},
};
pub use pipelink_ipc::Envelope;
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
mod dispatch;
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

    /// Sends the answer back to the client that asked for it.
    fn reply(reply_tx: Option<Sender<ControlReply>>, msg: ControlReply) {
        if let Some(reply_tx) = reply_tx {
            let _ = reply_tx.send(msg);
        }
    }
}

async fn sleep_until_optional(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(target) => tokio::time::sleep_until(target).await,
        None => std::future::pending().await,
    }
}
