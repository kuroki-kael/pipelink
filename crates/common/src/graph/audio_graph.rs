/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::{
    error::ExecutorError,
    graph::{
        app_node::AppNode, channel::Channel, link::StereoLink, mix::Mix, output::OutputDevice,
    },
    types::{ChannelName, MixName, NodeId, NodeName},
};

/// Routing entry pairing a single channel with a single mix in the N:M matrix.
#[derive(Clone, Debug, PartialEq)]
pub struct ChannelMix {
    /// Source channel of this route.
    pub channel: ChannelName,
    /// Destination mix of this route.
    pub mix: MixName,
    /// Whether audio is currently routed from `channel` into `mix`.
    pub active: bool,
    /// Linear gain multiplier applied to this route (`1.0` = unity gain).
    pub gain: f32,
    /// Whether this route is muted.
    pub muted: bool,
}

impl Default for ChannelMix {
    fn default() -> Self {
        Self {
            channel: ChannelName::default(),
            mix: MixName::default(),
            active: false,
            gain: 1.0,
            muted: false,
        }
    }
}

impl ChannelMix {
    /// Creates an inactive, unmuted routing entry at unity gain (`1.0`).
    pub fn new(channel: ChannelName, mix: MixName) -> Self {
        Self {
            channel,
            mix,
            ..Default::default()
        }
    }

    /// Returns the effective linear gain (`0.0` when inactive or muted).
    #[inline]
    #[must_use]
    pub fn effective_gain(&self) -> f32 {
        if self.active && !self.muted {
            self.gain
        } else {
            0.0
        }
    }
}

/// In-memory state of the audio routing graph.
///
/// Owned by the executor.
#[derive(Clone, Debug, Default)]
pub struct AudioGraph {
    channels: Vec<Channel>,
    mixes: Vec<Mix>,
    /// Currently selected active mix, if any.
    pub active_mix: Option<MixName>,
    /// Available audio output sinks discovered in `PipeWire`.
    pub outputs: Vec<OutputDevice>,
    /// Currently selected output sink node name, if any.
    pub selected_output: Option<NodeName>,
    /// Stereo link pair connecting the active mix source to the selected output.
    pub active_output_links: StereoLink,
    /// Guards against dispatching duplicate output link requests while awaiting `PipeWire`.
    pub output_links_requested: bool,
    /// `PipeWire` node id of the `PipeLink` `JACK` client, once discovered.
    pub jack_node_id: Option<NodeId>,
    /// Active stereo links between `PipeWire` nodes.
    pub links: Vec<StereoLink>,
    /// Resolved application stream nodes currently tracked in the graph.
    pub apps: Vec<AppNode>,
    ch_mix_matrix: Vec<ChannelMix>,
}

impl AudioGraph {
    /// Creates an empty audio graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a slice of all channels in the graph.
    pub fn channels(&self) -> &[Channel] {
        &self.channels
    }

    /// Returns a mutable slice of all channels in the graph.
    pub fn channels_mut(&mut self) -> &mut [Channel] {
        &mut self.channels
    }

    /// Returns a slice of all mixes in the graph.
    pub fn mixes(&self) -> &[Mix] {
        &self.mixes
    }

    /// Returns a mutable slice of all mixes in the graph.
    pub fn mixes_mut(&mut self) -> &mut [Mix] {
        &mut self.mixes
    }

    /// Returns all cells of the row-major `channels × mixes` routing matrix.
    pub fn routes(&self) -> &[ChannelMix] {
        &self.ch_mix_matrix
    }

    /// Returns the row index of `channel` (case-insensitive), if present.
    pub fn channel_index(&self, channel: &ChannelName) -> Option<usize> {
        self.channels
            .iter()
            .position(|c| c.name == *channel)
            .or_else(|| {
                self.channels
                    .iter()
                    .position(|c| c.name.eq_ignore_ascii_case(channel))
            })
    }

    /// Returns the column index of `mix` (case-insensitive), if present.
    pub fn mix_index(&self, mix: &MixName) -> Option<usize> {
        self.mixes
            .iter()
            .position(|m| m.name == *mix)
            .or_else(|| {
                self.mixes
                    .iter()
                    .position(|m| m.name.eq_ignore_ascii_case(mix))
            })
    }

    /// Resolves `channel` case-insensitively to its stored canonical name.
    #[must_use]
    pub fn resolve_channel_name(&self, channel: &ChannelName) -> Option<ChannelName> {
        self.channel_index(channel)
            .map(|idx| self.channels[idx].name.clone())
    }

    /// Resolves `mix` case-insensitively to its stored canonical name.
    #[must_use]
    pub fn resolve_mix_name(&self, mix: &MixName) -> Option<MixName> {
        self.mix_index(mix).map(|idx| self.mixes[idx].name.clone())
    }

    /// Adds a new channel and appends its routing row across all existing mixes.
    ///
    /// # Errors
    /// Returns [`ExecutorError::ChannelExists`] if a channel with the same name
    /// (case-insensitive) already exists in the graph.
    pub fn add_channel(&mut self, channel: ChannelName) -> Result<(), ExecutorError> {
        if self
            .channels
            .iter()
            .any(|c| c.name.eq_ignore_ascii_case(&channel))
        {
            return Err(ExecutorError::ChannelExists(channel));
        }

        let new_cells = self
            .mixes
            .iter()
            .map(|mix| ChannelMix::new(channel.clone(), mix.name.clone()));

        self.ch_mix_matrix.extend(new_cells);
        self.channels.push(Channel::new(channel));
        Ok(())
    }

    /// Removes the channel with the given name and its matrix row, if present.
    pub fn remove_channel(&mut self, channel: &ChannelName) -> Option<Channel> {
        let idx = self.channel_index(channel)?;
        self.remove_channel_idx(idx)
    }

    /// Removes the channel at `channel_idx` and its matrix row, if in bounds.
    pub fn remove_channel_idx(&mut self, channel_idx: usize) -> Option<Channel> {
        if channel_idx >= self.channels.len() {
            return None;
        }

        let num_mixes = self.mixes.len();
        let row_start = channel_idx * num_mixes;
        let row_end = row_start + num_mixes;

        self.ch_mix_matrix.drain(row_start..row_end);
        Some(self.channels.remove(channel_idx))
    }

    /// Adds a new mix and inserts its routing column across all existing channels.
    ///
    /// # Errors
    /// Returns [`ExecutorError::MixExists`] if a mix with the same name
    /// (case-insensitive) already exists in the graph.
    pub fn add_mix(&mut self, mix: MixName) -> Result<(), ExecutorError> {
        if self.mixes.iter().any(|m| m.name.eq_ignore_ascii_case(&mix)) {
            return Err(ExecutorError::MixExists(mix));
        }

        let old_cols = self.mixes.len();
        let new_cols = old_cols + 1;
        let num_channels = self.channels.len();

        let mut new_matrix = Vec::with_capacity(num_channels * new_cols);
        for (row, channel) in self.channels.iter().enumerate() {
            let start = row * old_cols;
            let end = start + old_cols;
            new_matrix.extend_from_slice(&self.ch_mix_matrix[start..end]);
            new_matrix.push(ChannelMix::new(channel.name.clone(), mix.clone()));
        }

        self.ch_mix_matrix = new_matrix;
        self.mixes.push(Mix::new(mix));
        Ok(())
    }

    /// Removes the mix with the given name and its matrix column, if present.
    pub fn remove_mix(&mut self, mix: &MixName) -> Option<Mix> {
        let idx = self.mix_index(mix)?;
        self.remove_mix_idx(idx)
    }

    /// Removes the mix at `mix_idx` and its matrix column, if in bounds.
    pub fn remove_mix_idx(&mut self, mix_idx: usize) -> Option<Mix> {
        if mix_idx >= self.mixes.len() {
            return None;
        }

        let old_cols = self.mixes.len();
        let num_channels = self.channels.len();

        for row in (0..num_channels).rev() {
            self.ch_mix_matrix.remove(row * old_cols + mix_idx);
        }

        Some(self.mixes.remove(mix_idx))
    }

    #[inline]
    fn matrix_index(&self, channel_idx: usize, mix_idx: usize) -> Option<usize> {
        if channel_idx < self.channels.len() && mix_idx < self.mixes.len() {
            Some(channel_idx * self.mixes.len() + mix_idx)
        } else {
            None
        }
    }

    /// Returns a shared reference to the routing cell for `(channel, mix)`.
    pub fn get_route(&self, channel: &ChannelName, mix: &MixName) -> Option<&ChannelMix> {
        let channel_idx = self.channel_index(channel)?;
        let mix_idx = self.mix_index(mix)?;

        let idx = self.matrix_index(channel_idx, mix_idx)?;
        self.ch_mix_matrix.get(idx)
    }

    /// Returns a mutable reference to the routing cell for `(channel, mix)`.
    pub fn get_route_mut(
        &mut self,
        channel: &ChannelName,
        mix: &MixName,
    ) -> Option<&mut ChannelMix> {
        let channel_idx = self.channel_index(channel)?;
        let mix_idx = self.mix_index(mix)?;

        let idx = self.matrix_index(channel_idx, mix_idx)?;
        self.ch_mix_matrix.get_mut(idx)
    }

    /// Builds a flat row-major (`channels × mixes`) slice of effective linear gains
    /// ready to be pushed to the real-time audio mixer.
    #[must_use]
    pub fn gains_snapshot(&self) -> Vec<f32> {
        self.ch_mix_matrix
            .iter()
            .map(ChannelMix::effective_gain)
            .collect()
    }

    /// Builds a gains snapshot with `(channel, mix)` replaced by `override_gain`,
    /// allowing the caller to sync the real-time mixer before committing the cell.
    #[must_use]
    pub fn gains_with_override(
        &self,
        channel: &ChannelName,
        mix: &MixName,
        override_gain: f32,
    ) -> Vec<f32> {
        let mut gains = self.gains_snapshot();
        if let Some(ch_idx) = self.channel_index(channel)
            && let Some(mix_idx) = self.mix_index(mix)
            && let Some(idx) = self.matrix_index(ch_idx, mix_idx)
        {
            gains[idx] = override_gain;
        }
        gains
    }
}

#[cfg(test)]
#[allow(clippy::cast_precision_loss, clippy::float_cmp, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn ch(name: &str) -> ChannelName {
        ChannelName::from(name)
    }

    fn mix(name: &str) -> MixName {
        MixName::from(name)
    }

    #[test]
    fn test_add_channel_and_mix_populates_matrix_with_defaults() {
        let mut graph = AudioGraph::new();
        assert!(graph.add_channel(ch("music")).is_ok());
        assert!(graph.add_channel(ch("voice")).is_ok());
        assert!(graph.add_mix(mix("stream")).is_ok());
        assert!(graph.add_mix(mix("headphones")).is_ok());

        assert_eq!(graph.channels().len(), 2);
        assert_eq!(graph.mixes().len(), 2);
        assert_eq!(graph.ch_mix_matrix.len(), 4);

        let route = graph.get_route(&ch("voice"), &mix("headphones")).unwrap();
        assert_eq!(route.channel, ch("voice"));
        assert_eq!(route.mix, mix("headphones"));
        assert!(!route.active);
        assert!(!route.muted);
        assert_eq!(route.gain, 1.0);
    }

    #[test]
    fn test_add_duplicate_channel_and_mix_returns_error() {
        let mut graph = AudioGraph::new();
        assert!(graph.add_channel(ch("music")).is_ok());
        assert!(graph.add_mix(mix("stream")).is_ok());

        let ch_err = graph.add_channel(ch("music")).unwrap_err();
        assert!(matches!(ch_err, ExecutorError::ChannelExists(name) if name == "music"));
        assert_eq!(graph.channels().len(), 1);
        assert_eq!(graph.ch_mix_matrix.len(), 1);

        let mix_err = graph.add_mix(mix("stream")).unwrap_err();
        assert!(matches!(mix_err, ExecutorError::MixExists(name) if name == "stream"));
        assert_eq!(graph.mixes().len(), 1);
        assert_eq!(graph.ch_mix_matrix.len(), 1);
    }

    #[test]
    fn test_add_mix_preserves_existing_route_state() {
        let mut graph = AudioGraph::new();
        graph.add_channel(ch("music")).unwrap();
        graph.add_channel(ch("game")).unwrap();
        graph.add_mix(mix("stream")).unwrap();

        let route = graph.get_route_mut(&ch("music"), &mix("stream")).unwrap();
        route.active = true;
        route.gain = 0.5;
        route.muted = true;

        graph.add_mix(mix("vod")).unwrap();

        let existing = graph.get_route(&ch("music"), &mix("stream")).unwrap();
        assert!(existing.active);
        assert_eq!(existing.gain, 0.5);
        assert!(existing.muted);

        let added = graph.get_route(&ch("music"), &mix("vod")).unwrap();
        assert!(!added.active);
        assert_eq!(added.gain, 1.0);
        assert!(!added.muted);
        assert_eq!(graph.ch_mix_matrix.len(), 4);
    }

    #[test]
    fn test_remove_channel_keeps_remaining_routes_aligned() {
        let mut graph = AudioGraph::new();
        for c in ["c0", "c1", "c2"] {
            graph.add_channel(ch(c)).unwrap();
        }
        for m in ["m0", "m1"] {
            graph.add_mix(mix(m)).unwrap();
        }

        for (ci, c) in ["c0", "c1", "c2"].iter().enumerate() {
            for (mi, m) in ["m0", "m1"].iter().enumerate() {
                let r = graph.get_route_mut(&ch(c), &mix(m)).unwrap();
                r.gain = (ci * 10 + mi) as f32;
            }
        }

        let removed = graph.remove_channel(&ch("c1")).unwrap();
        assert_eq!(removed.name, ch("c1"));
        assert_eq!(graph.channels().len(), 2);
        assert_eq!(graph.ch_mix_matrix.len(), 4);
        assert!(graph.get_route(&ch("c1"), &mix("m0")).is_none());

        assert_eq!(graph.get_route(&ch("c0"), &mix("m0")).unwrap().gain, 0.0);
        assert_eq!(graph.get_route(&ch("c0"), &mix("m1")).unwrap().gain, 1.0);
        assert_eq!(graph.get_route(&ch("c2"), &mix("m0")).unwrap().gain, 20.0);
        assert_eq!(graph.get_route(&ch("c2"), &mix("m1")).unwrap().gain, 21.0);

        assert!(graph.remove_channel(&ch("missing")).is_none());
        assert!(graph.remove_channel_idx(99).is_none());
    }

    #[test]
    fn test_remove_mix_keeps_remaining_routes_aligned() {
        let mut graph = AudioGraph::new();
        for c in ["c0", "c1"] {
            graph.add_channel(ch(c)).unwrap();
        }
        for m in ["m0", "m1", "m2"] {
            graph.add_mix(mix(m)).unwrap();
        }

        for (ci, c) in ["c0", "c1"].iter().enumerate() {
            for (mi, m) in ["m0", "m1", "m2"].iter().enumerate() {
                let r = graph.get_route_mut(&ch(c), &mix(m)).unwrap();
                r.gain = (ci * 10 + mi) as f32;
            }
        }

        let removed = graph.remove_mix(&mix("m1")).unwrap();
        assert_eq!(removed.name, mix("m1"));
        assert_eq!(graph.mixes().len(), 2);
        assert_eq!(graph.ch_mix_matrix.len(), 4);
        assert!(graph.get_route(&ch("c0"), &mix("m1")).is_none());

        assert_eq!(graph.get_route(&ch("c0"), &mix("m0")).unwrap().gain, 0.0);
        assert_eq!(graph.get_route(&ch("c0"), &mix("m2")).unwrap().gain, 2.0);
        assert_eq!(graph.get_route(&ch("c1"), &mix("m0")).unwrap().gain, 10.0);
        assert_eq!(graph.get_route(&ch("c1"), &mix("m2")).unwrap().gain, 12.0);

        assert!(graph.remove_mix(&mix("missing")).is_none());
        assert!(graph.remove_mix_idx(99).is_none());
    }

    #[test]
    fn test_get_route_returns_none_for_unknown_channel_or_mix() {
        let mut graph = AudioGraph::new();
        graph.add_channel(ch("music")).unwrap();
        graph.add_mix(mix("stream")).unwrap();

        assert!(graph.get_route(&ch("unknown"), &mix("stream")).is_none());
        assert!(graph.get_route(&ch("music"), &mix("unknown")).is_none());
        assert!(
            graph
                .get_route_mut(&ch("unknown"), &mix("stream"))
                .is_none()
        );
        assert!(graph.get_route_mut(&ch("music"), &mix("unknown")).is_none());
    }

    #[test]
    fn test_case_insensitive_route_lookup_preserves_original_casing() {
        let mut graph = AudioGraph::new();
        graph.add_channel(ch("Music")).unwrap();
        graph.add_mix(mix("Personal")).unwrap();

        assert!(graph.add_channel(ch("music")).is_err());
        assert!(graph.add_mix(mix("PERSONAL")).is_err());

        assert_eq!(graph.channels()[0].name, ch("Music"));
        assert_eq!(graph.mixes()[0].name, mix("Personal"));
        assert_eq!(graph.resolve_channel_name(&ch("music")), Some(ch("Music")));
        assert_eq!(graph.resolve_mix_name(&mix("personal")), Some(mix("Personal")));

        graph.get_route_mut(&ch("music"), &mix("personal")).unwrap().active = true;
        assert!(graph.get_route(&ch("MUSIC"), &mix("Personal")).unwrap().active);
    }
}
