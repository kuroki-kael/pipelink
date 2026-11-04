/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use std::collections::HashMap;

use crate::graph::node::{NativeNode, Node as PipewireNode};

/// Mix node backed by a `PipeWire` `libpipewire-module-loopback`.
///
/// Contains a sink node (`pipelink.mix.{name}`) and a source node
/// (`pipelink.mix.{name}.source`)
#[derive(Clone, Debug, Default)]
pub struct Mix {
    name: String,
    sink: PipewireNode,
    source: PipewireNode,
}

impl Mix {
    /// Creates a new `Mix`.
    ///
    /// # Errors
    ///
    /// Returns a [`PipewireError`] if creation failed.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let mut sink = PipewireNode::default();
        sink.set_name(&format!("pipelink.mix.{name}"));
        let mut source = PipewireNode::default();
        source.set_name(&format!("pipelink.mix.{name}.source"));

        Self { name, sink, source }
    }

    /// Returns a reference to the sink [`PipewireNode`].
    #[must_use]
    pub fn get_sink(&self) -> &PipewireNode {
        &self.sink
    }

    /// Returns a mutable reference to the sink [`PipewireNode`].
    pub fn get_sink_mut(&mut self) -> &mut PipewireNode {
        &mut self.sink
    }

    /// Returns a reference to the source [`PipewireNode`].
    #[must_use]
    pub fn get_source(&self) -> &PipewireNode {
        &self.source
    }

    /// Returns a mutable reference to the source [`PipewireNode`].
    pub fn get_source_mut(&mut self) -> &mut PipewireNode {
        &mut self.source
    }

    /// Returns the `PipeWire` global ID of the sink node.
    #[must_use]
    pub fn get_sink_id(&self) -> Option<u32> {
        self.sink.get_id()
    }

    /// Sets the `PipeWire` global ID of the sink node.
    pub fn set_sink_id(&mut self, id: Option<u32>) {
        self.sink.set_id(id);
    }

    /// Returns the `PipeWire` global ID of the source node.
    #[must_use]
    pub fn get_source_id(&self) -> Option<u32> {
        self.source.get_id()
    }

    /// Sets the `PipeWire` global ID of the source node.
    pub fn set_source_id(&mut self, id: Option<u32>) {
        self.source.set_id(id);
    }

    /// Returns whether `node_name` matches this mix's sink node name.
    #[must_use]
    pub fn matches_sink_node(&self, node_name: &str) -> bool {
        node_name.strip_prefix("pipelink.mix.") == Some(&self.name)
    }

    /// Returns whether `node_name` matches this mix's source node name.
    #[must_use]
    pub fn matches_source_node(&self, node_name: &str) -> bool {
        node_name
            .strip_prefix("pipelink.mix.")
            .and_then(|rest| rest.strip_suffix(".source"))
            == Some(&self.name)
    }

    /// Returns whether this mix node is ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        <Self as NativeNode>::is_ready(self)
    }
}

impl NativeNode for Mix {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    fn get_ports_from_node(&self, node: u32) -> Option<&HashMap<String, u32>> {
        if self.sink.get_id() == Some(node) {
            Some(self.sink.get_ports())
        } else if self.source.get_id() == Some(node) {
            Some(self.source.get_ports())
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
        if Some(node_id) == self.sink.get_id() {
            let key = match (direction, channel_pos) {
                ("Input", "FL") => "input_fl",
                ("Input", "FR") => "input_fr",
                ("Output", "FL") => "monitor_fl",
                ("Output", "FR") => "monitor_fr",
                _ => return false,
            };

            self.sink.add_port(key, port_id);
            true
        } else if Some(node_id) == self.source.get_id() {
            let key = match (direction, channel_pos) {
                ("Output", "FL") => "output_fl",
                ("Output", "FR") => "output_fr",
                _ => return false,
            };

            self.source.add_port(key, port_id);
            true
        } else {
            false
        }
    }

    fn is_ready(&self) -> bool {
        if self.sink.get_id().is_none() || self.source.get_id().is_none() {
            return false;
        }

        self.sink.get_port("input_fl").is_some()
            && self.sink.get_port("input_fr").is_some()
            && self.source.get_port("output_fl").is_some()
            && self.source.get_port("output_fr").is_some()
    }
}
