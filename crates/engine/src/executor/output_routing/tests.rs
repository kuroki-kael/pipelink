/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use pipelink_common::{
    graph::{
        audio_graph::AudioGraph,
        link::{LinkEndpoint, StereoLink},
        node::Node,
        output::OutputDevice,
        port::PortInfo,
    },
    types::{ChannelPosition, LinkId, MixName, NodeId, NodeName, PortDirection, PortId},
};

use super::planner::*;

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
    let fallback = pick_selected_output(&only_virt, Some(&NodeName::from("alsa_output.pci")), None);
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

#[test]
fn test_resolve_default_sink_target_supports_channels_and_outputs() {
    let mut graph = AudioGraph::new();
    graph
        .add_channel(pipelink_common::types::ChannelName::from("System"))
        .unwrap();
    // Before PipeWire node registration, resolving a channel errors cleanly.
    assert!(resolve_default_sink_target(&graph, "system").is_err());

    graph.channels_mut()[0].node.id = Some(NodeId(42));
    graph.channels_mut()[0].node.name = NodeName::from("pipelink.channel.System");
    graph.outputs.push(mk_output(
        "alsa_output.usb.g733",
        Some("G733 Gaming Headset"),
        true,
        10,
    ));

    assert_eq!(
        resolve_default_sink_target(&graph, "system").unwrap(),
        NodeName::from("pipelink.channel.System")
    );
    assert_eq!(
        resolve_default_sink_target(&graph, "pipelink.channel.system").unwrap(),
        NodeName::from("pipelink.channel.System")
    );
    assert_eq!(
        resolve_default_sink_target(&graph, "g733").unwrap(),
        NodeName::from("alsa_output.usb.g733")
    );
    assert!(resolve_default_sink_target(&graph, "   ").is_err());
}
