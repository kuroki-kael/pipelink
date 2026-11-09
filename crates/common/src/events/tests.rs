/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Tests for the domain events and their wire format.

#![allow(clippy::unwrap_used)]

use super::{ControlEvent, ControlReply, StateEvent};
use crate::types::{AppDisplay, ChannelName, MixName};

#[test]
fn test_control_event_json_format() {
    let event = ControlEvent::SetChannelGain {
        channel: ChannelName::from("mic"),
        mix: MixName::from("headphones"),
        db_value: -6.0,
    };

    assert_eq!(
        serde_json::to_string(&event).unwrap(),
        r#"{"type":"set_channel_gain","channel":"mic","mix":"headphones","db_value":-6.0}"#
    );
}

#[test]
fn test_control_event_create_channel_json_format() {
    let event = ControlEvent::CreateChannel {
        name: ChannelName::from("mic"),
    };

    assert_eq!(
        serde_json::to_string(&event).unwrap(),
        r#"{"type":"create_channel","name":"mic"}"#
    );
}

#[test]
fn test_state_change_event_json_format() {
    let event = StateEvent::ChannelAdded {
        channel: ChannelName::from("mic"),
    };

    assert_eq!(
        serde_json::to_string(&event).unwrap(),
        r#"{"type":"channel_added","channel":"mic"}"#
    );
}

#[test]
fn test_events_round_trip() {
    let event = ControlEvent::AddChannelMix {
        channel: ChannelName::from("mic"),
        mix: MixName::from("headphones"),
    };

    let json = serde_json::to_string(&event).unwrap();
    let parsed: ControlEvent = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed, event);
}

#[test]
fn test_control_reply_channels_json_format() {
    let reply = ControlReply::Channels {
        names: vec![ChannelName::from("mic"), ChannelName::from("line")],
    };

    assert_eq!(
        serde_json::to_string(&reply).unwrap(),
        r#"{"type":"channels","names":["mic","line"]}"#
    );
}

#[test]
fn test_control_reply_app_channels_json_format() {
    let reply = ControlReply::AppChannels {
        routes: vec![(AppDisplay::from("Discord"), ChannelName::from("chat"))],
    };

    assert_eq!(
        serde_json::to_string(&reply).unwrap(),
        r#"{"type":"app_channels","routes":[["Discord","chat"]]}"#
    );
}

#[test]
fn test_control_reply_mix_channels_json_format() {
    let reply = ControlReply::MixChannels {
        channels: vec![(
            MixName::from("music"),
            vec![ChannelName::from("mic"), ChannelName::from("line")],
        )],
    };

    assert_eq!(
        serde_json::to_string(&reply).unwrap(),
        r#"{"type":"mix_channels","channels":[["music",["mic","line"]]]}"#
    );
}

#[test]
fn test_control_reply_active_mix_json_format() {
    assert_eq!(
        serde_json::to_string(&ControlReply::ActiveMix { mix: None }).unwrap(),
        r#"{"type":"active_mix","mix":null}"#
    );
    assert_eq!(
        serde_json::to_string(&ControlReply::ActiveMix {
            mix: Some(MixName::from("music")),
        })
        .unwrap(),
        r#"{"type":"active_mix","mix":"music"}"#
    );
}

#[test]
fn test_control_reply_round_trip() {
    let reply = ControlReply::MixChannels {
        channels: vec![(MixName::from("music"), vec![ChannelName::from("mic")])],
    };

    let json = serde_json::to_string(&reply).unwrap();
    let parsed: ControlReply = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed, reply);
}
