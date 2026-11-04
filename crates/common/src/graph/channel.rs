/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use std::collections::HashMap;

use crate::graph::node::{NativeNode, Node as PipewireNode};

/// Channel node backed by a `Pipewire` `support.null-audio-sink` adapter.
#[derive(Debug, Clone, Default)]
pub struct Channel {
    name: String,
    node: PipewireNode,
}

impl Channel {
    /// Creates a mew `Channel`.
    ///
    /// # Errors
    ///
    /// Returns a [`PipewireError`] if creation failed.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let mut node = PipewireNode::default();
        node.set_name(&name);
        Self { name, node }
    }

    /// Returns the `PipeWire` global ID of the underlying node, if set.
    #[must_use]
    pub fn get_id(&self) -> Option<u32> {
        self.node.get_id()
    }

    /// Sets the `PipeWire` global ID of the underlying node.
    pub fn set_id(&mut self, id: Option<u32>) {
        self.node.set_id(id);
    }

    /// Returns a reference to the underlying [`PipewireNode`].
    #[must_use]
    pub fn get_node(&self) -> &PipewireNode {
        &self.node
    }

    /// Returns a mutable reference to the underlying [`PipewireNode`].
    pub fn get_node_mut(&mut self) -> &mut PipewireNode {
        &mut self.node
    }

    /// Returns whether this channel node is ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        <Self as NativeNode>::is_ready(self)
    }
}

impl NativeNode for Channel {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    fn get_ports_from_node(&self, node: u32) -> Option<&HashMap<String, u32>> {
        if self.node.get_id() == Some(node) {
            Some(self.node.get_ports())
        } else {
            None
        }
    }

    fn on_port_appeared(
        &mut self,
        port_id: u32,
        node_id: u32,
        direction: &str,
        channel_pos: &str,
    ) -> bool {
        if self.node.get_id() != Some(node_id) {
            return false;
        }

        let key = match (direction, channel_pos) {
            ("Input", "FL") => "input_fl",
            ("Input", "FR") => "input_fr",
            ("Output", "FL") => "output_fl",
            ("Output", "FR") => "output_fr",
            _ => return false,
        };

        self.node.add_port(key, port_id);
        true
    }

    fn is_ready(&self) -> bool {
        if self.node.get_id().is_none() {
            return false;
        }

        self.node.get_port("input_fl").is_some()
            && self.node.get_port("input_fr").is_some()
            && self.node.get_port("output_fl").is_some()
            && self.node.get_port("output_fr").is_some()
    }
}
