/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::{
    graph::{
        link::{LinkEndpoint, StereoLink, StereoPortPair},
        node::Node,
    },
    types::{ChannelName, ChannelPosition, NodeId, PortDirection},
};

/// Channel node backed by a `Pipewire` `support.null-audio-sink` adapter.
#[derive(Debug, Clone, Default)]
pub struct Channel {
    pub name: ChannelName,
    pub node: Node,
    /// `PipeLink` `JACK` input port IDs (`in_{name}_FL` / `FR`).
    pub jack_ports: StereoPortPair,
    /// Stereo links from this channel's output ports to `jack_ports`.
    pub links: StereoLink,
    /// Whether `CreateLink` has already been dispatched for this channel.
    pub links_requested: bool,
    /// Whether `ChannelAdded` has already been emitted to the UI/control layer.
    pub added_emitted: bool,
}

impl Channel {
    /// Creates a new `Channel`.
    pub fn new(name: impl Into<ChannelName>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Returns whether this channel's own `PipeWire` node and stereo ports are registered.
    #[must_use]
    pub fn are_pw_ports_ready(&self) -> bool {
        use ChannelPosition::{FrontLeft, FrontRight};
        use PortDirection::{Input, Output};

        if !self.node.is_ready() {
            return false;
        }

        self.node.has_port(Input, FrontLeft)
            && self.node.has_port(Input, FrontRight)
            && self.node.has_port(Output, FrontLeft)
            && self.node.has_port(Output, FrontRight)
    }

    /// Returns whether this channel node, its `JACK` ports, and its `PipeWire` links are ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.are_pw_ports_ready() && self.jack_ports.is_complete() && self.links.is_ready()
    }

    /// Builds the stereo [`StereoLink`] spec (`channel -> JACK`) when all endpoints exist and
    /// links have not yet been requested.
    pub fn prepare_links(&mut self, jack_node_id: NodeId) -> Option<StereoLink> {
        if self.links_requested || !self.are_pw_ports_ready() {
            return None;
        }
        let ch_node_id = self.node.id?;
        let jack_left = self.jack_ports.fl?;
        let jack_right = self.jack_ports.fr?;
        let out_left = self
            .node
            .port_id(PortDirection::Output, ChannelPosition::FrontLeft)?;
        let out_right = self
            .node
            .port_id(PortDirection::Output, ChannelPosition::FrontRight)?;

        let stereo = StereoLink::new(
            LinkEndpoint {
                node_id: ch_node_id,
                port_id: out_left,
            },
            LinkEndpoint {
                node_id: jack_node_id,
                port_id: jack_left,
            },
            LinkEndpoint {
                node_id: ch_node_id,
                port_id: out_right,
            },
            LinkEndpoint {
                node_id: jack_node_id,
                port_id: jack_right,
            },
        );
        self.links = stereo;
        self.links_requested = true;
        Some(stereo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LinkId, NodeName, NodeSerial, PortId};

    fn setup_pw_ports(ch: &mut Channel, node_id: NodeId) {
        ch.node.id = Some(node_id);
        ch.node.name = NodeName::from("pipelink.channel.Game");
        ch.node.serial = Some(NodeSerial(42));
        ch.node.on_port_appeared(
            PortId(1),
            node_id,
            PortDirection::Input,
            ChannelPosition::FrontLeft,
        );
        ch.node.on_port_appeared(
            PortId(2),
            node_id,
            PortDirection::Input,
            ChannelPosition::FrontRight,
        );
        ch.node.on_port_appeared(
            PortId(3),
            node_id,
            PortDirection::Output,
            ChannelPosition::FrontLeft,
        );
        ch.node.on_port_appeared(
            PortId(4),
            node_id,
            PortDirection::Output,
            ChannelPosition::FrontRight,
        );
    }

    #[test]
    fn test_channel_prepare_links_and_readiness() {
        let mut ch = Channel::new("Game");
        let jack_node = NodeId(99);

        setup_pw_ports(&mut ch, NodeId(10));
        assert!(ch.are_pw_ports_ready());
        assert!(!ch.is_ready());
        assert!(ch.prepare_links(jack_node).is_none());

        ch.jack_ports.set_port(ChannelPosition::FrontLeft, PortId(50));
        ch.jack_ports
            .set_port(ChannelPosition::FrontRight, PortId(51));

        let spec = ch.prepare_links(jack_node);
        assert!(spec.is_some());
        assert!(ch.prepare_links(jack_node).is_none());
        assert!(!ch.is_ready());

        assert!(ch.links.match_appeared(
            LinkId(200),
            LinkEndpoint {
                node_id: NodeId(10),
                port_id: PortId(3),
            },
            LinkEndpoint {
                node_id: jack_node,
                port_id: PortId(50),
            },
        ));
        assert!(!ch.is_ready());

        assert!(ch.links.match_appeared(
            LinkId(201),
            LinkEndpoint {
                node_id: NodeId(10),
                port_id: PortId(4),
            },
            LinkEndpoint {
                node_id: jack_node,
                port_id: PortId(51),
            },
        ));
        assert!(ch.is_ready());
    }
}
