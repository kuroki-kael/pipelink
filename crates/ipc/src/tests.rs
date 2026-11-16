/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Integration tests for Unix socket query, mutation, and state subscription.

#![allow(clippy::unwrap_used)]

use std::{
    env, fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use pipelink_common::{
    events::{ControlEvent, ControlReply, GraphSnapshot, StateEvent},
    types::{ChannelName, MixName},
};
use tokio::sync::{broadcast, mpsc};

use crate::{
    ControlClient, UnixSocketClient,
    server::{Envelope, handle_client, start_server_at},
};

struct TempSocketDir(PathBuf);

impl TempSocketDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!("{prefix}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn socket(&self) -> PathBuf {
        self.0.join("pipelink.sock")
    }
}

impl Drop for TempSocketDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn test_unix_socket_query_mutate_and_subscribe_round_trip() {
    let dir = TempSocketDir::new("pipelink-ipc-test");
    let sock_path = dir.socket();

    let (listener, guard) = start_server_at(&sock_path).await.unwrap();
    let (control_tx, mut control_rx) = mpsc::channel::<Envelope>(16);
    let (state_tx, _) = broadcast::channel::<StateEvent>(16);

    let accept_state_tx = state_tx.clone();
    let server_task = tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let tx = control_tx.clone();
            let rx = accept_state_tx.subscribe();
            tokio::spawn(handle_client(stream, tx, rx));
        }
    });

    let exec_state_tx = state_tx.clone();
    let executor_task = tokio::spawn(async move {
        while let Some(env) = control_rx.recv().await {
            match env.event {
                ControlEvent::GetGraphSnapshot => {
                    if let Some(reply) = env.reply {
                        let _ = reply.send(ControlReply::GraphSnapshot(GraphSnapshot {
                            channels: vec![ChannelName::from("Music")],
                            mixes: vec![MixName::from("Personal")],
                            ..Default::default()
                        }));
                    }
                },
                ControlEvent::CreateChannel { name } => {
                    drop(env.reply);
                    let _ = exec_state_tx.send(StateEvent::ChannelAdded { channel: name });
                },
                _ => {},
            }
        }
    });

    let client = UnixSocketClient::with_path(&sock_path);

    let mut sub = client.subscribe().await.unwrap();
    let reply = client.query(ControlEvent::GetGraphSnapshot).await.unwrap();
    match reply {
        ControlReply::GraphSnapshot(snapshot) => {
            assert_eq!(snapshot.channels, vec![ChannelName::from("Music")]);
        },
        other => panic!("unexpected reply: {other:?}"),
    }

    let state = client
        .mutate(
            ControlEvent::CreateChannel {
                name: ChannelName::from("Games"),
            },
            |ev| matches!(ev, StateEvent::ChannelAdded { channel } if channel == "Games"),
        )
        .await
        .unwrap();
    assert_eq!(
        state,
        StateEvent::ChannelAdded {
            channel: ChannelName::from("Games")
        }
    );

    let live_event = sub.next_event().await.unwrap();
    assert_eq!(
        live_event,
        Some(StateEvent::ChannelAdded {
            channel: ChannelName::from("Games")
        })
    );

    drop(guard);
    server_task.abort();
    executor_task.abort();
}
