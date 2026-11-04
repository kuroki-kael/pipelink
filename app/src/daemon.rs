/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use pipelink::{
    common::events::ControlEvent,
    config::topology::ConfigGraph,
    engine::{AudioGraph, executor::Executor},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("Starte Audio-Routing Daemon...");

    let (control_tx, control_rx) = tokio::sync::mpsc::channel(32);
    let (state_tx, mut state_rx) = tokio::sync::broadcast::channel(32);

    let mut executor = Executor::new(
        ConfigGraph::default(),
        AudioGraph::default(),
        control_rx,
        state_tx,
    )?;

    // Test-Task: Schickt nach 500ms CreateChannel und nach weiteren 500ms RemoveChannel
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        println!("🚀 Sende CreateChannel für 'test'...");
        if let Err(e) = control_tx
            .send(ControlEvent::CreateChannel {
                name: "test".into(),
            })
            .await
        {
            eprintln!("Fehler beim Senden von CreateChannel: {e:?}");
        }

        tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
        println!("🗑️ Sende RemoveChannel für 'test'...");
        if let Err(e) = control_tx
            .send(ControlEvent::RemoveChannel {
                name: "test".into(),
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
