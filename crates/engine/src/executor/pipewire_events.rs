/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Handling of the events the `PipeWire` background loop reports back.

use pipelink_common::{
    graph::{app_node::AppNode, node::Node},
    types::{ChannelPosition, GlobalId, NodeId, NodeName, NodeSerial, Pid, PortDirection, PortId},
};
use pipelink_resolver::is_process_alive;
use tracing::info;

use crate::executor::Executor;

impl Executor {
    /// Registers a newly appeared node on the channel it belongs to.
    pub(super) fn on_node_appeared(
        &mut self,
        id: NodeId,
        name: &NodeName,
        serial: Option<NodeSerial>,
    ) {
        let channel_name = name.strip_prefix("pipelink.channel.");

        if let Some(channel_name) = channel_name
            && let Some(ch) = self
                .graph
                .channels
                .iter_mut()
                .find(|c| c.name == channel_name)
        {
            ch.node.name = name.clone();
            ch.node.id = Some(id);
            ch.node.serial = serial;
            info!("Channel '{}' has the id {id}", ch.name);
        }
    }

    /// Registers a newly appeared port on the channel it belongs to.
    pub(super) fn on_port_appeared(
        &mut self,
        id: PortId,
        node_id: NodeId,
        direction: PortDirection,
        position: ChannelPosition,
    ) {
        if let Some(ch) = self
            .graph
            .channels
            .iter_mut()
            .find(|c| c.node.id == Some(node_id))
            && ch.node.on_port_appeared(id, node_id, direction, position)
            && ch.is_ready()
        {
            info!("Channel '{}' is ready", ch.name);
        }
    }

    /// Attributes a stream node to the application that owns it.
    pub(super) fn on_node_info(
        &mut self,
        id: NodeId,
        node_name: Option<NodeName>,
        pid: Option<Pid>,
    ) {
        // Ignore internal PipeLink nodes (e.g. pipelink.channel.* or
        // pipelink.mix.*)
        if let Some(ref name) = node_name
            && name.starts_with("pipelink.")
        {
            return;
        }
        if self.graph.channels.iter().any(|c| c.node.id == Some(id)) {
            return;
        }

        if let Some(pid) = pid
            && let Some(app) = self.resolver.resolve(pid)
        {
            info!(
                "Resolved stream node {id} to app '{}' ({})",
                app.display_name, app.app_name
            );

            let name = node_name.unwrap_or_else(|| NodeName::from(app.app_name.as_str()));

            let node = Node {
                id: Some(id),
                name,
                ..Default::default()
            };

            if let Some(existing) = self.graph.apps.iter_mut().find(|a| a.node.id == Some(id)) {
                existing.app = app;
                existing.node = node;
            } else {
                self.graph.apps.push(AppNode::new(app, node));
            }
        }
    }

    /// Drops the nodes that vanished from the `PipeWire` registry.
    pub(super) fn on_global_removed(&mut self, id: GlobalId) {
        let id = NodeId::from(id);

        if let Some(pos) = self.graph.apps.iter().position(|a| a.node.id == Some(id)) {
            let removed = self.graph.apps.remove(pos);
            let pid = removed.app.pid;
            let other_active = self.graph.apps.iter().any(|a| a.app.pid == pid);
            if !other_active && !is_process_alive(pid) {
                self.resolver.invalidate(pid);
                info!("Process {pid} exited, evicted from resolver cache");
            }
        }
    }
}
