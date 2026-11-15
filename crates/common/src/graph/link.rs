/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::types::{ChannelPosition, LinkId, NodeId, PortId};

/// Endpoint of a `PipeWire` link identifying both node and port.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LinkEndpoint {
    pub node_id: NodeId,
    pub port_id: PortId,
}

/// A single `PipeWire` link between an output port and an input port.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Link {
    /// Assigned `PipeWire` link id once `PipeWire` confirms creation.
    pub id: Option<LinkId>,
    /// Output (source) endpoint.
    pub out_port: LinkEndpoint,
    /// Input (sink) endpoint.
    pub in_port: LinkEndpoint,
}

/// A front-left / front-right pair of `PipeWire` port IDs (e.g. for `JACK` ports).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StereoPortPair {
    pub fl: Option<PortId>,
    pub fr: Option<PortId>,
}

impl StereoPortPair {
    /// Returns `true` when both `FL` and `FR` port IDs are known.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.fl.is_some() && self.fr.is_some()
    }

    /// Stores `port_id` for `FrontLeft` or `FrontRight`, returning `true` if updated.
    pub fn set_port(&mut self, position: ChannelPosition, port_id: PortId) -> bool {
        match position {
            ChannelPosition::FrontLeft => {
                self.fl = Some(port_id);
                true
            },
            ChannelPosition::FrontRight => {
                self.fr = Some(port_id);
                true
            },
            _ => false,
        }
    }
}

/// A stereo pair of [`Link`]s (front-left and front-right).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StereoLink {
    pub fl: Link,
    pub fr: Link,
}

impl StereoLink {
    /// Builds a [`StereoLink`] specification from matching left/right endpoint pairs.
    #[must_use]
    pub const fn new(
        left_out: LinkEndpoint,
        left_in: LinkEndpoint,
        right_out: LinkEndpoint,
        right_in: LinkEndpoint,
    ) -> Self {
        Self {
            fl: Link {
                id: None,
                out_port: left_out,
                in_port: left_in,
            },
            fr: Link {
                id: None,
                out_port: right_out,
                in_port: right_in,
            },
        }
    }

    /// Returns `true` when both `FL` and `FR` links have confirmed `PipeWire` IDs.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.fl.id.is_some() && self.fr.id.is_some()
    }

    /// Assigns `id` to whichever half (`FL` or `FR`) matches `(out_port, in_port)`.
    pub fn match_appeared(
        &mut self,
        id: LinkId,
        out_port: LinkEndpoint,
        in_port: LinkEndpoint,
    ) -> bool {
        for half in [&mut self.fl, &mut self.fr] {
            if half.out_port == out_port && half.in_port == in_port {
                half.id = Some(id);
                return true;
            }
        }
        false
    }

    /// Clears the stored link ID matching `id`, returning `true` if one matched.
    pub fn clear_link_id(&mut self, id: LinkId) -> bool {
        for half in [&mut self.fl, &mut self.fr] {
            if half.id == Some(id) {
                half.id = None;
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stereo_link_match_and_clear() {
        let ep = |node, port| LinkEndpoint {
            node_id: NodeId(node),
            port_id: PortId(port),
        };
        let mut links = StereoLink::new(ep(1, 10), ep(2, 20), ep(1, 11), ep(2, 21));

        assert!(!links.is_ready());
        assert!(!links.match_appeared(LinkId(99), ep(1, 10), ep(2, 99)));
        assert!(links.match_appeared(LinkId(100), ep(1, 10), ep(2, 20)));
        assert!(!links.is_ready());
        assert!(links.match_appeared(LinkId(101), ep(1, 11), ep(2, 21)));
        assert!(links.is_ready());

        assert!(links.clear_link_id(LinkId(100)));
        assert!(!links.is_ready());
        assert!(!links.clear_link_id(LinkId(100)));
    }

    #[test]
    fn test_stereo_port_pair_set_and_complete() {
        let mut pair = StereoPortPair::default();
        assert!(!pair.is_complete());
        assert!(!pair.set_port(ChannelPosition::Mono, PortId(1)));
        assert!(pair.set_port(ChannelPosition::FrontLeft, PortId(2)));
        assert!(!pair.is_complete());
        assert!(pair.set_port(ChannelPosition::FrontRight, PortId(3)));
        assert!(pair.is_complete());
    }
}
