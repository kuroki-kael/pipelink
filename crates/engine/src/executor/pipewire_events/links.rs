/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Handlers for `PipeWire` link appearance, link removal, and pending link creation.

use pipelink_common::{
    events::StateEvent,
    graph::link::{LinkEndpoint, StereoLink},
    types::{LinkId, NodeId, PortId},
};
use pipelink_pw::Command as PipewireCommand;
use tracing::{info, warn};

use crate::executor::Executor;

impl Executor {
    /// Records a newly appeared `PipeWire` link on its channel or mix and emits
    /// `ChannelAdded` / `MixAdded` once both stereo links are confirmed.
    pub(in crate::executor) fn on_link_appeared(
        &mut self,
        id: LinkId,
        out_node: NodeId,
        in_node: NodeId,
        out_port: PortId,
        in_port: PortId,
    ) {
        let out_ep = LinkEndpoint {
            node_id: out_node,
            port_id: out_port,
        };
        let in_ep = LinkEndpoint {
            node_id: in_node,
            port_id: in_port,
        };

        for ch in self.graph.channels_mut() {
            if ch.links.match_appeared(id, out_ep, in_ep) {
                if !ch.added_emitted && ch.is_ready() {
                    ch.added_emitted = true;
                    let ch_name = ch.name.clone();
                    info!("Channel '{ch_name}' is ready");
                    self.pending_channel_creates.remove(&ch_name);
                    if !self.finish_pending_channel_rename(&ch_name) {
                        // The graph entry is optimistic, the ChannelAdded event is sent
                        // once PipeWire has registered all ports and stereo links to
                        // JACK.
                        let _ = self
                            .state_tx
                            .send(StateEvent::ChannelAdded { channel: ch_name });
                    }
                }
                return;
            }
        }

        for m in self.graph.mixes_mut() {
            if m.links.match_appeared(id, out_ep, in_ep) {
                if !m.added_emitted && m.is_ready() {
                    m.added_emitted = true;
                    let mix_name = m.name.clone();
                    info!("Mix '{mix_name}' is ready");
                    self.pending_mix_creates.remove(&mix_name);
                    if !self.finish_pending_mix_rename(&mix_name) {
                        // The graph entry is optimistic, the MixAdded event is sent once
                        // PipeWire has registered all ports and stereo links from JACK.
                        let _ = self.state_tx.send(StateEvent::MixAdded { mix: mix_name });
                    }
                }
                return;
            }
        }

        let _ = self
            .graph
            .active_output_links
            .match_appeared(id, out_ep, in_ep);
    }

    /// Clears a disappeared link ID from whichever channel, mix, or output route owned
    /// it.
    pub(in crate::executor) fn on_link_removed(&mut self, id: LinkId) {
        for ch in self.graph.channels_mut() {
            if ch.links.clear_link_id(id) {
                return;
            }
        }
        for m in self.graph.mixes_mut() {
            if m.links.clear_link_id(id) {
                return;
            }
        }
        if self.graph.active_output_links.clear_link_id(id) {
            self.graph.output_links_requested = false;
        }
    }

    pub(super) fn try_link_pending(&mut self, jack_node_id: NodeId) {
        let mut pending = Vec::new();
        for ch in self.graph.channels_mut() {
            if let Some(stereo_link) = ch.prepare_links(jack_node_id) {
                pending.push(stereo_link);
            }
        }
        for m in self.graph.mixes_mut() {
            if let Some(stereo_link) = m.prepare_links(jack_node_id) {
                pending.push(stereo_link);
            }
        }
        for stereo_link in pending {
            self.dispatch_create_link(stereo_link);
        }
    }

    pub(in crate::executor) fn dispatch_create_link(&self, stereo_link: StereoLink) {
        if let Err(err) = self.pw_tx.send(PipewireCommand::CreateLink { stereo_link }) {
            warn!("Failed to send CreateLink command to PipeWire: {err:?}");
        }
    }
}
