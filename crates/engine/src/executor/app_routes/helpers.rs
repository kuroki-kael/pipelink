/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Pure helper functions for dispatching and clearing `target.object` app routes.

use pipelink_common::{
    graph::app_node::AppNode,
    types::{ChannelName, NodeId, NodeSerial},
};
use pipelink_pw::Command as PipewireCommand;
use pipewire::channel::Sender as ChannelSender;
use tracing::warn;

use crate::config::topology::ConfigGraph;

pub(super) fn find_channel_for_app<'a>(
    config: &'a ConfigGraph,
    app_node: &AppNode,
) -> Option<&'a ChannelName> {
    config
        .find_app_channel(app_node.app.display_name.as_ref())
        .or_else(|| config.find_app_channel(&app_node.app.app_name))
        .or_else(|| {
            app_node
                .app
                .app_id
                .as_deref()
                .and_then(|id| config.find_app_channel(id))
        })
}

pub(super) fn collect_set_routes(
    app_node: &AppNode,
    target_serial: NodeSerial,
) -> Vec<PipewireCommand> {
    let label = app_node.app.display_name.as_ref();
    app_node
        .nodes
        .iter()
        .filter_map(|node| {
            node.id.map(|node_id| PipewireCommand::SetAppRoute {
                node_id,
                target_serial,
                label: label.to_string(),
            })
        })
        .collect()
}

pub(super) fn collect_clear_routes(app_node: &mut AppNode) -> Vec<PipewireCommand> {
    app_node.channel = None;
    app_node.target_serial = None;
    app_node
        .nodes
        .iter()
        .filter_map(|node| {
            node.id
                .map(|node_id| PipewireCommand::ClearNodeRoute { node_id })
        })
        .collect()
}

pub(super) fn dispatch_all_nodes(
    pw_tx: &ChannelSender<PipewireCommand>,
    app_node: &AppNode,
    target_serial: NodeSerial,
) {
    for cmd in collect_set_routes(app_node, target_serial) {
        if let Err(err) = pw_tx.send(cmd) {
            warn!("Failed to send SetAppRoute: {err:?}");
        }
    }
}

pub(super) fn clear_app_node_route(pw_tx: &ChannelSender<PipewireCommand>, app_node: &mut AppNode) {
    for cmd in collect_clear_routes(app_node) {
        if let Err(err) = pw_tx.send(cmd) {
            warn!("Failed to send ClearNodeRoute: {err:?}");
        }
    }
}

pub(super) fn send_set_app_route(
    pw_tx: &ChannelSender<PipewireCommand>,
    node_id: NodeId,
    target_serial: NodeSerial,
    label: &str,
) {
    if let Err(err) = pw_tx.send(PipewireCommand::SetAppRoute {
        node_id,
        target_serial,
        label: label.to_string(),
    }) {
        warn!("Failed to send SetAppRoute for '{label}' (node {node_id}): {err:?}");
    }
}
