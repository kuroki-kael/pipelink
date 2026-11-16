/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Engine handlers and synchronization helpers for active-mix and output routing.

use pipelink_common::{
    events::StateEvent,
    graph::{
        audio_graph::AudioGraph,
        link::{LinkEndpoint, StereoLink},
        mix::Mix,
        node::Node,
        output::OutputDevice,
    },
    types::{ChannelPosition, LinkId, MixName, NodeId, NodeName, NodeSerial, PortDirection},
};
use pipelink_pw::Command as PipewireCommand;
use tracing::{info, warn};

use crate::executor::Executor;

#[derive(Debug, PartialEq, Eq)]
struct OutputLinkPlan {
    destroy_ids: Vec<LinkId>,
    create: Option<StereoLink>,
    next_links: StereoLink,
    next_requested: bool,
}

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

fn resolve_output_query<'a>(
    outputs: &'a [OutputDevice],
    query: &str,
) -> Result<&'a OutputDevice, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err("output query cannot be empty".to_string());
    }

    if let Some(exact) = outputs.iter().find(|o| *o.node.name == *trimmed) {
        return Ok(exact);
    }

    let exact_ci: Vec<&OutputDevice> = outputs
        .iter()
        .filter(|o| o.matches_exact_query(trimmed))
        .collect();
    if let [single] = exact_ci.as_slice() {
        return Ok(single);
    }

    let matches: Vec<&OutputDevice> = outputs
        .iter()
        .filter(|o| o.matches_query(trimmed))
        .collect();
    match matches.as_slice() {
        [] => Err(format!("no output device matches '{trimmed}'")),
        [single] => Ok(single),
        _ => {
            let candidates: Vec<&str> = matches.iter().map(|o| o.display_name()).collect();
            Err(format!(
                "output '{trimmed}' is ambiguous ({} matches: {})",
                matches.len(),
                candidates.join(", ")
            ))
        },
    }
}

fn pick_selected_output(
    outputs: &[OutputDevice],
    current: Option<&NodeName>,
    configured: Option<&NodeName>,
) -> Option<NodeName> {
    if let Some(name) = configured
        && outputs.iter().any(|o| o.node.name == *name)
    {
        return Some(name.clone());
    }

    if let Some(name) = current
        && outputs.iter().any(|o| o.node.name == *name)
    {
        return Some(name.clone());
    }

    outputs
        .iter()
        .find(|o| o.is_hardware)
        .or_else(|| outputs.first())
        .map(|o| o.node.name.clone())
}

fn pick_fallback_active_mix(mixes: &[Mix], current: Option<&MixName>) -> Option<MixName> {
    if let Some(name) = current
        && mixes.iter().any(|m| m.name == *name)
    {
        return Some(name.clone());
    }
    mixes.first().map(|m| m.name.clone())
}

fn desired_output_link(graph: &AudioGraph) -> Option<StereoLink> {
    let active_mix = graph.active_mix.as_ref()?;
    let selected_output = graph.selected_output.as_ref()?;

    let mix = graph.mixes().iter().find(|m| m.name == *active_mix)?;
    let output = graph
        .outputs
        .iter()
        .find(|o| o.node.name == *selected_output)?;

    let src_node = mix.source.id?;
    let src_left = mix
        .source
        .port_id(PortDirection::Output, ChannelPosition::FrontLeft)?;
    let src_right = mix
        .source
        .port_id(PortDirection::Output, ChannelPosition::FrontRight)?;

    let out_node = output.node.id?;
    let out_left = output
        .node
        .port_id(PortDirection::Input, ChannelPosition::FrontLeft)?;
    let out_right = output
        .node
        .port_id(PortDirection::Input, ChannelPosition::FrontRight)?;

    Some(StereoLink::new(
        LinkEndpoint {
            node_id: src_node,
            port_id: src_left,
        },
        LinkEndpoint {
            node_id: out_node,
            port_id: out_left,
        },
        LinkEndpoint {
            node_id: src_node,
            port_id: src_right,
        },
        LinkEndpoint {
            node_id: out_node,
            port_id: out_right,
        },
    ))
}

const fn same_endpoints(a: &StereoLink, b: &StereoLink) -> bool {
    a.fl.out_port.node_id.0 == b.fl.out_port.node_id.0
        && a.fl.out_port.port_id.0 == b.fl.out_port.port_id.0
        && a.fl.in_port.node_id.0 == b.fl.in_port.node_id.0
        && a.fl.in_port.port_id.0 == b.fl.in_port.port_id.0
        && a.fr.out_port.node_id.0 == b.fr.out_port.node_id.0
        && a.fr.out_port.port_id.0 == b.fr.out_port.port_id.0
        && a.fr.in_port.node_id.0 == b.fr.in_port.node_id.0
        && a.fr.in_port.port_id.0 == b.fr.in_port.port_id.0
}

fn confirmed_link_ids(link: &StereoLink) -> Vec<LinkId> {
    [link.fl.id, link.fr.id].into_iter().flatten().collect()
}

fn plan_output_link_sync(
    current: &StereoLink,
    requested: bool,
    desired: Option<StereoLink>,
) -> OutputLinkPlan {
    match desired {
        Some(target) => {
            if same_endpoints(current, &target) && (requested || current.is_ready()) {
                return OutputLinkPlan {
                    destroy_ids: Vec::new(),
                    create: None,
                    next_links: *current,
                    next_requested: requested,
                };
            }
            OutputLinkPlan {
                destroy_ids: confirmed_link_ids(current),
                create: Some(target),
                next_links: target,
                next_requested: true,
            }
        },
        None => OutputLinkPlan {
            destroy_ids: confirmed_link_ids(current),
            create: None,
            next_links: StereoLink::default(),
            next_requested: false,
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use pipelink_common::{
        graph::{node::Node, port::PortInfo},
        types::PortId,
    };

    use super::*;

    fn mk_output(name: &str, desc: Option<&str>, is_hardware: bool, id: u32) -> OutputDevice {
        let mut node = Node {
            name: NodeName::from(name),
            id: Some(NodeId(id)),
            ..Default::default()
        };
        node.ports.insert(
            PortId(id * 10 + 1),
            PortInfo {
                direction: PortDirection::Input,
                position: ChannelPosition::FrontLeft,
            },
        );
        node.ports.insert(
            PortId(id * 10 + 2),
            PortInfo {
                direction: PortDirection::Input,
                position: ChannelPosition::FrontRight,
            },
        );
        OutputDevice {
            node,
            description: desc.map(ToString::to_string),
            nick: None,
            is_hardware,
        }
    }

    #[test]
    fn test_pick_selected_output_prefers_hardware_over_virtual_on_fallback() {
        let virt = mk_output("virtual.sink", Some("Virtual Sink"), false, 1);
        let hw = mk_output("alsa_output.pci", Some("Built-in Audio"), true, 2);
        let outputs = vec![virt, hw];

        let selected = pick_selected_output(&outputs, None, None);
        assert_eq!(selected, Some(NodeName::from("alsa_output.pci")));

        // Keep current virtual output if it is explicitly chosen and still present.
        let keep_virt = pick_selected_output(&outputs, Some(&NodeName::from("virtual.sink")), None);
        assert_eq!(keep_virt, Some(NodeName::from("virtual.sink")));

        // Fail over to virtual when the only hardware sink disappears.
        let only_virt = vec![outputs[0].clone()];
        let fallback =
            pick_selected_output(&only_virt, Some(&NodeName::from("alsa_output.pci")), None);
        assert_eq!(fallback, Some(NodeName::from("virtual.sink")));

        // Configured preferred device (e.g. Bluetooth headphones) reclaims focus when
        // reconnected.
        let bluetooth = mk_output("bluez_output.test", Some("Bluetooth Headphones"), true, 3);
        let with_bt = vec![outputs[0].clone(), outputs[1].clone(), bluetooth];
        let reclaimed = pick_selected_output(
            &with_bt,
            Some(&NodeName::from("alsa_output.pci")),
            Some(&NodeName::from("bluez_output.test")),
        );
        assert_eq!(reclaimed, Some(NodeName::from("bluez_output.test")));
    }

    #[test]
    fn test_resolve_output_query_supports_substrings_and_rejects_ambiguous() {
        let scarlett = mk_output(
            "alsa_output.usb-Focusrite_Scarlett.sink",
            Some("Scarlett Solo (3rd Gen.) Direct Scarlett Solo USB"),
            true,
            1,
        );
        let headset = mk_output(
            "alsa_output.usb-Logitech_G733.analog-stereo",
            Some("G733 Gaming Headset Analog Stereo"),
            true,
            2,
        );
        let speaker = mk_output(
            "alsa_output.usb-Logitech_G560.analog-stereo",
            Some("G560 Gaming Speaker Analog Stereo"),
            true,
            3,
        );
        let outputs = vec![scarlett, headset, speaker];

        assert!(resolve_output_query(&outputs, "   ").is_err());
        assert!(resolve_output_query(&outputs, "nonexistent").is_err());
        // Ambiguous substring ("logitech" matches both G733 and G560)
        assert!(resolve_output_query(&outputs, "logitech").is_err());
        // Short case-insensitive substring resolves unambiguously
        assert_eq!(
            resolve_output_query(&outputs, "g733").unwrap().node.name,
            NodeName::from("alsa_output.usb-Logitech_G733.analog-stereo")
        );
        assert_eq!(
            resolve_output_query(&outputs, "scarlett")
                .unwrap()
                .node
                .name,
            NodeName::from("alsa_output.usb-Focusrite_Scarlett.sink")
        );
    }

    #[test]
    fn test_pick_fallback_active_mix_selects_next_remaining_mix() {
        let mut graph = AudioGraph::new();
        graph.add_mix(MixName::from("stream")).unwrap();
        graph.add_mix(MixName::from("headphones")).unwrap();

        assert_eq!(
            pick_fallback_active_mix(graph.mixes(), Some(&MixName::from("headphones"))),
            Some(MixName::from("headphones"))
        );

        assert_eq!(
            pick_fallback_active_mix(graph.mixes(), Some(&MixName::from("removed"))),
            Some(MixName::from("stream"))
        );
    }

    #[test]
    fn test_plan_output_link_sync_tears_down_old_links_and_deduplicates() {
        let ep = |n, p| LinkEndpoint {
            node_id: NodeId(n),
            port_id: PortId(p),
        };
        let target_a = StereoLink::new(ep(10, 1), ep(20, 1), ep(10, 2), ep(20, 2));
        let target_b = StereoLink::new(ep(10, 1), ep(30, 1), ep(10, 2), ep(30, 2));

        let plan1 = plan_output_link_sync(&StereoLink::default(), false, Some(target_a));
        assert_eq!(plan1.destroy_ids, Vec::new());
        assert_eq!(plan1.create, Some(target_a));
        assert!(plan1.next_requested);

        // Same endpoints already requested -> no-op.
        let plan_dup = plan_output_link_sync(&plan1.next_links, true, Some(target_a));
        assert_eq!(plan_dup.destroy_ids, Vec::new());
        assert_eq!(plan_dup.create, None);

        // Confirmed links replaced when target changes.
        let mut confirmed = target_a;
        confirmed.fl.id = Some(LinkId(100));
        confirmed.fr.id = Some(LinkId(101));
        let plan_switch = plan_output_link_sync(&confirmed, true, Some(target_b));
        assert_eq!(plan_switch.destroy_ids, vec![LinkId(100), LinkId(101)]);
        assert_eq!(plan_switch.create, Some(target_b));

        // Tears down when desired becomes None.
        let plan_clear = plan_output_link_sync(&confirmed, true, None);
        assert_eq!(plan_clear.destroy_ids, vec![LinkId(100), LinkId(101)]);
        assert_eq!(plan_clear.create, None);
        assert!(!plan_clear.next_requested);
    }
}
