/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Internal node representation tracking IDs, serials, and port mappings.

use std::collections::HashMap;

/// `PipeWire` node metadata and registered ports.
#[derive(Debug, Clone, Default)]
pub struct Node {
    pub name: String,
    pub id: Option<u32>,
    pub serial: Option<u64>,
    pub ports: HashMap<String, u32>,
}

impl Node {
    /// Sets the node name.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.into();
    }

    /// Returns the node name.
    #[must_use]
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Sets the node global ID.
    pub fn set_id(&mut self, id: Option<u32>) {
        self.id = id;
    }

    /// Returns the node global ID.
    #[must_use]
    pub fn get_id(&self) -> Option<u32> {
        self.id
    }

    /// Sets the node 64-bit serial number.
    pub fn set_serial(&mut self, serial: Option<u64>) {
        self.serial = serial;
    }

    /// Returns the node 64-bit serial number.
    #[must_use]
    pub fn get_serial(&self) -> Option<u64> {
        self.serial
    }

    /// Adds or updates a port by name.
    pub fn add_port(&mut self, port: &str, id: u32) {
        self.ports.insert(port.into(), id);
    }

    /// Removes a port by name.
    pub fn remove_port(&mut self, port: &str) {
        self.ports.remove(port);
    }

    /// Returns the port ID for a given port name, if present.
    #[must_use]
    pub fn get_port(&self, port: &str) -> Option<u32> {
        self.ports.get(port).copied()
    }

    /// Returns a reference to all ports mapped on this node.
    #[must_use]
    pub fn get_ports(&self) -> &HashMap<String, u32> {
        &self.ports
    }

    /// Returns whether this node is considered ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        !self.name.is_empty() && self.serial.is_some() && !self.ports.is_empty()
    }
}

/// Common trait representing a native `PipeWire` node in `PipeLink`.
pub trait NativeNode {
    /// Returns the node name.
    fn get_name(&self) -> &str;

    /// Returns a mutable reference to the node name.
    fn get_name_mut(&mut self) -> &mut String;

    /// Sets the node name.
    ///
    /// Returns `true` if updated, or `false` if `name` is blank.
    fn set_name(&mut self, name: impl Into<String>) -> bool {
        let name = name.into();
        if name.trim().is_empty() {
            return false;
        }
        *self.get_name_mut() = name;
        true
    }

    /// Returns ports map for a specific `PipeWire` node ID if managed by this native
    /// node.
    fn get_ports_from_node(&self, node: u32) -> Option<&HashMap<String, u32>>;

    /// Returns the port ID for a given port name and `PipeWire` node ID.
    fn get_port(&self, port: &str, node: u32) -> Option<u32> {
        self.get_ports_from_node(node)
            .and_then(|ports| ports.get(port))
            .copied()
    }

    /// Handles port registration when a port appears in `PipeWire`.
    fn on_port_appeared(
        &mut self,
        port_id: u32,
        node_id: u32,
        direction: &str,
        channel_pos: &str,
    ) -> bool;

    /// Returns whether this native node is fully initialized and ready.
    fn is_ready(&self) -> bool;
}
