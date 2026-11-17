/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Live daemon event subscription feeding `StateEvent` updates into the `iced` loop.

use std::time::Duration;

use iced::{
    Subscription,
    futures::{SinkExt, Stream, channel::mpsc},
    stream,
};
use pipelink_ipc::{ControlClient, UnixSocketClient};
use tokio::net::UnixListener;

use crate::{instance, state::Message};

const RECONNECT_DELAY: Duration = Duration::from_secs(2);

/// Subscribes to daemon state broadcasts, automatically reconnecting if the daemon
/// restarts.
pub(crate) fn daemon_events() -> Subscription<Message> {
    Subscription::run(event_worker)
}

/// Listens for single-instance activation pings so secondary launches focus the
/// existing GUI window.
pub(crate) fn activation_events() -> Subscription<Message> {
    Subscription::run(activation_worker)
}

fn event_worker() -> impl Stream<Item = Message> {
    stream::channel(64, async |mut output: mpsc::Sender<Message>| {
        let client = UnixSocketClient::new();
        loop {
            match client.subscribe().await {
                Ok(mut sub) => {
                    let _ = output.send(Message::DaemonConnected).await;
                    loop {
                        if let Ok(Some(event)) = sub.next_event().await {
                            let _ = output.send(Message::StateEventReceived(event)).await;
                        } else {
                            let _ = output.send(Message::DaemonDisconnected).await;
                            break;
                        }
                    }
                },
                Err(_) => {
                    let _ = output.send(Message::DaemonDisconnected).await;
                },
            }

            tokio::time::sleep(RECONNECT_DELAY).await;
        }
    })
}

fn activation_worker() -> impl Stream<Item = Message> {
    stream::channel(16, async |mut output: mpsc::Sender<Message>| {
        let Some(std_listener) = instance::take_activation_listener() else {
            return;
        };
        if std_listener.set_nonblocking(true).is_err() {
            return;
        }
        let Ok(listener) = UnixListener::from_std(std_listener) else {
            return;
        };
        while let Ok((stream, _)) = listener.accept().await {
            drop(stream);
            let _ = output.send(Message::FocusWindow).await;
        }
    })
}
