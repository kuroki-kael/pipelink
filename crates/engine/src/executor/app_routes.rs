/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Engine handlers and synchronization helpers for application-to-channel routing.

mod helpers;
#[cfg(test)]
mod tests;

use helpers::{clear_app_node_route, dispatch_all_nodes, find_channel_for_app, send_set_app_route};
use pipelink_common::{
    events::StateEvent,
    types::{AppDisplay, ChannelName, NodeId},
};
use tracing::info;

use crate::executor::Executor;

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
