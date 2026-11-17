/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use pipelink_common::{
    events::{
        ActiveAppInfo, ChannelVolumeInfo, ControlEvent, ControlReply, GraphSnapshot, MixRouteInfo,
        OutputDeviceInfo,
    },
    graph::audio_graph::AudioGraph,
    types::{ChannelName, MixName},
};

use crate::{
    config::{state::linear_to_percent, topology::ConfigGraph},
    executor::{Envelope, Executor},
};

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
            ControlEvent::ListAppChannels => {
                Self::reply(
                    reply,
                    ControlReply::AppChannels {
                        routes: self.config.app_routes(),
                    },
                );
                None
            },
            ControlEvent::ListApps => {
                Self::reply(
                    reply,
                    ControlReply::Apps {
                        apps: collect_active_apps(&self.graph),
                    },
                );
                None
            },
            ControlEvent::GetActiveMix => {
                Self::reply(
                    reply,
                    ControlReply::ActiveMix {
                        mix: self.graph.active_mix.clone(),
                    },
                );
                None
            },
            ControlEvent::ListOutputs => {
                Self::reply(
                    reply,
                    ControlReply::Outputs {
                        outputs: collect_outputs(&self.graph),
                    },
                );
                None
            },
            ControlEvent::GetGraphSnapshot => {
                Self::reply(
                    reply,
                    ControlReply::GraphSnapshot(build_graph_snapshot(&self.graph, &self.config)),
                );
                None
            },
            ControlEvent::ListInstalledApps => {
                Self::reply(
                    reply,
                    ControlReply::InstalledApps {
                        apps: self.resolver.installed_apps(),
                    },
                );
                None
            },
            other => Some(other),
        }
    }
}

fn collect_active_apps(graph: &AudioGraph) -> Vec<ActiveAppInfo> {
    graph
        .apps
        .iter()
        .map(|a| ActiveAppInfo {
            display_name: a.app.display_name.clone(),
            app_name: a.app.app_name.clone(),
            channel: a.channel.clone(),
            stream_count: a.nodes.len(),
        })
        .collect()
}

fn collect_outputs(graph: &AudioGraph) -> Vec<OutputDeviceInfo> {
    let active = graph.selected_output.as_ref();
    graph
        .outputs
        .iter()
        .map(|o| OutputDeviceInfo {
            name: o.node.name.clone(),
            description: o.description.clone(),
            nick: o.nick.clone(),
            is_hardware: o.is_hardware,
            active: active == Some(&o.node.name),
        })
        .collect()
}

fn collect_mix_routes(graph: &AudioGraph) -> Vec<MixRouteInfo> {
    graph
        .routes()
        .iter()
        .map(|r| MixRouteInfo {
            channel: r.channel.clone(),
            mix: r.mix.clone(),
            active: r.active,
            gain_percent: linear_to_percent(r.gain),
            muted: r.muted,
        })
        .collect()
}

fn build_graph_snapshot(graph: &AudioGraph, config: &ConfigGraph) -> GraphSnapshot {
    GraphSnapshot {
        channels: graph.channels().iter().map(|c| c.name.clone()).collect(),
        channel_volumes: graph
            .channels()
            .iter()
            .map(|c| ChannelVolumeInfo {
                channel: c.name.clone(),
                volume_percent: c.volume_percent,
                muted: c.muted,
            })
            .collect(),
        mixes: graph.mixes().iter().map(|m| m.name.clone()).collect(),
        active_mix: graph.active_mix.clone(),
        default_sink: graph.default_sink.clone(),
        routes: collect_mix_routes(graph),
        outputs: collect_outputs(graph),
        apps: collect_active_apps(graph),
        app_routes: config.app_routes(),
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

    #[test]
    fn test_build_graph_snapshot_includes_routes_gains_and_mutes() {
        let mut graph = AudioGraph::new();
        graph.add_channel(ChannelName::from("Music")).unwrap();
        graph.add_mix(MixName::from("Personal")).unwrap();
        graph.active_mix = Some(MixName::from("Personal"));
        let route = graph
            .get_route_mut(&ChannelName::from("Music"), &MixName::from("Personal"))
            .unwrap();
        route.active = true;
        route.gain = 0.5;
        route.muted = true;

        let config = ConfigGraph::default();
        let snapshot = build_graph_snapshot(&graph, &config);
        assert_eq!(snapshot.channels, vec![ChannelName::from("Music")]);
        assert_eq!(snapshot.mixes, vec![MixName::from("Personal")]);
        assert_eq!(snapshot.active_mix, Some(MixName::from("Personal")));
        assert_eq!(snapshot.routes.len(), 1);
        assert!(snapshot.routes[0].active);
        assert!((snapshot.routes[0].gain_percent - 50.0).abs() < f32::EPSILON);
        assert!(snapshot.routes[0].muted);
    }
}
