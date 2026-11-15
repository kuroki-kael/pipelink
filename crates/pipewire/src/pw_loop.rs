/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::{fmt, rc::Rc, thread};

use pipelink_common::types::GlobalId;
use pipewire::{channel, context::ContextRc, main_loop::MainLoopRc};
use tokio::sync::mpsc::UnboundedSender as TokioSender;
use tracing::{debug, error, warn};

use crate::{Command, LoopState, PipewireEvent};

/// Handle to the `PipeWire` background loop running in its own thread.
pub struct PipewireService {
    /// Sender which the main thread uses to send commands
    cmd_tx: channel::Sender<Command>,
    /// Handle on the pipewire background thread
    thread: Option<thread::JoinHandle<()>>,
}

impl fmt::Debug for PipewireService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PipewireService")
            .field("cmd_tx", &"channel::Sender<Command>")
            .field("thread", &self.thread)
            .finish()
    }
}

impl PipewireService {
    /// Starts the `PipeWire` service in a background thread.
    ///
    /// # Errors
    /// Returns a [`pipewire::Error`] if creating or spawning the thread fails.
    pub fn start(event_tx: TokioSender<PipewireEvent>) -> Result<Self, pipewire::Error> {
        pipewire::init();

        // Cross-Thread channel: Main-Thread -> Pipewire Loop
        let (cmd_tx, cmd_rx) = channel::channel::<Command>();

        let thread = thread::Builder::new()
            .name("PipewireService".to_string())
            .spawn(move || {
                if let Err(e) = Self::run_loop(cmd_rx, &event_tx) {
                    error!("Pipewire loop exited with error: {e:?}");
                }
            })
            .map_err(|_| pipewire::Error::NoMemory)?;

        Ok(Self {
            cmd_tx,
            thread: Some(thread),
        })
    }

    /// Returns a sender for commands addressed to the background loop.
    #[must_use]
    pub fn command_sender(&self) -> channel::Sender<Command> {
        self.cmd_tx.clone()
    }

    fn run_loop(
        cmd_rx: channel::Receiver<Command>,
        event_tx: &TokioSender<PipewireEvent>,
    ) -> Result<(), pipewire::Error> {
        let main_loop = MainLoopRc::new(None)?;
        let context = ContextRc::new(&main_loop, None)?;
        let core = context.connect_rc(None)?;
        let registry = core.get_registry_rc()?;

        let state = Rc::new(LoopState::new());

        let state_cmd = state.clone();
        let state_reg = state.clone();
        let state_remove = state;

        let reg_cmd = registry.clone();
        let reg_global = registry.clone();

        let _loop_clone = main_loop.clone();
        let event_tx_actions = event_tx.clone();
        let event_tx_remove = event_tx.clone();
        let event_tx_commands = event_tx.clone();

        // Bind Pipewire channel to loop
        let _commands = cmd_rx.attach(main_loop.loop_(), move |cmd| {
            state_cmd.handle_pipewire_command(cmd, &core, &context, &reg_cmd, &event_tx_commands);
        });

        let _registry_listener = registry
            .add_listener_local()
            .global(move |obj| {
                state_reg.handle_pipewire_event(obj, &event_tx_actions, &reg_global);
            })
            .global_remove(move |id| {
                state_remove.global_removed(GlobalId::from(id), &event_tx_remove);
            })
            .register();

        if let Err(e) = event_tx.send(PipewireEvent::Ready) {
            warn!("Failed to notify Main Thread: {e}");
        }

        debug!("Background thread pipewire loop running");
        main_loop.run();

        Ok(())
    }
}
