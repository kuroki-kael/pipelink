/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Handlers for `PipeWire` global removals and creation/removal failures.

use pipelink_common::{
    events::StateEvent,
    types::{ChannelName, GlobalId, LinkId, MixName, NodeId},
};
use pipelink_resolver::is_process_alive;
use tracing::{info, warn};

use crate::executor::Executor;

impl Executor {
    /// Drops the nodes that vanished from the `PipeWire` registry.
    ///
    /// The entry is taken out of the graph here and not when the removal was
    /// requested, because only the vanished node confirms the removal.
    pub(in crate::executor) fn on_global_removed(&mut self, id: GlobalId) {
        let link_id = LinkId::from(id.0);
        let id = NodeId::from(id);

        if self.graph.jack_node_id == Some(id) {
            self.graph.jack_node_id = None;
            return;
        }

        if let Some(Some(channel)) = self
            .graph
            .channels()
            .iter()
            .position(|c| c.node.id == Some(id))
            .map(|pos| self.graph.remove_channel_idx(pos))
        {
            let user_requested = self.pending_channel_removals.remove(&channel.name);
            self.clear_channel_app_routes(&channel.name, user_requested);
            if let Err(err) = self
                .jack
                .remove_channel(&channel.name, self.graph.gains_snapshot())
            {
                warn!("Failed to remove JACK channel '{}': {err}", channel.name);
            }
            let _ = self.state_tx.send(StateEvent::ChannelRemoved {
                channel: channel.name,
            });
        }
        if let Some(Some(mix)) = self
            .graph
            .mixes()
            .iter()
            .position(|m| m.source.id == Some(id) || m.sink.id == Some(id))
            .map(|pos| self.graph.remove_mix_idx(pos))
        {
            if self.pending_mix_removals.remove(&mix.name) && self.config.remove_mix(&mix.name) {
                self.storage.save_config(&self.config);
            }
            if let Err(err) = self.jack.remove_mix(&mix.name, self.graph.gains_snapshot()) {
                warn!("Failed to remove JACK mix '{}': {err}", mix.name);
            }
            let _ = self.state_tx.send(StateEvent::MixRemoved { mix: mix.name });
            self.ensure_active_mix();
        }

        if let Some(pos) = self
            .graph
            .outputs
            .iter()
            .position(|o| o.node.id == Some(id))
        {
            let removed = self.graph.outputs.remove(pos);
            if self.graph.selected_output.as_ref() == Some(&removed.node.name) {
                self.graph.selected_output = None;
                self.ensure_selected_output();
                self.sync_active_mix_output();
            }
        }

        let mut evicted_pids = Vec::new();
        for app_node in &mut self.graph.apps {
            if app_node.remove_node(id) && app_node.nodes.is_empty() {
                evicted_pids.push(app_node.app.pid);
            }
        }
        self.graph.apps.retain(|a| !a.nodes.is_empty());

        for pid in evicted_pids {
            let other_active = self.graph.apps.iter().any(|a| a.app.pid == pid);
            if !other_active && !is_process_alive(pid) {
                self.resolver.invalidate(pid);
                info!("Process {pid} exited, evicted from resolver cache");
            }
        }

        self.on_link_removed(link_id);
    }

    /// Reports a node that could not be destroyed.
    ///
    /// The entry stays in the graph, because the node is still there. The
    /// client learns that the removal did not happen.
    pub(in crate::executor) fn on_global_removal_failed(&mut self, id: GlobalId, reason: &str) {
        let id = NodeId::from(id);

        if let Some(ch_name) = self
            .graph
            .channels()
            .iter()
            .find(|c| c.node.id == Some(id))
            .map(|c| c.name.clone())
        {
            self.pending_channel_removals.remove(&ch_name);
            warn!("Channel '{ch_name}' could not be removed: {reason}");
            let _ = self.state_tx.send(StateEvent::ChannelRemovalFailed {
                channel: ch_name,
                error: reason.to_string(),
            });
        }

        if let Some(mix_name) = self
            .graph
            .mixes()
            .iter()
            .find(|m| m.source.id == Some(id) || m.sink.id == Some(id))
            .map(|m| m.name.clone())
        {
            self.pending_mix_removals.remove(&mix_name);
            warn!("Mix '{mix_name}' could not be removed: {reason}");
            let _ = self.state_tx.send(StateEvent::MixRemovalFailed {
                mix: mix_name,
                error: reason.to_string(),
            });
        }
    }

    /// Drops a channel that `PipeWire` refused to create.
    ///
    /// The channel was added to the graph optimistically, so it has to be
    /// taken out again once the creation fails.
    pub(in crate::executor) fn on_channel_creation_failed(
        &mut self,
        name: ChannelName,
        reason: &str,
    ) {
        self.graph.remove_channel(&name);
        let newly_created = self.pending_channel_creates.remove(&name);
        self.clear_channel_app_routes(&name, newly_created);
        if let Err(err) = self.jack.remove_channel(&name, self.graph.gains_snapshot()) {
            warn!("Failed to remove JACK channel '{name}': {err}");
        }

        warn!("Channel '{name}' could not be created: {reason}");

        let _ = self.state_tx.send(StateEvent::ChannelCreationFailed {
            channel: name,
            error: reason.to_string(),
        });
    }

    /// Reports a mix whose loopback could not be removed.
    pub(in crate::executor) fn on_mix_removal_failed(&mut self, name: MixName, reason: &str) {
        self.pending_mix_removals.remove(&name);
        warn!("Mix '{name}' could not be removed: {reason}");

        let _ = self.state_tx.send(StateEvent::MixRemovalFailed {
            mix: name,
            error: reason.to_string(),
        });
    }

    /// Drops a mix that `PipeWire` refused to create.
    ///
    /// The mix was added to the graph optimistically, so it has to be taken
    /// out again once the creation fails.
    pub(in crate::executor) fn on_mix_creation_failed(&mut self, name: MixName, reason: &str) {
        self.graph.remove_mix(&name);
        if self.pending_mix_creates.remove(&name) && self.config.remove_mix(&name) {
            self.storage.save_config(&self.config);
        }
        if let Err(err) = self.jack.remove_mix(&name, self.graph.gains_snapshot()) {
            warn!("Failed to remove JACK mix '{name}': {err}");
        }
        self.ensure_active_mix();

        warn!("Mix '{name}' could not be created: {reason}");

        let _ = self.state_tx.send(StateEvent::MixCreationFailed {
            mix: name,
            error: reason.to_string(),
        });
    }
}
