/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Physical or virtual audio output sink discovered in `PipeWire`.

use crate::{
    graph::node::Node,
    types::{ChannelPosition, PortDirection},
};

/// An available `Audio/Sink` node in `PipeWire` that the active mix can drive.
#[derive(Debug, Clone, Default)]
pub struct OutputDevice {
    /// Underlying `PipeWire` node metadata and discovered ports.
    pub node: Node,
    /// Human-readable device name from `node.description`, if set.
    pub description: Option<String>,
    /// Short device nickname from `node.nick`, if set.
    pub nick: Option<String>,
    /// `true` when backed by a `PipeWire` device (`device.id` present).
    pub is_hardware: bool,
}

impl OutputDevice {
    /// Preferred label for UI/CLI display (`description`, `nick`, or `node.name`).
    #[must_use]
    pub fn display_name(&self) -> &str {
        preferred_output_label(
            self.description.as_deref(),
            self.nick.as_deref(),
            &self.node.name,
        )
    }

    /// Returns `true` once both front-left and front-right input ports are registered.
    #[must_use]
    pub fn are_pw_ports_ready(&self) -> bool {
        self.node
            .has_port(PortDirection::Input, ChannelPosition::FrontLeft)
            && self
                .node
                .has_port(PortDirection::Input, ChannelPosition::FrontRight)
    }

    /// Returns `true` if `query` matches the node name, description, or nick
    /// case-insensitively as an exact string.
    #[must_use]
    pub fn matches_exact_query(&self, query: &str) -> bool {
        let q = query.trim();
        if q.is_empty() {
            return false;
        }
        self.node.name.eq_ignore_ascii_case(q)
            || self
                .description
                .as_deref()
                .is_some_and(|d| d.eq_ignore_ascii_case(q))
            || self
                .nick
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case(q))
    }

    /// Returns `true` if `query` appears case-insensitively anywhere in the
    /// node name, description, or nick.
    #[must_use]
    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.trim();
        if q.is_empty() {
            return false;
        }
        let q_lower = q.to_lowercase();
        self.node.name.to_lowercase().contains(&q_lower)
            || self
                .description
                .as_deref()
                .is_some_and(|d| d.to_lowercase().contains(&q_lower))
            || self
                .nick
                .as_deref()
                .is_some_and(|n| n.to_lowercase().contains(&q_lower))
    }
}

/// Returns the preferred non-empty display label among `description`, `nick`, and
/// `fallback_name`.
#[must_use]
pub fn preferred_output_label<'a>(
    description: Option<&'a str>,
    nick: Option<&'a str>,
    fallback_name: &'a str,
) -> &'a str {
    description
        .filter(|s| !s.is_empty())
        .or_else(|| nick.filter(|s| !s.is_empty()))
        .unwrap_or(fallback_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        graph::port::PortInfo,
        types::{NodeId, NodeName, PortId},
    };

    fn sample_output() -> OutputDevice {
        OutputDevice {
            node: Node {
                name: NodeName::from("alsa_output.pci-0000.analog-stereo"),
                id: Some(NodeId(42)),
                ..Default::default()
            },
            description: Some("Built-in Audio Analog Stereo".to_string()),
            nick: Some("ALC897 Analog".to_string()),
            is_hardware: true,
        }
    }

    #[test]
    fn test_display_name_prefers_description_then_nick_then_node_name() {
        let mut dev = sample_output();
        assert_eq!(dev.display_name(), "Built-in Audio Analog Stereo");

        dev.description = Some(String::new());
        assert_eq!(dev.display_name(), "ALC897 Analog");

        dev.nick = None;
        assert_eq!(dev.display_name(), "alsa_output.pci-0000.analog-stereo");
    }

    #[test]
    fn test_matches_query_checks_node_name_description_and_nick() {
        let dev = sample_output();
        assert!(dev.matches_exact_query("alsa_output.pci-0000.analog-stereo"));
        assert!(dev.matches_exact_query("built-in audio analog stereo"));
        assert!(dev.matches_exact_query("alc897 analog"));
        assert!(!dev.matches_exact_query("alc897"));

        assert!(dev.matches_query("alc897"));
        assert!(dev.matches_query("built-in"));
        assert!(dev.matches_query("pci-0000"));
        assert!(!dev.matches_query("hdmi"));
        assert!(!dev.matches_query("   "));
    }

    #[test]
    fn test_are_pw_ports_ready_requires_stereo_input_ports() {
        let mut dev = sample_output();
        assert!(!dev.are_pw_ports_ready());

        dev.node.ports.insert(
            PortId(1),
            PortInfo {
                direction: PortDirection::Input,
                position: ChannelPosition::FrontLeft,
            },
        );
        assert!(!dev.are_pw_ports_ready());

        dev.node.ports.insert(
            PortId(2),
            PortInfo {
                direction: PortDirection::Input,
                position: ChannelPosition::FrontRight,
            },
        );
        assert!(dev.are_pw_ports_ready());
    }
}
