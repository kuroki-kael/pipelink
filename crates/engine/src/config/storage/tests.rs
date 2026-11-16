/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::{env, process};

use pipelink_common::types::{AppDisplay, ChannelName, MixName, NodeName};

use super::*;
use crate::config::state::RouteState;

fn unique_test_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!("pipelink-{tag}-{}-{nanos}", process::id()))
}

#[test]
fn test_storage_round_trip_and_corrupt_backup() {
    let root = unique_test_dir("storage-test");
    let paths = StoragePaths {
        config_file: root.join("config/config.json"),
        state_file: root.join("state/state.json"),
    };

    assert_eq!(paths.load_config(), ConfigGraph::default());
    assert_eq!(paths.load_state(), RuntimeState::default());

    let mut cfg = ConfigGraph::new();
    let music = ChannelName::from("Music");
    let personal = MixName::from("Personal");
    cfg.ensure_channel(&music);
    cfg.ensure_mix(&personal);
    cfg.set_channel_mix(&music, &personal);
    cfg.set_app_route(AppDisplay::from("Spotify"), &music);
    cfg.output = Some(NodeName::from("alsa_output.usb.test"));

    paths.save_config(&cfg).unwrap();
    assert_eq!(paths.load_config(), cfg);

    let state = RuntimeState {
        active_mix: Some(personal.clone()),
        routes: vec![RouteState {
            channel: music,
            mix: personal,
            gain_percent: 75.0,
            muted: true,
        }],
    };
    paths.save_state(&state).unwrap();
    assert_eq!(paths.load_state(), state);

    // Corrupt JSON resets cleanly and creates a `.bak` backup.
    fs::write(&paths.config_file, b"{not valid json").unwrap();
    assert_eq!(paths.load_config(), ConfigGraph::default());
    assert!(io_helpers::backup_path(&paths.config_file).exists());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_persistence_store_debounce_and_flush() {
    let root = unique_test_dir("store-flush");
    let paths = StoragePaths {
        config_file: root.join("config.json"),
        state_file: root.join("state.json"),
    };

    let mut store = PersistenceStore::new(Some(paths.clone()));
    assert!(store.state_flush_deadline().is_none());

    store.mark_state_dirty();
    assert!(store.state_flush_deadline().is_some());

    let mut graph = AudioGraph::new();
    graph.add_mix(MixName::from("Stream")).unwrap();
    graph.active_mix = Some(MixName::from("Stream"));

    store.flush_state_if_dirty(&graph);
    assert!(store.state_flush_deadline().is_none());
    assert_eq!(paths.load_state().active_mix, Some(MixName::from("Stream")));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_load_or_init_config_creates_starter_once_and_respects_empty_existing_file() {
    let root = unique_test_dir("starter-config");
    let paths = StoragePaths {
        config_file: root.join("config/config.json"),
        state_file: root.join("state/state.json"),
    };

    let first = paths.load_or_init_config();
    assert_eq!(first, ConfigGraph::starter_default());
    assert!(paths.config_file.exists());

    // An existing empty topology must be respected rather than overwritten.
    let empty = ConfigGraph::default();
    paths.save_config(&empty).unwrap();
    assert_eq!(paths.load_or_init_config(), empty);

    let _ = fs::remove_dir_all(&root);
}
