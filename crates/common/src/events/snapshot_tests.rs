/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Round-trip serialization tests for snapshot, installed app, and subscription events.

#![allow(clippy::unwrap_used)]

use super::{ControlEvent, ControlReply, GraphSnapshot, InstalledAppInfo, MixRouteInfo};
use crate::types::{AppId, ChannelName, MixName};

#[test]
fn test_snapshot_and_subscription_events_round_trip() {
    for ev in [
        ControlEvent::GetGraphSnapshot,
        ControlEvent::ListInstalledApps,
        ControlEvent::SubscribeState,
    ] {
        let json = serde_json::to_string(&ev).unwrap();
        assert_eq!(serde_json::from_str::<ControlEvent>(&json).unwrap(), ev);
    }

    let snapshot = ControlReply::GraphSnapshot(GraphSnapshot {
        channels: vec![ChannelName::from("Music")],
        mixes: vec![MixName::from("Personal")],
        active_mix: Some(MixName::from("Personal")),
        routes: vec![MixRouteInfo {
            channel: ChannelName::from("Music"),
            mix: MixName::from("Personal"),
            active: true,
            gain_percent: 75.0,
            muted: false,
        }],
        ..Default::default()
    });
    let snap_json = serde_json::to_string(&snapshot).unwrap();
    assert_eq!(
        serde_json::from_str::<ControlReply>(&snap_json).unwrap(),
        snapshot
    );

    let apps_reply = ControlReply::InstalledApps {
        apps: vec![InstalledAppInfo {
            id: AppId::from("firefox"),
            name: "Firefox".to_string(),
            icon: Some("firefox".to_string()),
            exec_bin: Some("firefox".to_string()),
        }],
    };
    let apps_json = serde_json::to_string(&apps_reply).unwrap();
    assert_eq!(
        serde_json::from_str::<ControlReply>(&apps_json).unwrap(),
        apps_reply
    );
}
