/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Tests for the domain events and their wire format.

#![allow(clippy::unwrap_used)]

use super::{ActiveAppInfo, ControlEvent, ControlReply, StateEvent};
use crate::types::{AppDisplay, ChannelName, MixName};

#[test]
fn test_control_event_json_format() {
    let event = ControlEvent::SetChannelGain {
        channel: ChannelName::from("mic"),
        mix: MixName::from("headphones"),
        percent: 50.0,
    };

    assert_eq!(
        serde_json::to_string(&event).unwrap(),
        r#"{"type":"set_channel_gain","channel":"mic","mix":"headphones","percent":50.0}"#
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
fn test_control_event_query_json_format() {
    assert_eq!(
        serde_json::to_string(&ControlEvent::ListChannels).unwrap(),
        r#"{"type":"list_channels"}"#
    );
    assert_eq!(
        serde_json::to_string(&ControlEvent::GetActiveMix).unwrap(),
        r#"{"type":"get_active_mix"}"#
    );
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
fn test_state_event_removal_failed_json_format() {
    let event = StateEvent::ChannelRemovalFailed {
        channel: ChannelName::from("mic"),
        error: "there is no channel with this name".to_string(),
    };

    assert_eq!(
        serde_json::to_string(&event).unwrap(),
        r#"{"type":"channel_removal_failed","channel":"mic","error":"there is no channel with this name"}"#
    );

    let event = StateEvent::MixRemovalFailed {
        mix: MixName::from("music"),
        error: "there is no mix with this name".to_string(),
    };

    assert_eq!(
        serde_json::to_string(&event).unwrap(),
        r#"{"type":"mix_removal_failed","mix":"music","error":"there is no mix with this name"}"#
    );

    let event = StateEvent::RouteOperationFailed {
        channel: ChannelName::from("mic"),
        mix: MixName::from("music"),
        error: "channel or mix not found".to_string(),
    };

    assert_eq!(
        serde_json::to_string(&event).unwrap(),
        r#"{"type":"route_operation_failed","channel":"mic","mix":"music","error":"channel or mix not found"}"#
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

#[test]
fn test_app_routing_events_and_replies_round_trip() {
    let route = ControlEvent::RouteApp {
        app: AppDisplay::from("Discord"),
        channel: ChannelName::from("Chat"),
    };
    let json = serde_json::to_string(&route).unwrap();
    assert_eq!(serde_json::from_str::<ControlEvent>(&json).unwrap(), route);

    let apps_reply = ControlReply::Apps {
        apps: vec![ActiveAppInfo {
            display_name: AppDisplay::from("Discord"),
            app_name: "discord".to_string(),
            channel: Some(ChannelName::from("Chat")),
            stream_count: 2,
        }],
    };
    let reply_json = serde_json::to_string(&apps_reply).unwrap();
    assert_eq!(
        serde_json::from_str::<ControlReply>(&reply_json).unwrap(),
        apps_reply
    );

    let state = StateEvent::AppRouted {
        app: AppDisplay::from("Discord"),
        channel: ChannelName::from("Chat"),
    };
    let state_json = serde_json::to_string(&state).unwrap();
    assert_eq!(
        serde_json::from_str::<StateEvent>(&state_json).unwrap(),
        state
    );
}
