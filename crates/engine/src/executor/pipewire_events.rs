/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Handling of the events the `PipeWire` background loop reports back.

use pipelink_common::{
    events::StateEvent,
    graph::{
        app_node::AppNode,
        link::{LinkEndpoint, StereoLink},
        node::Node,
        port::MixerPort,
    },
    types::{
        ChannelName, ChannelPosition, GlobalId, LinkId, MediaClass, MixName, NodeId, NodeName,
        NodeSerial, Pid, PortDirection, PortId,
    },
};
use pipelink_pw::Command as PipewireCommand;
use pipelink_resolver::is_process_alive;
use tracing::{info, warn};

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

    /// Records a newly appeared `PipeWire` link on its channel or mix and emits
    /// `ChannelAdded` / `MixAdded` once both stereo links are confirmed.
    pub(super) fn on_link_appeared(
        &mut self,
        id: LinkId,
        out_node: NodeId,
        in_node: NodeId,
        out_port: PortId,
        in_port: PortId,
    ) {
        let out_ep = LinkEndpoint {
            node_id: out_node,
            port_id: out_port,
        };
        let in_ep = LinkEndpoint {
            node_id: in_node,
            port_id: in_port,
        };

        for ch in self.graph.channels_mut() {
            if ch.links.match_appeared(id, out_ep, in_ep) {
                if !ch.added_emitted && ch.is_ready() {
                    ch.added_emitted = true;
                    info!("Channel '{}' is ready", ch.name);
                    // The graph entry is optimistic, the ChannelAdded event is sent once
                    // PipeWire has registered all ports and stereo links to JACK.
                    let _ = self.state_tx.send(StateEvent::ChannelAdded {
                        channel: ch.name.clone(),
                    });
                }
                return;
            }
        }

        for m in self.graph.mixes_mut() {
            if m.links.match_appeared(id, out_ep, in_ep) {
                if !m.added_emitted && m.is_ready() {
                    m.added_emitted = true;
                    info!("Mix '{}' is ready", m.name);
                    // The graph entry is optimistic, the MixAdded event is sent once
                    // PipeWire has registered all ports and stereo links from JACK.
                    let _ = self.state_tx.send(StateEvent::MixAdded {
                        mix: m.name.clone(),
                    });
                }
                return;
            }
        }

        let _ = self
            .graph
            .active_output_links
            .match_appeared(id, out_ep, in_ep);
    }

    /// Clears a disappeared link ID from whichever channel, mix, or output route owned
    /// it.
    pub(super) fn on_link_removed(&mut self, id: LinkId) {
        for ch in self.graph.channels_mut() {
            if ch.links.clear_link_id(id) {
                return;
            }
        }
        for m in self.graph.mixes_mut() {
            if m.links.clear_link_id(id) {
                return;
            }
        }
        if self.graph.active_output_links.clear_link_id(id) {
            self.graph.output_links_requested = false;
        }
    }

    fn try_link_pending(&mut self, jack_node_id: NodeId) {
        let mut pending = Vec::new();
        for ch in self.graph.channels_mut() {
            if let Some(stereo_link) = ch.prepare_links(jack_node_id) {
                pending.push(stereo_link);
            }
        }
        for m in self.graph.mixes_mut() {
            if let Some(stereo_link) = m.prepare_links(jack_node_id) {
                pending.push(stereo_link);
            }
        }
        for stereo_link in pending {
            self.dispatch_create_link(stereo_link);
        }
    }

    pub(super) fn dispatch_create_link(&self, stereo_link: StereoLink) {
        if let Err(err) = self.pw_tx.send(PipewireCommand::CreateLink { stereo_link }) {
            warn!("Failed to send CreateLink command to PipeWire: {err:?}");
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

    /// Drops the nodes that vanished from the `PipeWire` registry.
    ///
    /// The entry is taken out of the graph here and not when the removal was
    /// requested, because only the vanished node confirms the removal.
    pub(super) fn on_global_removed(&mut self, id: GlobalId) {
        let link_id = LinkId::from(id.0);
        let id = NodeId::from(id);

        if self.graph.jack_node_id == Some(id) {
            self.graph.jack_node_id = None;
            return;
        }

        if let Some(Some(channel)) = self
            .graph
            .channels()
            .iter()
            .position(|c| c.node.id == Some(id))
            .map(|pos| self.graph.remove_channel_idx(pos))
        {
            self.clear_channel_app_routes(&channel.name);
            if let Err(err) = self
                .jack
                .remove_channel(&channel.name, self.graph.gains_snapshot())
            {
                warn!("Failed to remove JACK channel '{}': {err}", channel.name);
            }
            let _ = self.state_tx.send(StateEvent::ChannelRemoved {
                channel: channel.name,
            });
        }
        if let Some(Some(mix)) = self
            .graph
            .mixes()
            .iter()
            .position(|m| m.source.id == Some(id) || m.sink.id == Some(id))
            .map(|pos| self.graph.remove_mix_idx(pos))
        {
            if let Err(err) = self.jack.remove_mix(&mix.name, self.graph.gains_snapshot()) {
                warn!("Failed to remove JACK mix '{}': {err}", mix.name);
            }
            let _ = self.state_tx.send(StateEvent::MixRemoved { mix: mix.name });
            self.ensure_active_mix();
        }

        if let Some(pos) = self
            .graph
            .outputs
            .iter()
            .position(|o| o.node.id == Some(id))
        {
            let removed = self.graph.outputs.remove(pos);
            if self.graph.selected_output.as_ref() == Some(&removed.node.name) {
                self.graph.selected_output = None;
                self.ensure_selected_output();
                self.sync_active_mix_output();
            }
        }

        let mut evicted_pids = Vec::new();
        for app_node in &mut self.graph.apps {
            if app_node.remove_node(id) && app_node.nodes.is_empty() {
                evicted_pids.push(app_node.app.pid);
            }
        }
        self.graph.apps.retain(|a| !a.nodes.is_empty());

        for pid in evicted_pids {
            let other_active = self.graph.apps.iter().any(|a| a.app.pid == pid);
            if !other_active && !is_process_alive(pid) {
                self.resolver.invalidate(pid);
                info!("Process {pid} exited, evicted from resolver cache");
            }
        }

        self.on_link_removed(link_id);
    }

    /// Reports a node that could not be destroyed.
    ///
    /// The entry stays in the graph, because the node is still there. The
    /// client learns that the removal did not happen.
    pub(super) fn on_global_removal_failed(&mut self, id: GlobalId, reason: &str) {
        let id = NodeId::from(id);

        if let Some(channel) = self.graph.channels().iter().find(|c| c.node.id == Some(id)) {
            warn!("Channel '{}' could not be removed: {reason}", channel.name);
            let _ = self.state_tx.send(StateEvent::ChannelRemovalFailed {
                channel: channel.name.clone(),
                error: reason.to_string(),
            });
        }

        if let Some(mix) = self
            .graph
            .mixes()
            .iter()
            .find(|m| m.source.id == Some(id) || m.sink.id == Some(id))
        {
            warn!("Mix '{}' could not be removed: {reason}", mix.name);
            let _ = self.state_tx.send(StateEvent::MixRemovalFailed {
                mix: mix.name.clone(),
                error: reason.to_string(),
            });
        }
    }

    /// Drops a channel that `PipeWire` refused to create.
    ///
    /// The channel was added to the graph optimistically, so it has to be
    /// taken out again once the creation fails.
    pub(super) fn on_channel_creation_failed(&mut self, name: ChannelName, reason: &str) {
        self.graph.remove_channel(&name);
        self.clear_channel_app_routes(&name);
        if let Err(err) = self.jack.remove_channel(&name, self.graph.gains_snapshot()) {
            warn!("Failed to remove JACK channel '{name}': {err}");
        }

        warn!("Channel '{name}' could not be created: {reason}");

        let _ = self.state_tx.send(StateEvent::ChannelCreationFailed {
            channel: name,
            error: reason.to_string(),
        });
    }

    /// Reports a mix whose loopback could not be removed.
    pub(super) fn on_mix_removal_failed(&mut self, name: MixName, reason: &str) {
        warn!("Mix '{name}' could not be removed: {reason}");

        let _ = self.state_tx.send(StateEvent::MixRemovalFailed {
            mix: name,
            error: reason.to_string(),
        });
    }

    /// Drops a mix that `PipeWire` refused to create.
    ///
    /// The mix was added to the graph optimistically, so it has to be taken
    /// out again once the creation fails.
    pub(super) fn on_mix_creation_failed(&mut self, name: MixName, reason: &str) {
        self.graph.remove_mix(&name);
        if let Err(err) = self.jack.remove_mix(&name, self.graph.gains_snapshot()) {
            warn!("Failed to remove JACK mix '{name}': {err}");
        }
        self.ensure_active_mix();

        warn!("Mix '{name}' could not be created: {reason}");

        let _ = self.state_tx.send(StateEvent::MixCreationFailed {
            mix: name,
            error: reason.to_string(),
        });
    }
}
