/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Handling of the events the `PipeWire` background loop reports back.

use pipelink_common::{
    events::StateChangeEvent,
    graph::{app_node::AppNode, node::Node},
    types::{
        ChannelName, ChannelPosition, GlobalId, MixName, NodeId, NodeName, NodeSerial, Pid,
        PortDirection, PortId,
    },
};
use pipelink_resolver::is_process_alive;
use tracing::{info, warn};

use crate::executor::Executor;

impl Executor {
    /// Registers a newly appeared node on the channel or mix it belongs to.
    pub(super) fn on_node_appeared(
        &mut self,
        id: NodeId,
        name: &NodeName,
        serial: Option<NodeSerial>,
    ) {
        let channel_name = name.strip_prefix("pipelink.channel.");
        let mix_part = name.strip_prefix("pipelink.mix.");

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

        if let Some(mix_part) = mix_part {
            let (mix_name, is_source) = match mix_part.strip_suffix(".source") {
                Some(base) => (base, true),
                None => (mix_part, false),
            };

            if let Some(m) = self.graph.mixes.iter_mut().find(|m| m.name == mix_name) {
                let node = if is_source {
                    &mut m.source
                } else {
                    &mut m.sink
                };
                node.name = name.clone();
                node.id = Some(id);
                node.serial = serial;
                info!(
                    "Mix '{}' {} has the id {id}",
                    m.name,
                    if is_source { "source" } else { "sink" }
                );
            }
        }
    }

    /// Registers a newly appeared port on the channel or mix it belongs to.
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

        if let Some(m) = self
            .graph
            .mixes
            .iter_mut()
            .find(|m| m.sink.id == Some(node_id))
            && m.sink.on_port_appeared(id, node_id, direction, position)
            && m.is_ready()
        {
            info!("Mix '{}' is ready", m.name);
        } else if let Some(m) = self
            .graph
            .mixes
            .iter_mut()
            .find(|m| m.source.id == Some(node_id))
            && m.source.on_port_appeared(id, node_id, direction, position)
            && m.is_ready()
        {
            info!("Mix '{}' is ready", m.name);
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

        self.graph
            .mixes
            .retain(|m| m.source.id != Some(id) && m.sink.id != Some(id));
        self.graph.channels.retain(|c| c.node.id != Some(id));

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

    /// Drops a channel that `PipeWire` refused to create.
    ///
    /// The channel was added to the graph optimistically, so it has to be
    /// taken out again once the creation fails.
    pub(super) fn on_channel_creation_failed(&mut self, name: ChannelName, reason: &str) {
        self.graph.channels.retain(|c| c.name != name);

        warn!("Channel '{name}' could not be created: {reason}");

        let _ = self
            .state_tx
            .send(StateChangeEvent::ChannelRemoved { channel: name });
    }

    /// Drops a mix that `PipeWire` refused to create.
    ///
    /// The mix was added to the graph optimistically, so it has to be taken
    /// out again once the creation fails.
    pub(super) fn on_mix_creation_failed(&mut self, name: MixName, reason: &str) {
        self.graph.mixes.retain(|m| m.name != name);

        warn!("Mix '{name}' could not be created: {reason}");

        let _ = self
            .state_tx
            .send(StateChangeEvent::MixRemoved { mix: name });
    }
}
