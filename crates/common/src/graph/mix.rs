/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::{
    graph::{
        link::{LinkEndpoint, StereoLink},
        node::Node,
        port::StereoPortPair,
    },
    types::{ChannelPosition, MixName, NodeId, PortDirection},
};

/// Mix node backed by a `PipeWire` `libpipewire-module-loopback`.
///
/// Contains a sink node (`pipelink.mix.{name}`) and a source node
/// (`pipelink.mix.{name}.source`)
#[derive(Clone, Debug, Default)]
pub struct Mix {
    pub name: MixName,
    pub sink: Node,
    pub source: Node,
    /// `PipeLink` `JACK` output port IDs (`out_{name}_FL` / `FR`).
    pub jack_ports: StereoPortPair,
    /// Stereo links from `jack_ports` to this mix's sink input ports.
    pub links: StereoLink,
    /// Whether `CreateLink` has already been dispatched for this mix.
    pub links_requested: bool,
    /// Whether `MixAdded` has already been emitted to the UI/control layer.
    pub added_emitted: bool,
}

impl Mix {
    /// Creates a new `Mix`.
    pub fn new(name: impl Into<MixName>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Returns whether this mix's `PipeWire` sink and source ports are registered.
    #[must_use]
    pub fn are_pw_ports_ready(&self) -> bool {
        use ChannelPosition::{FrontLeft, FrontRight};
        use PortDirection::{Input, Output};

        if self.sink.id.is_none() || self.source.id.is_none() {
            return false;
        }

        self.sink.has_port(Input, FrontLeft)
            && self.sink.has_port(Input, FrontRight)
            && self.source.has_port(Output, FrontLeft)
            && self.source.has_port(Output, FrontRight)
    }

    /// Returns whether this mix node, its `JACK` ports, and its `PipeWire` links are ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.are_pw_ports_ready() && self.jack_ports.is_complete() && self.links.is_ready()
    }

    /// Builds the stereo [`StereoLink`] spec (`JACK -> mix.sink`) when all endpoints exist and
    /// links have not yet been requested.
    pub fn prepare_links(&mut self, jack_node_id: NodeId) -> Option<StereoLink> {
        if self.links_requested || !self.are_pw_ports_ready() {
            return None;
        }
        let sink_node_id = self.sink.id?;
        let jack_left = self.jack_ports.fl?;
        let jack_right = self.jack_ports.fr?;
        let in_left = self
            .sink
            .port_id(PortDirection::Input, ChannelPosition::FrontLeft)?;
        let in_right = self
            .sink
            .port_id(PortDirection::Input, ChannelPosition::FrontRight)?;

        let stereo = StereoLink::new(
            LinkEndpoint {
                node_id: jack_node_id,
                port_id: jack_left,
            },
            LinkEndpoint {
                node_id: sink_node_id,
                port_id: in_left,
            },
            LinkEndpoint {
                node_id: jack_node_id,
                port_id: jack_right,
            },
            LinkEndpoint {
                node_id: sink_node_id,
                port_id: in_right,
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
    use crate::types::{LinkId, PortId};

    fn setup_pw_ports(mix: &mut Mix, sink_id: NodeId, source_id: NodeId) {
        mix.sink.id = Some(sink_id);
        mix.source.id = Some(source_id);
        mix.sink.on_port_appeared(
            PortId(1),
            sink_id,
            PortDirection::Input,
            ChannelPosition::FrontLeft,
        );
        mix.sink.on_port_appeared(
            PortId(2),
            sink_id,
            PortDirection::Input,
            ChannelPosition::FrontRight,
        );
        mix.source.on_port_appeared(
            PortId(3),
            source_id,
            PortDirection::Output,
            ChannelPosition::FrontLeft,
        );
        mix.source.on_port_appeared(
            PortId(4),
            source_id,
            PortDirection::Output,
            ChannelPosition::FrontRight,
        );
    }

    #[test]
    fn test_mix_prepare_links_and_readiness() {
        let mut mix = Mix::new("Main");
        let jack_node = NodeId(99);

        setup_pw_ports(&mut mix, NodeId(20), NodeId(21));
        assert!(mix.are_pw_ports_ready());
        assert!(!mix.is_ready());
        assert!(mix.prepare_links(jack_node).is_none());

        mix.jack_ports
            .set_port(ChannelPosition::FrontLeft, PortId(60));
        mix.jack_ports
            .set_port(ChannelPosition::FrontRight, PortId(61));

        assert!(mix.prepare_links(jack_node).is_some());
        assert!(mix.prepare_links(jack_node).is_none());
        assert!(!mix.is_ready());

        assert!(mix.links.match_appeared(
            LinkId(300),
            LinkEndpoint {
                node_id: jack_node,
                port_id: PortId(60),
            },
            LinkEndpoint {
                node_id: NodeId(20),
                port_id: PortId(1),
            },
        ));
        assert!(mix.links.match_appeared(
            LinkId(301),
            LinkEndpoint {
                node_id: jack_node,
                port_id: PortId(61),
            },
            LinkEndpoint {
                node_id: NodeId(20),
                port_id: PortId(2),
            },
        ));
        assert!(mix.is_ready());
    }
}
