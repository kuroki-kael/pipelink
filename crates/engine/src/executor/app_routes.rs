/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Engine handlers and synchronization helpers for application-to-channel routing.

use pipelink_common::{
    events::StateEvent,
    graph::app_node::AppNode,
    types::{AppDisplay, ChannelName, NodeId, NodeSerial},
};
use pipelink_pw::Command as PipewireCommand;
use pipewire::channel::Sender as ChannelSender;
use tracing::{info, warn};

use crate::{config::topology::ConfigGraph, executor::Executor};

impl Executor {
    /// Routes all active and future stream nodes of `app` (by display name) into
    /// `channel`.
    pub(super) fn on_route_app(&mut self, app: AppDisplay, channel: &ChannelName) {
        let trimmed = app.trim();
        if trimmed.is_empty() {
            let _ = self.state_tx.send(StateEvent::AppRoutingFailed {
                app,
                error: "application name cannot be empty".to_string(),
            });
            return;
        }

        let Some(target_channel) = self
            .graph
            .channels()
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(channel))
        else {
            let _ = self.state_tx.send(StateEvent::AppRoutingFailed {
                app,
                error: format!("there is no channel named '{channel}'"),
            });
            return;
        };
        let channel = target_channel.name.clone();
        let target_serial = target_channel.node.serial;

        // Normalize to the resolved display name when the app is currently running,
        // otherwise keep the user-supplied display name so pre-routing rules work
        // before the app starts.
        let canonical_app = self
            .graph
            .apps
            .iter()
            .find(|a| a.matches_query(trimmed))
            .map_or_else(|| AppDisplay::from(trimmed), |a| a.app.display_name.clone());

        self.config.ensure_channel(&channel);
        self.config.set_app_route(canonical_app.clone(), &channel);
        self.storage.save_config(&self.config);

        for app_node in self
            .graph
            .apps
            .iter_mut()
            .filter(|a| a.matches_query(canonical_app.as_ref()))
        {
            app_node.channel = Some(channel.clone());
            app_node.target_serial = target_serial;
            if let Some(serial) = target_serial {
                dispatch_all_nodes(&self.pw_tx, app_node, serial);
            }
        }

        info!("Routed app '{canonical_app}' -> '{channel}'");
        let _ = self.state_tx.send(StateEvent::AppRouted {
            app: canonical_app,
            channel,
        });
    }

    /// Removes the channel routing rule for `app` and clears `target.object` on its
    /// active stream nodes.
    pub(super) fn on_unroute_app(&mut self, app: AppDisplay) {
        let trimmed = app.trim();
        let removed_rule = self.config.remove_app_route(&AppDisplay::from(trimmed));
        if removed_rule.is_some() {
            self.storage.save_config(&self.config);
        }

        let mut cleared_any = false;
        let mut canonical_name: Option<AppDisplay> = None;

        for app_node in self
            .graph
            .apps
            .iter_mut()
            .filter(|a| a.matches_query(trimmed))
        {
            if canonical_name.is_none() {
                canonical_name = Some(app_node.app.display_name.clone());
            }
            if app_node.channel.is_some() || app_node.target_serial.is_some() {
                cleared_any = true;
            }
            clear_app_node_route(&self.pw_tx, app_node);
        }

        let Some(resolved_name) =
            removed_rule.or_else(|| cleared_any.then_some(canonical_name).flatten())
        else {
            let _ = self.state_tx.send(StateEvent::AppRoutingFailed {
                app,
                error: "app has no route configured".to_string(),
            });
            return;
        };

        info!("Removed app route for '{resolved_name}'");
        let _ = self
            .state_tx
            .send(StateEvent::AppUnrouted { app: resolved_name });
    }

    /// Applies any configured route for `app_idx`'s newly resolved `node_id`.
    pub(super) fn apply_configured_app_route(&mut self, app_idx: usize, node_id: NodeId) {
        let Some(channel_name) = self.find_configured_channel(app_idx).cloned() else {
            return;
        };

        let Some(target_channel) = self
            .graph
            .channels()
            .iter()
            .find(|c| c.name == channel_name)
        else {
            return;
        };
        let target_serial = target_channel.node.serial;

        let Some(app_node) = self.graph.apps.get_mut(app_idx) else {
            return;
        };

        app_node.channel = Some(channel_name);
        app_node.target_serial = target_serial;

        if let Some(serial) = target_serial {
            send_set_app_route(
                &self.pw_tx,
                node_id,
                serial,
                app_node.app.display_name.as_ref(),
            );
        }
    }

    /// Pushes `SetAppRoute` for every active app currently assigned to `channel`.
    pub(super) fn sync_channel_app_routes(&mut self, channel: &ChannelName) {
        let Some(target_serial) = self
            .graph
            .channels()
            .iter()
            .find(|c| &c.name == channel)
            .and_then(|c| c.node.serial)
        else {
            return;
        };

        let indices: Vec<usize> = (0..self.graph.apps.len())
            .filter(|&idx| {
                self.graph.apps[idx].channel.as_ref() == Some(channel)
                    || self.find_configured_channel(idx) == Some(channel)
            })
            .collect();

        for idx in indices {
            if let Some(app_node) = self.graph.apps.get_mut(idx) {
                app_node.channel = Some(channel.clone());
                app_node.target_serial = Some(target_serial);
                dispatch_all_nodes(&self.pw_tx, app_node, target_serial);
            }
        }
    }

    /// Re-applies all active app routes once the `PipeWire` `default` metadata global
    /// appears.
    pub(super) fn on_metadata_appeared(&mut self) {
        let channels: Vec<ChannelName> = self
            .graph
            .channels()
            .iter()
            .filter(|c| c.node.serial.is_some())
            .map(|c| c.name.clone())
            .collect();

        for channel in &channels {
            self.sync_channel_app_routes(channel);
        }
    }

    /// Clears `target.object` on any active stream nodes routed to `channel` and,
    /// when `remove_from_config` is `true`, removes `channel` and its app rules
    /// from [`ConfigGraph`] and persists the config.
    pub(super) fn clear_channel_app_routes(
        &mut self,
        channel: &ChannelName,
        remove_from_config: bool,
    ) {
        if remove_from_config && self.config.remove_channel(channel) {
            self.storage.save_config(&self.config);
        }

        for app_node in self
            .graph
            .apps
            .iter_mut()
            .filter(|a| a.channel.as_ref() == Some(channel))
        {
            clear_app_node_route(&self.pw_tx, app_node);
        }
    }

    fn find_configured_channel(&self, app_idx: usize) -> Option<&ChannelName> {
        find_channel_for_app(&self.config, &self.graph.apps[app_idx])
    }
}

fn find_channel_for_app<'a>(
    config: &'a ConfigGraph,
    app_node: &AppNode,
) -> Option<&'a ChannelName> {
    config
        .find_app_channel(app_node.app.display_name.as_ref())
        .or_else(|| config.find_app_channel(&app_node.app.app_name))
        .or_else(|| {
            app_node
                .app
                .app_id
                .as_deref()
                .and_then(|id| config.find_app_channel(id))
        })
}

fn collect_set_routes(app_node: &AppNode, target_serial: NodeSerial) -> Vec<PipewireCommand> {
    let label = app_node.app.display_name.as_ref();
    app_node
        .nodes
        .iter()
        .filter_map(|node| {
            node.id.map(|node_id| PipewireCommand::SetAppRoute {
                node_id,
                target_serial,
                label: label.to_string(),
            })
        })
        .collect()
}

fn collect_clear_routes(app_node: &mut AppNode) -> Vec<PipewireCommand> {
    app_node.channel = None;
    app_node.target_serial = None;
    app_node
        .nodes
        .iter()
        .filter_map(|node| {
            node.id
                .map(|node_id| PipewireCommand::ClearNodeRoute { node_id })
        })
        .collect()
}

fn dispatch_all_nodes(
    pw_tx: &ChannelSender<PipewireCommand>,
    app_node: &AppNode,
    target_serial: NodeSerial,
) {
    for cmd in collect_set_routes(app_node, target_serial) {
        if let Err(err) = pw_tx.send(cmd) {
            warn!("Failed to send SetAppRoute: {err:?}");
        }
    }
}

fn clear_app_node_route(pw_tx: &ChannelSender<PipewireCommand>, app_node: &mut AppNode) {
    for cmd in collect_clear_routes(app_node) {
        if let Err(err) = pw_tx.send(cmd) {
            warn!("Failed to send ClearNodeRoute: {err:?}");
        }
    }
}

fn send_set_app_route(
    pw_tx: &ChannelSender<PipewireCommand>,
    node_id: NodeId,
    target_serial: NodeSerial,
    label: &str,
) {
    if let Err(err) = pw_tx.send(PipewireCommand::SetAppRoute {
        node_id,
        target_serial,
        label: label.to_string(),
    }) {
        warn!("Failed to send SetAppRoute for '{label}' (node {node_id}): {err:?}");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pipelink_common::{
        app::{App, WrapperKind},
        graph::node::Node,
        types::{AppDisplay, ChannelName, NodeId, NodeName, NodeSerial, Pid},
    };

    use super::*;

    fn test_app(display: &str, slug: &str) -> Arc<App> {
        Arc::new(App {
            pid: Pid(42),
            actual_pid: Pid(42),
            app_name: slug.to_string(),
            display_name: AppDisplay::from(display),
            wrapper: WrapperKind::None,
            binary_path: None,
            app_id: None,
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
            id: Some(NodeId(id)),
            name: NodeName::from("stream"),
            ..Default::default()
        }
    }

    #[test]
    fn test_dispatch_and_clear_multi_node_app_route() {
        let chat = ChannelName::from("Chat");

        let mut app_node = AppNode::new(test_app("Discord", "discord"), test_node(10));
        app_node.upsert_node(test_node(11));
        app_node.channel = Some(chat);
        app_node.target_serial = Some(NodeSerial(999));

        let dispatched: Vec<_> = collect_set_routes(&app_node, NodeSerial(999))
            .into_iter()
            .filter_map(|cmd| match cmd {
                PipewireCommand::SetAppRoute {
                    node_id,
                    target_serial,
                    ..
                } => Some((node_id, target_serial)),
                _ => None,
            })
            .collect();
        assert_eq!(
            dispatched,
            vec![(NodeId(10), NodeSerial(999)), (NodeId(11), NodeSerial(999))]
        );

        let cleared: Vec<_> = collect_clear_routes(&mut app_node)
            .into_iter()
            .filter_map(|cmd| match cmd {
                PipewireCommand::ClearNodeRoute { node_id } => Some(node_id),
                _ => None,
            })
            .collect();
        assert_eq!(app_node.channel, None);
        assert_eq!(app_node.target_serial, None);
        assert_eq!(cleared, vec![NodeId(10), NodeId(11)]);
    }

    #[test]
    fn test_find_channel_for_app_falls_back_to_slug() {
        let mut cfg = ConfigGraph::new();
        let music = ChannelName::from("Music");
        cfg.ensure_channel(&music);
        assert!(cfg.set_app_route(AppDisplay::from("Spotify"), &music));

        let app_node = AppNode::new(test_app("Spotify", "spotify"), test_node(1));
        assert_eq!(find_channel_for_app(&cfg, &app_node), Some(&music));

        let by_slug = AppNode::new(test_app("Spotify Free", "spotify"), test_node(2));
        cfg.remove_app_route(&AppDisplay::from("Spotify"));
        assert!(cfg.set_app_route(AppDisplay::from("spotify"), &music));
        assert_eq!(find_channel_for_app(&cfg, &by_slug), Some(&music));
    }
}
