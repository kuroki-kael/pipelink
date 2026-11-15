/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use pipelink_common::{
    events::{ControlEvent, ControlReply},
    graph::audio_graph::AudioGraph,
    types::{ChannelName, MixName},
};

use crate::executor::{Envelope, Executor};

impl Executor {
    /// Answers a query from the graph and returns the event if it was no query.
    pub(super) fn handle_query(&mut self, envelope: Envelope) -> Option<ControlEvent> {
        let Envelope { event, reply } = envelope;
        match event {
            ControlEvent::ListChannels => {
                Self::reply(
                    reply,
                    ControlReply::Channels {
                        names: self
                            .graph
                            .channels()
                            .iter()
                            .map(|c| c.name.clone())
                            .collect(),
                    },
                );
                None
            },
            ControlEvent::ListMixes => {
                Self::reply(
                    reply,
                    ControlReply::Mixes {
                        names: self.graph.mixes().iter().map(|m| m.name.clone()).collect(),
                    },
                );
                None
            },
            ControlEvent::ListMixChannels => {
                Self::reply(
                    reply,
                    ControlReply::MixChannels {
                        channels: collect_mix_channels(&self.graph),
                    },
                );
                None
            },
            // The remaining queries report on data models that do not exist yet.
            // TODO: answer ListAppChannels once the graph knows which app is routed to which channel.
            // TODO: answer GetActiveMix once a mix can be selected as active.
            other => Some(other),
        }
    }
}

fn collect_mix_channels(graph: &AudioGraph) -> Vec<(MixName, Vec<ChannelName>)> {
    graph
        .mixes()
        .iter()
        .map(|mix| {
            let routed = graph
                .routes()
                .iter()
                .filter(|r| r.mix == mix.name && r.active)
                .map(|r| r.channel.clone())
                .collect();
            (mix.name.clone(), routed)
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_mix_channels_lists_only_active_routes_per_mix() {
        let mut graph = AudioGraph::new();
        graph.add_channel(ChannelName::from("Game")).unwrap();
        graph.add_channel(ChannelName::from("Music")).unwrap();
        graph.add_mix(MixName::from("Personal")).unwrap();
        graph.add_mix(MixName::from("Stream")).unwrap();

        graph
            .get_route_mut(&ChannelName::from("Game"), &MixName::from("Personal"))
            .unwrap()
            .active = true;
        graph
            .get_route_mut(&ChannelName::from("Music"), &MixName::from("Stream"))
            .unwrap()
            .active = true;

        let result = collect_mix_channels(&graph);
        assert_eq!(
            result,
            vec![
                (MixName::from("Personal"), vec![ChannelName::from("Game")]),
                (MixName::from("Stream"), vec![ChannelName::from("Music")]),
            ]
        );
    }
}
