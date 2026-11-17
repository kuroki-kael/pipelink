/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Startup restoration of persisted [`ConfigGraph`] topology and [`RuntimeState`].

use pipelink_common::{
    error::ExecutorError,
    graph::audio_graph::AudioGraph,
    types::{ChannelName, MixName},
};
use pipelink_jack::JackError;
use pipelink_pw::Command as PipewireCommand;
use tracing::{info, warn};

use crate::{
    config::{
        state::{RuntimeState, normalize_channel_percent, percent_to_linear},
        topology::ConfigGraph,
    },
    executor::Executor,
};

impl Executor {
    /// Applies persisted channels, mixes, channel-to-mix routes, gains, mutes,
    /// and the last active mix without rewriting `config.json` during startup.
    pub(super) fn bootstrap(&mut self, state: &RuntimeState) -> Result<(), ExecutorError> {
        let channels: Vec<ChannelName> = self
            .config
            .channels
            .iter()
            .map(|c| c.name.clone())
            .collect();
        for name in channels {
            if let Err(err) = self.graph.add_channel(name.clone()) {
                warn!("Skipping duplicate channel '{name}' in config: {err}");
                continue;
            }
            if let Err(err) = self.jack.create_channel(&name, self.graph.gains_snapshot()) {
                self.graph.remove_channel(&name);
                if matches!(err, JackError::ChannelClosed) {
                    return Err(ExecutorError::Jack(err.to_string()));
                }
                warn!("Failed to restore JACK channel '{name}': {err}");
                continue;
            }
            if let Err(err) = self
                .pw_tx
                .send(PipewireCommand::CreateChannel { name: name.clone() })
            {
                self.graph.remove_channel(&name);
                let _ = self.jack.remove_channel(&name, self.graph.gains_snapshot());
                return Err(ExecutorError::Pipewire(format!("{err:?}")));
            }
            info!("Restored channel '{name}' from config");
        }

        let mixes: Vec<MixName> = self.config.mixes.iter().map(|m| m.name.clone()).collect();
        for name in mixes {
            if let Err(err) = self.graph.add_mix(name.clone()) {
                warn!("Skipping duplicate mix '{name}' in config: {err}");
                continue;
            }
            if let Err(err) = self.jack.create_mix(&name, self.graph.gains_snapshot()) {
                self.graph.remove_mix(&name);
                if matches!(err, JackError::ChannelClosed) {
                    return Err(ExecutorError::Jack(err.to_string()));
                }
                warn!("Failed to restore JACK mix '{name}': {err}");
                continue;
            }
            if let Err(err) = self
                .pw_tx
                .send(PipewireCommand::CreateMix { name: name.clone() })
            {
                self.graph.remove_mix(&name);
                let _ = self.jack.remove_mix(&name, self.graph.gains_snapshot());
                return Err(ExecutorError::Pipewire(format!("{err:?}")));
            }
            info!("Restored mix '{name}' from config");
        }

        let initial_active = apply_persisted_routes(&self.config, state, &mut self.graph);
        self.jack
            .sync_gains(self.graph.gains_snapshot())
            .map_err(|e| ExecutorError::Jack(e.to_string()))?;

        self.graph.active_mix = initial_active.clone();
        self.config.active_mix = initial_active;

        Ok(())
    }
}

/// Applies configured active `channel -> mix` routes and volatile `RuntimeState`
/// gains/mutes onto `graph`, returning the initial `active_mix`.
pub(super) fn apply_persisted_routes(
    config: &ConfigGraph,
    state: &RuntimeState,
    graph: &mut AudioGraph,
) -> Option<MixName> {
    for mix in &config.mixes {
        for channel in &mix.channels {
            if let Some(route) = graph.get_route_mut(channel, &mix.name) {
                route.active = true;
            } else {
                warn!(
                    "Ignoring unknown route ({channel} -> {}) in config",
                    mix.name
                );
            }
        }
    }

    for cell in &state.routes {
        if let Some(route) = graph.get_route_mut(&cell.channel, &cell.mix) {
            route.gain = percent_to_linear(cell.gain_percent);
            route.muted = cell.muted;
        }
    }

    for ch_state in &state.channels {
        if let Some(ch) = graph
            .channels_mut()
            .iter_mut()
            .find(|c| c.name.eq_ignore_ascii_case(&ch_state.channel))
        {
            ch.volume_percent = normalize_channel_percent(ch_state.volume_percent);
            ch.muted = ch_state.muted;
        }
    }

    state
        .active_mix
        .as_ref()
        .and_then(|m| graph.resolve_mix_name(m))
        .or_else(|| {
            config
                .active_mix
                .as_ref()
                .and_then(|m| graph.resolve_mix_name(m))
        })
        .or_else(|| graph.mixes().first().map(|m| m.name.clone()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::config::state::RouteState;

    #[test]
    fn test_apply_persisted_routes_restores_topology_gains_mutes_and_active_mix() {
        let mut cfg = ConfigGraph::new();
        let music = ChannelName::from("Music");
        let voice = ChannelName::from("Voice");
        let stream = MixName::from("Stream");
        let personal = MixName::from("Personal");
        cfg.ensure_channel(&music);
        cfg.ensure_channel(&voice);
        cfg.ensure_mix(&stream);
        cfg.ensure_mix(&personal);
        cfg.set_channel_mix(&music, &stream);

        let mut graph = AudioGraph::new();
        graph.add_channel(music.clone()).unwrap();
        graph.add_channel(voice.clone()).unwrap();
        graph.add_mix(stream.clone()).unwrap();
        graph.add_mix(personal.clone()).unwrap();

        let state = RuntimeState {
            active_mix: Some(MixName::from("personal")),
            routes: vec![RouteState {
                channel: ChannelName::from("music"),
                mix: MixName::from("stream"),
                gain_percent: 50.0,
                muted: true,
            }],
            ..Default::default()
        };

        let active = apply_persisted_routes(&cfg, &state, &mut graph);
        assert_eq!(active, Some(personal));

        let restored = graph.get_route(&music, &stream).unwrap();
        assert!(restored.active);
        assert!((restored.gain - 0.5).abs() < f32::EPSILON);
        assert!(restored.muted);

        let untouched = graph.get_route(&voice, &stream).unwrap();
        assert!(!untouched.active);
        assert!((untouched.gain - 1.0).abs() < f32::EPSILON);
        assert!(!untouched.muted);
    }
}
