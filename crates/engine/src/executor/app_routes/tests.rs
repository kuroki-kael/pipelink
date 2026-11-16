/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::sync::Arc;

use pipelink_common::{
    app::{App, WrapperKind},
    graph::{app_node::AppNode, node::Node},
    types::{AppDisplay, ChannelName, NodeId, NodeName, NodeSerial, Pid},
};
use pipelink_pw::Command as PipewireCommand;

use super::helpers::{collect_clear_routes, collect_set_routes, find_channel_for_app};
use crate::config::topology::ConfigGraph;

fn test_app(display: &str, slug: &str) -> Arc<App> {
    Arc::new(App {
        pid: Pid(42),
        actual_pid: Pid(42),
        app_name: slug.to_string(),
        display_name: AppDisplay::from(display),
        wrapper: WrapperKind::None,
        binary_path: None,
        app_id: None,
        icon: None,
        desktop_file: None,
        categories: Vec::new(),
        generic_name: None,
        comment: None,
        cmdline: Vec::new(),
        cwd: None,
    })
}

fn test_node(id: u32) -> Node {
    Node {
        id: Some(NodeId(id)),
        name: NodeName::from("stream"),
        ..Default::default()
    }
}

#[test]
fn test_dispatch_and_clear_multi_node_app_route() {
    let chat = ChannelName::from("Chat");

    let mut app_node = AppNode::new(test_app("Discord", "discord"), test_node(10));
    app_node.upsert_node(test_node(11));
    app_node.channel = Some(chat);
    app_node.target_serial = Some(NodeSerial(999));

    let dispatched: Vec<_> = collect_set_routes(&app_node, NodeSerial(999))
        .into_iter()
        .filter_map(|cmd| match cmd {
            PipewireCommand::SetAppRoute {
                node_id,
                target_serial,
                ..
            } => Some((node_id, target_serial)),
            _ => None,
        })
        .collect();
    assert_eq!(
        dispatched,
        vec![(NodeId(10), NodeSerial(999)), (NodeId(11), NodeSerial(999))]
    );

    let cleared: Vec<_> = collect_clear_routes(&mut app_node)
        .into_iter()
        .filter_map(|cmd| match cmd {
            PipewireCommand::ClearNodeRoute { node_id } => Some(node_id),
            _ => None,
        })
        .collect();
    assert_eq!(app_node.channel, None);
    assert_eq!(app_node.target_serial, None);
    assert_eq!(cleared, vec![NodeId(10), NodeId(11)]);
}

#[test]
fn test_find_channel_for_app_falls_back_to_slug() {
    let mut cfg = ConfigGraph::new();
    let music = ChannelName::from("Music");
    cfg.ensure_channel(&music);
    assert!(cfg.set_app_route(AppDisplay::from("Spotify"), &music));

    let app_node = AppNode::new(test_app("Spotify", "spotify"), test_node(1));
    assert_eq!(find_channel_for_app(&cfg, &app_node), Some(&music));

    let by_slug = AppNode::new(test_app("Spotify Free", "spotify"), test_node(2));
    cfg.remove_app_route(&AppDisplay::from("Spotify"));
    assert!(cfg.set_app_route(AppDisplay::from("spotify"), &music));
    assert_eq!(find_channel_for_app(&cfg, &by_slug), Some(&music));
}
