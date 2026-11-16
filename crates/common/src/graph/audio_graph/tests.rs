/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

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
    assert_eq!(
        graph.resolve_mix_name(&mix("personal")),
        Some(mix("Personal"))
    );

    graph
        .get_route_mut(&ch("music"), &mix("personal"))
        .unwrap()
        .active = true;
    assert!(
        graph
            .get_route(&ch("MUSIC"), &mix("Personal"))
            .unwrap()
            .active
    );
}
