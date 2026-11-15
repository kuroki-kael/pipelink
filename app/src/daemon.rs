/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use pipelink_engine::{ConfigGraph, executor::Executor};

use crate::ipc::{handle_client, start_server};

/// Runs the audio routing daemon until the executor stops.
///
/// # Errors
/// Returns an error if the executor cannot be created or fails while running.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = tracing_subscriber::fmt::try_init() {
        eprintln!("Tracing-Subscriber nicht initialisierbar: {e}");
    }
    println!("Starte Audio-Routing Daemon...");

    let (control_tx, control_rx) = tokio::sync::mpsc::channel(32);
    let (state_tx, _) = tokio::sync::broadcast::channel(32);

    let state_broadcaster = state_tx.clone();

    let mut executor = Executor::new(ConfigGraph::default(), control_rx, state_tx)?;

    let (listener, socket_guard) = start_server().await?;

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((socket, _addr)) => {
                    let control_tx = control_tx.clone();
                    let state_rx = state_broadcaster.subscribe();
                    tokio::spawn(handle_client(socket, control_tx, state_rx));
                },
                Err(e) => {
                    eprintln!("connection error: {e}");
                },
            }
        }
    });

    #[cfg(feature = "web")]
    {
        println!("Web-Interface verfügbar auf http://localhost:8080");
        // webserver::start();
    }

    #[cfg(feature = "midi")]
    {
        // midi_handler::init();
    }

    executor.run().await?;

    // The socket of the daemon is gone when the daemon ends.
    drop(socket_guard);

    Ok(())
}
