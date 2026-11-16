/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Engine handlers and synchronization helpers for active-mix and output routing.

use pipelink_common::{
    events::StateEvent,
    graph::{node::Node, output::OutputDevice},
    types::{MixName, NodeId, NodeName, NodeSerial},
};
use pipelink_pw::Command as PipewireCommand;
use tracing::{info, warn};

use crate::executor::Executor;

mod planner;
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;

use planner::{
    desired_output_link, pick_fallback_active_mix, pick_selected_output, plan_output_link_sync,
    resolve_default_sink_target, resolve_output_query,
};

impl Executor {
    /// Records or updates an available `Audio/Sink` output device and ensures a
    /// valid output is selected and linked.
    pub(super) fn on_output_node_appeared(
        &mut self,
        id: NodeId,
        name: &NodeName,
        serial: Option<NodeSerial>,
        description: Option<String>,
        nick: Option<String>,
        is_hardware: bool,
    ) {
        if let Some(existing) = self
            .graph
            .outputs
            .iter_mut()
            .find(|o| o.node.id == Some(id))
        {
            existing.node.name = name.clone();
            existing.node.serial = serial;
            existing.description = description;
            existing.nick = nick;
            existing.is_hardware = is_hardware;
        } else {
            self.graph.outputs.push(OutputDevice {
                node: Node {
                    name: name.clone(),
                    id: Some(id),
                    serial,
                    ..Default::default()
                },
                description,
                nick,
                is_hardware,
            });
        }

        self.ensure_selected_output();
        self.sync_active_mix_output();
    }

    /// Selects an output sink matching `query` (`node.name`, description, or nick).
    pub(super) fn on_set_output(&mut self, query: &str) {
        let matched = match resolve_output_query(&self.graph.outputs, query) {
            Ok(dev) => dev.node.name.clone(),
            Err(error) => {
                let _ = self
                    .state_tx
                    .send(StateEvent::OutputOperationFailed { error });
                return;
            },
        };

        self.apply_selected_output(Some(matched), true);
        self.sync_active_mix_output();
    }

    /// Sets `WirePlumber`'s default audio sink (`default.configured.audio.sink`) to a
    /// `PipeLink` channel or an available output device.
    pub(super) fn on_set_default_sink(&mut self, query: &str) {
        let node_name = match resolve_default_sink_target(&self.graph, query) {
            Ok(name) => name,
            Err(error) => {
                let _ = self
                    .state_tx
                    .send(StateEvent::DefaultSinkOperationFailed { error });
                return;
            },
        };

        if let Err(err) = self
            .pw_tx
            .send(PipewireCommand::SetDefaultSink { node_name })
        {
            let _ = self.state_tx.send(StateEvent::DefaultSinkOperationFailed {
                error: format!("failed to send PipeWire command: {err:?}"),
            });
        }
    }

    /// Selects `mix` (case-insensitively) as the globally active mix driving the selected
    /// output.
    pub(super) fn on_select_active_mix(&mut self, mix: MixName) {
        let Some(canonical) = self
            .graph
            .mixes()
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(&mix))
            .map(|m| m.name.clone())
        else {
            let _ = self.state_tx.send(StateEvent::LayerSelectionFailed {
                mix,
                error: "there is no mix with this name".to_string(),
            });
            return;
        };

        self.set_active_mix(Some(canonical));
        self.sync_active_mix_output();
    }

    /// Auto-selects `mix` if no active mix is currently chosen.
    pub(super) fn maybe_auto_select_active_mix(&mut self, mix: &MixName) {
        if self.graph.active_mix.is_none() {
            self.set_active_mix(Some(mix.clone()));
            self.sync_active_mix_output();
        }
    }

    /// Ensures `active_mix` points to an existing mix, falling back to the
    /// first remaining mix (or `None` if no mixes remain).
    pub(super) fn ensure_active_mix(&mut self) {
        let next = pick_fallback_active_mix(self.graph.mixes(), self.graph.active_mix.as_ref());
        if next != self.graph.active_mix {
            self.set_active_mix(next);
        }
        self.sync_active_mix_output();
    }

    /// Ensures `selected_output` points to an available output sink, preferring
    /// hardware sinks when failing over.
    pub(super) fn ensure_selected_output(&mut self) {
        let next = pick_selected_output(
            &self.graph.outputs,
            self.graph.selected_output.as_ref(),
            self.config.output.as_ref(),
        );
        if next != self.graph.selected_output {
            self.apply_selected_output(next, false);
        }
    }

    /// Reconciles the stereo `PipeWire` links from `active_mix.source` to
    /// `selected_output` with the current graph state.
    pub(super) fn sync_active_mix_output(&mut self) {
        let desired = desired_output_link(&self.graph);
        let plan = plan_output_link_sync(
            &self.graph.active_output_links,
            self.graph.output_links_requested,
            desired,
        );

        for link_id in plan.destroy_ids {
            if let Err(err) = self.pw_tx.send(PipewireCommand::DestroyLink { link_id }) {
                warn!("Failed to destroy output link {link_id}: {err:?}");
            }
        }

        self.graph.active_output_links = plan.next_links;
        self.graph.output_links_requested = plan.next_requested;

        if let Some(stereo_link) = plan.create {
            self.dispatch_create_link(stereo_link);
        }
    }

    fn set_active_mix(&mut self, mix: Option<MixName>) {
        if self.graph.active_mix != mix {
            self.storage.mark_state_dirty();
        }
        self.graph.active_mix.clone_from(&mix);
        self.config.active_mix.clone_from(&mix);
        if let Some(mix) = mix {
            info!("Active mix set to '{mix}'");
            let _ = self.state_tx.send(StateEvent::GlobalLayerSwitched { mix });
        } else {
            info!("No mixes remaining; active mix cleared");
        }
    }

    fn apply_selected_output(&mut self, output: Option<NodeName>, persist_preference: bool) {
        self.graph.selected_output.clone_from(&output);
        if let Some(output) = output {
            if persist_preference {
                self.config.output = Some(output.clone());
                self.storage.save_config(&self.config);
            }
            info!("Selected output set to '{output}'");
            let _ = self.state_tx.send(StateEvent::OutputChanged { output });
        } else {
            info!("No audio outputs currently available");
        }
    }
}
