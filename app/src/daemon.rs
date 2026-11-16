/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use pipelink_common::logger::{self, LogConfig};
use pipelink_engine::{ConfigGraph, StoragePaths, executor::Executor};
use tokio::{
    signal::unix::{SignalKind, signal},
    sync::watch,
};
use tracing::{info, warn};

use crate::ipc::{handle_client, start_server};

/// Runs the audio routing daemon until the executor stops or receives `SIGINT`/`SIGTERM`.
///
/// # Errors
/// Returns an error if the executor cannot be created or fails while running.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = logger::init(&LogConfig::default()) {
        eprintln!("Failed to initialize logger: {e}");
    }
    info!("Starting audio routing daemon");

    let (control_tx, control_rx) = tokio::sync::mpsc::channel(32);
    let (state_tx, _) = tokio::sync::broadcast::channel(32);

    let state_broadcaster = state_tx.clone();

    let mut executor = match StoragePaths::discover() {
        Some(paths) => {
            let config = paths.load_or_init_config();
            let state = paths.load_state();
            Executor::with_persistence(config, &state, Some(paths), control_rx, state_tx)?
        },
        None => Executor::new(ConfigGraph::default(), control_rx, state_tx)?,
    };

    let (listener, socket_guard) = start_server().await?;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;
    tokio::spawn(async move {
        tokio::select! {
            _ = sigint.recv() => {},
            _ = sigterm.recv() => {},
        }
        let _ = shutdown_tx.send(true);
    });

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((socket, _addr)) => {
                    let control_tx = control_tx.clone();
                    let state_rx = state_broadcaster.subscribe();
                    tokio::spawn(handle_client(socket, control_tx, state_rx));
                },
                Err(e) => {
                    warn!("IPC client connection error: {e}");
                },
            }
        }
    });

    #[cfg(feature = "web")]
    {
        info!("Web interface available at http://localhost:8080");
        // webserver::start();
    }

    #[cfg(feature = "midi")]
    {
        // midi_handler::init();
    }

    executor.run(shutdown_rx).await?;

    // The socket of the daemon is gone when the daemon ends.
    drop(socket_guard);

    Ok(())
}
