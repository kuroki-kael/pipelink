/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Tests for the shared types.

#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn test_channel_position_from_pw_str() {
    assert_eq!(
        ChannelPosition::from_pw_str("FL"),
        ChannelPosition::FrontLeft
    );
    assert_eq!(
        ChannelPosition::from_pw_str("FR"),
        ChannelPosition::FrontRight
    );
    assert_eq!(
        ChannelPosition::from_pw_str("FC"),
        ChannelPosition::FrontCenter
    );
    assert_eq!(
        ChannelPosition::from_pw_str("RL"),
        ChannelPosition::RearLeft
    );
    assert_eq!(
        ChannelPosition::from_pw_str("RR"),
        ChannelPosition::RearRight
    );
    assert_eq!(ChannelPosition::from_pw_str("LFE"), ChannelPosition::Lfe);
    assert_eq!(ChannelPosition::from_pw_str("MONO"), ChannelPosition::Mono);
    assert_eq!(
        ChannelPosition::from_pw_str("AUX0"),
        ChannelPosition::Aux(0)
    );
    assert_eq!(
        ChannelPosition::from_pw_str("AUX7"),
        ChannelPosition::Aux(7)
    );
}

#[test]
fn test_channel_position_from_pw_str_falls_back_to_other() {
    assert_eq!(ChannelPosition::from_pw_str("AUX"), ChannelPosition::Other);
    assert_eq!(ChannelPosition::from_pw_str("AUXX"), ChannelPosition::Other);
    assert_eq!(
        ChannelPosition::from_pw_str("AUX999"),
        ChannelPosition::Other
    );
    assert_eq!(ChannelPosition::from_pw_str(""), ChannelPosition::Other);
}

#[test]
fn test_channel_position_display() {
    assert_eq!(ChannelPosition::FrontLeft.to_string(), "FL");
    assert_eq!(ChannelPosition::Lfe.to_string(), "LFE");
    assert_eq!(ChannelPosition::Aux(3).to_string(), "AUX3");
    assert_eq!(ChannelPosition::Other.to_string(), "Other");
}

#[test]
fn test_node_name_compares_with_str() {
    let name = NodeName::from("pipelink.channel.test");

    assert_eq!(name, "pipelink.channel.test");
    assert_eq!(name, *"pipelink.channel.test");
    assert_eq!("pipelink.channel.test", name);
    assert_ne!(name, "pipelink.channel.other");
}

#[test]
fn test_app_id_compares_with_str() {
    let app_id = AppId::from("com.discordapp.Discord");

    assert_eq!(app_id, "com.discordapp.Discord");
    assert_eq!("com.discordapp.Discord", app_id);
}

#[test]
fn test_ids_parse_from_str() {
    assert_eq!("42".parse::<NodeId>().unwrap(), NodeId(42));
    assert_eq!("42".parse::<PortId>().unwrap(), PortId(42));
    assert_eq!("42".parse::<LinkId>().unwrap(), LinkId(42));
    assert_eq!("42".parse::<GlobalId>().unwrap(), GlobalId(42));
    assert_eq!("42".parse::<Pid>().unwrap(), Pid(42));
    assert_eq!("42".parse::<NodeSerial>().unwrap(), NodeSerial(42));
    assert!("nope".parse::<NodeId>().is_err());
}

#[test]
fn test_global_id_converts_to_node_id() {
    assert_eq!(NodeId::from(GlobalId(7)), NodeId(7));
}

#[test]
fn test_flat_reexport_matches_submodule_item() {
    // The flat re-export is the public surface; it must denote the very same
    // item as the defining submodule.
    let flat: super::NodeId = super::node::NodeId(7);

    assert_eq!(flat, NodeId(7));
    assert_eq!(super::node::NodeId(7), flat);
}
