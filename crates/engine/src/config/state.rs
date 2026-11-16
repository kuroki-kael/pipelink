/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Runtime mixer state persisted separately from static routing topology.

use pipelink_common::{
    graph::audio_graph::AudioGraph,
    types::{ChannelName, MixName},
};
use serde::{Deserialize, Serialize};

/// Upper limit for user-configured channel-to-mix gain percentages (+12 dB).
pub(crate) const MAX_GAIN_PERCENT: f32 = 400.0;

const fn default_gain_percent() -> f32 {
    100.0
}

/// Clamps non-finite or negative percentages to `0.0` and caps at `MAX_GAIN_PERCENT`.
#[inline]
#[must_use]
pub(crate) fn normalize_percent(percent: f32) -> f32 {
    if !percent.is_finite() || percent <= 0.0 {
        0.0
    } else {
        percent.min(MAX_GAIN_PERCENT)
    }
}

/// Converts a normalized percentage (`0.0` = silence, `100.0` = unity) to a linear
/// multiplier.
#[inline]
#[must_use]
pub(crate) fn percent_to_linear(percent: f32) -> f32 {
    normalize_percent(percent) / 100.0
}

/// Converts a linear gain multiplier back into a normalized percentage.
#[inline]
#[must_use]
pub(crate) fn linear_to_percent(linear: f32) -> f32 {
    normalize_percent(linear * 100.0)
}

/// Persisted mixer cell parameters for a `(channel, mix)` pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RouteState {
    /// Source channel name.
    pub channel: ChannelName,
    /// Target mix name.
    pub mix: MixName,
    /// Gain percentage (`100.0` = unity, clamped to `0.0..=400.0`).
    #[serde(default = "default_gain_percent")]
    pub gain_percent: f32,
    /// Whether this route is muted.
    #[serde(default)]
    pub muted: bool,
}

/// Volatile runtime state stored separately from `ConfigGraph`
/// (`$XDG_STATE_HOME/pipelink/`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeState {
    /// Currently selected active mix driving the audio output.
    #[serde(default)]
    pub active_mix: Option<MixName>,
    /// Non-default gain or mute settings per `(channel, mix)` route.
    #[serde(default)]
    pub routes: Vec<RouteState>,
}

impl RuntimeState {
    /// Extracts the current runtime state from `graph`, omitting default cells
    /// (`gain == 1.0 && !muted`) to keep the state file concise.
    #[must_use]
    pub fn from_graph(graph: &AudioGraph) -> Self {
        let routes = graph
            .routes()
            .iter()
            .filter(|r| (r.gain - 1.0).abs() > f32::EPSILON || r.muted)
            .map(|r| RouteState {
                channel: r.channel.clone(),
                mix: r.mix.clone(),
                gain_percent: linear_to_percent(r.gain),
                muted: r.muted,
            })
            .collect();

        Self {
            active_mix: graph.active_mix.clone(),
            routes,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_to_linear_handles_unity_attenuation_and_silence() {
        assert!((percent_to_linear(100.0) - 1.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(50.0) - 0.5).abs() < f32::EPSILON);
        assert!((percent_to_linear(150.0) - 1.5).abs() < f32::EPSILON);
        assert!((percent_to_linear(400.0) - 4.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(500.0) - 4.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(-10.0) - 0.0).abs() < f32::EPSILON);
        assert!((percent_to_linear(f32::NAN) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_runtime_state_from_graph_skips_default_cells() {
        let mut graph = AudioGraph::new();
        let music = ChannelName::from("Music");
        let game = ChannelName::from("Game");
        let stream = MixName::from("Stream");
        graph.add_channel(music.clone()).unwrap();
        graph.add_channel(game).unwrap();
        graph.add_mix(stream.clone()).unwrap();
        graph.active_mix = Some(stream.clone());

        let r = graph.get_route_mut(&music, &stream).unwrap();
        r.gain = 0.5;
        r.muted = true;

        let state = RuntimeState::from_graph(&graph);
        assert_eq!(state.active_mix, Some(stream.clone()));
        assert_eq!(state.routes.len(), 1);
        assert_eq!(state.routes[0].channel, music);
        assert_eq!(state.routes[0].mix, stream);
        assert!((state.routes[0].gain_percent - 50.0).abs() < f32::EPSILON);
        assert!(state.routes[0].muted);
    }
}
