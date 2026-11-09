/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use pipelink_common::events::ControlEvent;
use pipelink_engine::{
    ConfigGraph,
    executor::{Envelope, Executor},
};

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
    let (state_tx, mut state_rx) = tokio::sync::broadcast::channel(32);

    let mut executor = Executor::new(ConfigGraph::default(), control_rx, state_tx)?;

    // Test-Task: Schickt nach 500ms CreateChannel und nach weiteren 500ms RemoveChannel
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        println!("🚀 Sende CreateChannel für 'test1'...");
        if let Err(e) = control_tx
            .send(Envelope {
                event: ControlEvent::CreateChannel {
                    name: "test1".into(),
                },
                reply: None,
            })
            .await
        {
            eprintln!("Fehler beim Senden von CreateChannel: {e:?}");
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        println!("🚀 Sende CreateMix für 'test2'...");
        if let Err(e) = control_tx
            .send(Envelope {
                event: ControlEvent::CreateMix {
                    name: "test2".into(),
                },
                reply: None,
            })
            .await
        {
            eprintln!("Fehler beim Senden von CreateChannel: {e:?}");
        }

        tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
        println!("🗑️ Sende RemoveChannel für 'test1'...");
        if let Err(e) = control_tx
            .send(Envelope {
                event: ControlEvent::RemoveChannel {
                    name: "test1".into(),
                },
                reply: None,
            })
            .await
        {
            eprintln!("Fehler beim Senden von RemoveChannel: {e:?}");
        }

        tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
        println!("🗑️ Sende RemoveMix für 'test2'...");
        if let Err(e) = control_tx
            .send(Envelope {
                event: ControlEvent::RemoveMix {
                    name: "test2".into(),
                },
                reply: None,
            })
            .await
        {
            eprintln!("Fehler beim Senden von RemoveChannel: {e:?}");
        }
    });

    // State-Listener
    tokio::spawn(async move {
        while let Ok(event) = state_rx.recv().await {
            println!("📢 State Event empfangen: {event:?}");
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
    Ok(())
}
