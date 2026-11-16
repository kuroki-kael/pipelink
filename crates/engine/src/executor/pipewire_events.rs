/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Handling of the events the `PipeWire` background loop reports back.

mod links;
mod removals;

use pipelink_common::{
    graph::{app_node::AppNode, node::Node, port::MixerPort},
    types::{
        ChannelPosition, MediaClass, NodeId, NodeName, NodeSerial, Pid, PortDirection, PortId,
    },
};
use tracing::info;

use crate::executor::Executor;

impl Executor {
    /// Registers a newly appeared node on the channel, mix, output sink, or `JACK`
    /// client it belongs to.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn on_node_appeared(
        &mut self,
        id: NodeId,
        name: &NodeName,
        media_class: &MediaClass,
        serial: Option<NodeSerial>,
        description: Option<String>,
        nick: Option<String>,
        is_hardware: bool,
    ) {
        if name == self.jack.client_name() {
            self.graph.jack_node_id = Some(id);
            info!("JACK client '{name}' has the id {id}");
            self.try_link_pending(id);
            return;
        }

        if let Some(channel_name) = name.strip_prefix("pipelink.channel.") {
            let synced_channel = if let Some(ch) = self
                .graph
                .channels_mut()
                .iter_mut()
                .find(|c| c.name == channel_name)
            {
                ch.node.name = name.clone();
                ch.node.id = Some(id);
                ch.node.serial = serial;
                info!("Channel '{}' has the id {id}", ch.name);
                Some(ch.name.clone())
            } else {
                None
            };
            if let Some(ch_name) = synced_channel {
                self.sync_channel_app_routes(&ch_name);
            }
        } else if let Some(mix_part) = name.strip_prefix("pipelink.mix.") {
            let (mix_name, is_source) = match mix_part.strip_suffix(".source") {
                Some(base) => (base, true),
                None => (mix_part, false),
            };
            if let Some(m) = self
                .graph
                .mixes_mut()
                .iter_mut()
                .find(|m| m.name == mix_name)
            {
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
            if is_source {
                self.sync_active_mix_output();
            }
        } else if !name.starts_with("pipelink.") && media_class.is_audio_sink() {
            self.on_output_node_appeared(id, name, serial, description, nick, is_hardware);
        }
    }

    /// Registers a newly appeared port on the channel, mix, or `PipeLink` `JACK` node,
    /// and requests stereo `PipeWire` links once both sides' ports are known.
    pub(super) fn on_port_appeared(
        &mut self,
        id: PortId,
        node_id: NodeId,
        name: Option<&str>,
        direction: PortDirection,
        position: ChannelPosition,
    ) {
        if self.graph.jack_node_id == Some(node_id) {
            if let Some(port_name) = name {
                self.on_jack_port_appeared(id, port_name);
            }
            return;
        }

        let jack_node_id = self.graph.jack_node_id;

        if let Some(ch) = self
            .graph
            .channels_mut()
            .iter_mut()
            .find(|c| c.node.id == Some(node_id))
        {
            if ch.node.on_port_appeared(id, node_id, direction, position)
                && let Some(jack_id) = jack_node_id
                && let Some(stereo_link) = ch.prepare_links(jack_id)
            {
                self.dispatch_create_link(stereo_link);
            }
            return;
        }

        if let Some(m) = self
            .graph
            .mixes_mut()
            .iter_mut()
            .find(|m| m.sink.id == Some(node_id) || m.source.id == Some(node_id))
        {
            let sink_updated = m.sink.on_port_appeared(id, node_id, direction, position);
            let source_updated = m.source.on_port_appeared(id, node_id, direction, position);
            if (sink_updated || source_updated)
                && let Some(jack_id) = jack_node_id
                && let Some(stereo_link) = m.prepare_links(jack_id)
            {
                self.dispatch_create_link(stereo_link);
            }
            if source_updated {
                self.sync_active_mix_output();
            }
            return;
        }

        if let Some(out) = self
            .graph
            .outputs
            .iter_mut()
            .find(|o| o.node.id == Some(node_id))
            && out.node.on_port_appeared(id, node_id, direction, position)
        {
            self.sync_active_mix_output();
        }
    }

    /// Matches a `PipeWire` port belonging to the `PipeLink` `JACK` client by `port.name`
    /// onto its owning [`Channel`](pipelink_common::graph::channel::Channel) or
    /// [`Mix`](pipelink_common::graph::mix::Mix).
    fn on_jack_port_appeared(&mut self, port_id: PortId, port_name: &str) {
        let Some(jack_id) = self.graph.jack_node_id else {
            return;
        };

        match MixerPort::from_name(port_name) {
            Some(MixerPort::ChannelInput {
                channel,
                position: pos,
            }) => {
                if let Some(ch) = self
                    .graph
                    .channels_mut()
                    .iter_mut()
                    .find(|c| c.name == channel)
                    && ch.jack_ports.set_port(pos, port_id)
                    && let Some(stereo_link) = ch.prepare_links(jack_id)
                {
                    self.dispatch_create_link(stereo_link);
                }
            },
            Some(MixerPort::MixOutput { mix, position: pos }) => {
                if let Some(m) = self.graph.mixes_mut().iter_mut().find(|m| m.name == mix)
                    && m.jack_ports.set_port(pos, port_id)
                    && let Some(stereo_link) = m.prepare_links(jack_id)
                {
                    self.dispatch_create_link(stereo_link);
                }
            },
            None => {},
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

            let app_idx = if let Some(idx) = self
                .graph
                .apps
                .iter()
                .position(|existing| existing.has_node(id))
                .or_else(|| {
                    self.graph
                        .apps
                        .iter()
                        .position(|existing| existing.matches_app(&app))
                }) {
                self.graph.apps[idx].app = app;
                self.graph.apps[idx].upsert_node(node);
                idx
            } else {
                self.graph.apps.push(AppNode::new(app, node));
                self.graph.apps.len() - 1
            };

            self.apply_configured_app_route(app_idx, id);
        }
    }
}
