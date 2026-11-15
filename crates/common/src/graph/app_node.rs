/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use std::sync::Arc;

use crate::{
    app::App,
    graph::node::Node,
    types::{ChannelName, NodeId, NodeSerial},
};

/// A resolved application and all active `PipeWire` stream nodes belonging to it.
#[derive(Debug, Clone, Default)]
pub struct AppNode {
    pub app: Arc<App>,
    pub nodes: Vec<Node>,
    pub channel: Option<ChannelName>,
    pub target_serial: Option<NodeSerial>,
}

impl AppNode {
    /// Creates a new [`AppNode`] from a resolved application and its initial node.
    #[must_use]
    pub fn new(app: Arc<App>, node: Node) -> Self {
        Self {
            app,
            nodes: vec![node],
            channel: None,
            target_serial: None,
        }
    }

    /// Inserts `node` or updates the existing entry sharing its `NodeId`.
    pub fn upsert_node(&mut self, node: Node) {
        if let Some(id) = node.id
            && let Some(existing) = self.nodes.iter_mut().find(|n| n.id == Some(id))
        {
            *existing = node;
            return;
        }
        self.nodes.push(node);
    }

    /// Removes the stream node matching `id`, returning `true` if one was removed.
    pub fn remove_node(&mut self, id: NodeId) -> bool {
        let before = self.nodes.len();
        self.nodes.retain(|n| n.id != Some(id));
        self.nodes.len() != before
    }

    /// Returns `true` if this app currently owns a stream node with `id`.
    #[must_use]
    pub fn has_node(&self, id: NodeId) -> bool {
        self.nodes.iter().any(|n| n.id == Some(id))
    }

    /// Returns `true` when `candidate` represents the same logical application.
    #[must_use]
    pub fn matches_app(&self, candidate: &App) -> bool {
        if self.app.actual_pid == candidate.actual_pid {
            return true;
        }
        if let (Some(a), Some(b)) = (&self.app.app_id, &candidate.app_id)
            && a == b
        {
            return true;
        }
        self.app
            .display_name
            .eq_ignore_ascii_case(&candidate.display_name)
    }

    /// Returns `true` when `query` matches this app's display name, slug, or app ID.
    #[must_use]
    pub fn matches_query(&self, query: &str) -> bool {
        let query = query.trim();
        !query.is_empty()
            && (self.app.display_name.eq_ignore_ascii_case(query)
                || self.app.app_name.eq_ignore_ascii_case(query)
                || self
                    .app
                    .app_id
                    .as_deref()
                    .is_some_and(|id| id.eq_ignore_ascii_case(query)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::WrapperKind,
        types::{AppDisplay, AppId, NodeName, Pid},
    };

    fn test_app(pid: u32, slug: &str, display: &str, app_id: Option<&str>) -> Arc<App> {
        Arc::new(App {
            pid: Pid(pid),
            actual_pid: Pid(pid),
            app_name: slug.to_string(),
            display_name: AppDisplay::from(display),
            wrapper: WrapperKind::None,
            binary_path: None,
            app_id: app_id.map(AppId::from),
            icon: None,
            desktop_file: None,
            categories: Vec::new(),
            generic_name: None,
            comment: None,
            cmdline: Vec::new(),
            cwd: None,
        })
    }

    fn test_node(id: u32) -> Node {
        Node {
            name: NodeName::from("stream"),
            id: Some(NodeId(id)),
            ..Default::default()
        }
    }

    #[test]
    fn test_app_node_tracks_multiple_nodes_and_lifecycle() {
        let app = test_app(100, "firefox", "Firefox", Some("org.mozilla.firefox"));
        let mut app_node = AppNode::new(app, test_node(1));
        app_node.upsert_node(test_node(2));
        assert_eq!(app_node.nodes.len(), 2);
        assert!(app_node.has_node(NodeId(1)));
        assert!(app_node.has_node(NodeId(2)));

        // Re-upserting the same NodeId updates in place rather than duplicating.
        app_node.upsert_node(test_node(2));
        assert_eq!(app_node.nodes.len(), 2);

        assert!(app_node.remove_node(NodeId(1)));
        assert!(!app_node.remove_node(NodeId(99)));
        assert_eq!(app_node.nodes.len(), 1);
    }

    #[test]
    fn test_app_node_matching_handles_display_name_and_app_id() {
        let discord = test_app(10, "discord", "Discord", Some("com.discordapp.Discord"));
        let app_node = AppNode::new(discord, test_node(1));

        let second_stream = test_app(20, "discord", "Discord", Some("com.discordapp.Discord"));
        let other = test_app(30, "reaper", "REAPER", None);

        assert!(app_node.matches_app(&second_stream));
        assert!(!app_node.matches_app(&other));
        assert!(app_node.matches_query("discord"));
        assert!(app_node.matches_query("Discord"));
        assert!(app_node.matches_query("com.discordapp.Discord"));
        assert!(!app_node.matches_query(""));
    }
}
