/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use std::{collections::HashMap, sync::Arc};

use crate::{
    app::App,
    graph::node::{NativeNode, Node as PipewireNode},
};

#[derive(Debug, Clone, Default)]
pub struct AppNode {
    pub app: Arc<App>,
    pub node: PipewireNode,
    pub target_serial: Option<u64>,
}

impl AppNode {
    pub fn new(app: Arc<App>, node: PipewireNode) -> Self {
        Self {
            app,
            node,
            target_serial: None,
        }
    }
}

impl NativeNode for AppNode {
    fn get_name(&self) -> &str {
        self.node.get_name()
    }

    fn get_name_mut(&mut self) -> &mut String {
        &mut self.node.name
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
        self.node.is_ready()
    }
}
