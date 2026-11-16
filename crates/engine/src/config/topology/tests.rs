/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use super::*;

#[test]
fn test_config_graph_app_route_lifecycle() {
    let mut cfg = ConfigGraph::new();
    let game = ChannelName::from("Game");
    let chat = ChannelName::from("Chat");

    assert!(!cfg.set_app_route(AppDisplay::from("Discord"), &chat));

    cfg.ensure_channel(&game);
    cfg.ensure_channel(&chat);

    assert!(cfg.set_app_route(AppDisplay::from("Discord"), &chat));
    assert_eq!(cfg.find_app_channel("discord"), Some(&chat));
    assert_eq!(
        cfg.app_routes(),
        vec![(AppDisplay::from("Discord"), chat.clone())]
    );

    // Moving the app to another channel replaces the previous rule without
    // duplicates.
    assert!(cfg.set_app_route(AppDisplay::from("discord"), &game));
    assert_eq!(cfg.find_app_channel("Discord"), Some(&game));
    assert_eq!(
        cfg.app_routes(),
        vec![(AppDisplay::from("discord"), game.clone())]
    );

    // Deleting a channel clears its app rules as well.
    cfg.remove_channel(&game);
    assert_eq!(cfg.find_app_channel("Discord"), None);
    assert_eq!(cfg.app_routes(), Vec::new());
}

#[test]
fn test_config_graph_mix_route_sync_is_case_insensitive() {
    let mut cfg = ConfigGraph::new();
    let music = ChannelName::from("Music");
    let personal = MixName::from("Personal");
    cfg.ensure_channel(&music);
    cfg.ensure_mix(&personal);

    // Using differently cased names does not duplicate entries and normalizes
    // channel.
    cfg.set_channel_mix(&ChannelName::from("music"), &MixName::from("personal"));
    assert_eq!(cfg.channels.len(), 1);
    assert_eq!(cfg.mixes.len(), 1);
    assert_eq!(cfg.mixes[0].channels, vec![music.clone()]);

    cfg.remove_channel_mix(&ChannelName::from("MUSIC"), &MixName::from("PERSONAL"));
    assert_eq!(cfg.mixes[0].channels, Vec::<ChannelName>::new());

    cfg.set_channel_mix(&music, &personal);
    assert!(cfg.remove_channel(&ChannelName::from("music")));
    assert_eq!(cfg.mixes[0].channels, Vec::<ChannelName>::new());

    assert!(cfg.remove_mix(&MixName::from("PERSONAL")));
    assert_eq!(cfg.mixes, Vec::new());
}

#[test]
fn test_starter_default_topology() {
    let cfg = ConfigGraph::starter_default();
    assert!(cfg.output.is_none());
    let ch_names: Vec<&str> = cfg.channels.iter().map(|c| c.name.as_ref()).collect();
    assert_eq!(ch_names, vec!["System", "Music", "Games", "Voice"]);
    assert_eq!(cfg.mixes.len(), 2);
    assert_eq!(cfg.mixes[0].name.as_ref(), "Personal");
    assert_eq!(
        cfg.mixes[0]
            .channels
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        vec!["System", "Music", "Games", "Voice"]
    );
    assert_eq!(cfg.mixes[1].name.as_ref(), "Stream");
    assert_eq!(cfg.mixes[1].channels, Vec::<ChannelName>::new());

    let example = serde_json::from_str::<ConfigGraph>(include_str!(
        "../../../../../packaging/config.example.json"
    ))
    .map_err(|e| e.to_string());
    assert_eq!(example, Ok(cfg));
}
