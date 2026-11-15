/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Internal node representation tracking IDs, serials, and port mappings.

use std::collections::HashMap;

use crate::types::{ChannelPosition, NodeId, NodeName, NodeSerial, PortDirection, PortId};

/// Direction and channel position of a single registered port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortInfo {
    /// Direction the port faces.
    pub direction: PortDirection,
    /// Channel position the port occupies within its node.
    pub position: ChannelPosition,
}

/// `PipeWire` node metadata and registered ports.
#[derive(Debug, Clone, Default)]
pub struct Node {
    pub name: NodeName,
    pub id: Option<NodeId>,
    pub serial: Option<NodeSerial>,
    /// Ports of this node, keyed by their `PipeWire` port id.
    pub ports: HashMap<PortId, PortInfo>,
}

impl Node {
    /// Returns whether this node is considered ready.
    #[must_use]
    pub(crate) fn is_ready(&self) -> bool {
        !self.name.is_empty() && self.serial.is_some() && !self.ports.is_empty()
    }

    /// Returns whether a port with the given direction and position is registered.
    #[must_use]
    pub(crate) fn has_port(&self, direction: PortDirection, position: ChannelPosition) -> bool {
        self.ports
            .values()
            .any(|port| port.direction == direction && port.position == position)
    }

    /// Returns the `PortId` matching `direction` and `position`, if registered.
    #[must_use]
    pub fn port_id(&self, direction: PortDirection, position: ChannelPosition) -> Option<PortId> {
        self.ports
            .iter()
            .find(|(_, info)| info.direction == direction && info.position == position)
            .map(|(id, _)| *id)
    }

    /// Registers a port on this node if the `node_id` matches.
    pub fn on_port_appeared(
        &mut self,
        port_id: PortId,
        node_id: NodeId,
        direction: PortDirection,
        position: ChannelPosition,
    ) -> bool {
        if self.id != Some(node_id) {
            return false;
        }

        self.ports.insert(
            port_id,
            PortInfo {
                direction,
                position,
            },
        );
        true
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn node_with_id(id: NodeId) -> Node {
        Node {
            id: Some(id),
            ..Default::default()
        }
    }

    #[test]
    fn test_on_port_appeared_rejects_foreign_node() {
        let mut node = node_with_id(NodeId(1));

        assert!(!node.on_port_appeared(
            PortId(10),
            NodeId(2),
            PortDirection::Input,
            ChannelPosition::FrontLeft
        ));
        assert!(node.ports.is_empty());
    }

    #[test]
    fn test_ports_with_same_direction_but_distinct_position_coexist() {
        let mut node = node_with_id(NodeId(1));

        assert!(node.on_port_appeared(
            PortId(10),
            NodeId(1),
            PortDirection::Output,
            ChannelPosition::Aux(0)
        ));
        assert!(node.on_port_appeared(
            PortId(11),
            NodeId(1),
            PortDirection::Output,
            ChannelPosition::Aux(1)
        ));

        assert_eq!(node.ports.len(), 2);
        assert_eq!(
            node.ports.get(&PortId(10)).unwrap().position,
            ChannelPosition::Aux(0)
        );
        assert_eq!(
            node.ports.get(&PortId(11)).unwrap().position,
            ChannelPosition::Aux(1)
        );
    }

    #[test]
    fn test_has_port_matches_direction_and_position() {
        let mut node = node_with_id(NodeId(1));

        node.on_port_appeared(
            PortId(10),
            NodeId(1),
            PortDirection::Input,
            ChannelPosition::FrontLeft,
        );

        assert!(node.has_port(PortDirection::Input, ChannelPosition::FrontLeft));
        assert!(!node.has_port(PortDirection::Output, ChannelPosition::FrontLeft));
        assert!(!node.has_port(PortDirection::Input, ChannelPosition::FrontRight));
    }
}
