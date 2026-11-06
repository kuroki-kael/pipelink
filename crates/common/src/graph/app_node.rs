/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use std::sync::Arc;

use crate::{app::App, graph::node::Node, types::NodeSerial};

/// A `PipeWire` node that has been attributed to a resolved application.
#[derive(Debug, Clone, Default)]
pub struct AppNode {
    pub app: Arc<App>,
    pub node: Node,
    pub target_serial: Option<NodeSerial>,
}

impl AppNode {
    /// Creates a new [`AppNode`] from a resolved application and its node.
    pub fn new(app: Arc<App>, node: Node) -> Self {
        Self {
            app,
            node,
            target_serial: None,
        }
    }
}
