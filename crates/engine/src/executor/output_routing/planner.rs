/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Pure resolution and link-planning helpers for active-mix and output routing.

use pipelink_common::{
    graph::{
        audio_graph::AudioGraph,
        link::{LinkEndpoint, StereoLink},
        mix::Mix,
        output::OutputDevice,
    },
    types::{ChannelPosition, LinkId, MixName, NodeName, PortDirection},
};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct OutputLinkPlan {
    pub(super) destroy_ids: Vec<LinkId>,
    pub(super) create: Option<StereoLink>,
    pub(super) next_links: StereoLink,
    pub(super) next_requested: bool,
}

pub(super) fn resolve_output_query<'a>(
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

pub(super) fn resolve_default_sink_target(
    graph: &AudioGraph,
    query: &str,
) -> Result<NodeName, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err("default sink query cannot be empty".to_string());
    }

    if let Some(channel) = graph.channels().iter().find(|ch| {
        ch.name.eq_ignore_ascii_case(trimmed)
            || format!("pipelink.channel.{}", ch.name).eq_ignore_ascii_case(trimmed)
    }) {
        if channel.node.id.is_none() {
            return Err(format!(
                "channel '{}' is not registered in PipeWire yet",
                channel.name
            ));
        }
        return Ok(channel.node.name.clone());
    }

    resolve_output_query(&graph.outputs, trimmed).map(|dev| dev.node.name.clone())
}

pub(super) fn pick_selected_output(
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

pub(super) fn pick_fallback_active_mix(
    mixes: &[Mix],
    current: Option<&MixName>,
) -> Option<MixName> {
    if let Some(name) = current
        && mixes.iter().any(|m| m.name == *name)
    {
        return Some(name.clone());
    }
    mixes.first().map(|m| m.name.clone())
}

pub(super) fn desired_output_link(graph: &AudioGraph) -> Option<StereoLink> {
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

pub(super) fn plan_output_link_sync(
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
