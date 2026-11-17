/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Unit tests for GUI state synchronization.

use pipelink_common::{
    events::{ActiveAppInfo, InstalledAppInfo, OutputDeviceInfo, StateEvent},
    types::{AppDisplay, AppId, ChannelName, MixName, NodeName},
};

use crate::{
    icons,
    state::{App, Message, normalize_output_name},
};

#[test]
fn test_live_state_events_update_matrix_and_outputs() {
    let (mut app, _) = App::boot();

    let _ = app.update(Message::StateEventReceived(StateEvent::ChannelAdded {
        channel: ChannelName::from("Music"),
    }));
    let _ = app.update(Message::StateEventReceived(StateEvent::MixAdded {
        mix: MixName::from("Personal"),
    }));
    let _ = app.update(Message::StateEventReceived(StateEvent::MixChannelAdded {
        channel: ChannelName::from("Music"),
        mix: MixName::from("Personal"),
    }));
    let _ = app.update(Message::StateEventReceived(StateEvent::GainChanged {
        channel: ChannelName::from("Music"),
        mix: MixName::from("Personal"),
        gain_linear: 0.75,
        percent: 75.0,
    }));
    let _ = app.update(Message::StateEventReceived(StateEvent::MuteChanged {
        channel: ChannelName::from("Music"),
        mix: MixName::from("Personal"),
        muted: true,
    }));
    let _ = app.update(Message::StateEventReceived(
        StateEvent::GlobalLayerSwitched {
            mix: MixName::from("Personal"),
        },
    ));

    let ch = ChannelName::from("Music");
    let mix = MixName::from("Personal");
    let route = app.find_route(&ch, &mix);
    assert!(route.is_some());
    if let Some(cell) = route {
        assert!(cell.active);
        assert!(cell.muted);
        assert!((cell.gain_percent - 75.0).abs() < f32::EPSILON);
    }
    assert_eq!(app.snapshot.active_mix, Some(mix));

    app.snapshot.outputs.push(OutputDeviceInfo {
        name: NodeName::from("sink.a"),
        description: None,
        nick: None,
        is_hardware: true,
        active: true,
    });
    app.snapshot.outputs.push(OutputDeviceInfo {
        name: NodeName::from("sink.b"),
        description: None,
        nick: None,
        is_hardware: false,
        active: false,
    });

    let _ = app.update(Message::StateEventReceived(StateEvent::OutputChanged {
        output: NodeName::from("sink.b"),
    }));
    assert!(!app.snapshot.outputs[0].active);
    assert!(app.snapshot.outputs[1].active);

    let _ = app.update(Message::StateEventReceived(
        StateEvent::DefaultSinkChanged {
            sink: NodeName::from("pipelink.channel.music"),
        },
    ));
    assert!(app.is_default_channel(&ch));
}

#[test]
fn test_normalize_output_name_strips_verbose_alsa_suffixes() {
    assert_eq!(
        normalize_output_name("G733 Gaming Headset Analog Stereo"),
        "G733 Gaming Headset"
    );
    assert_eq!(
        normalize_output_name("Built-in Audio Digital Stereo (IEC958)"),
        "Built-in Audio"
    );
    assert_eq!(
        normalize_output_name("alsa_output.usb-Logitech_G733_Gaming_Headset-00.analog-stereo"),
        "Logitech G733 Gaming Headset"
    );
}

#[test]
fn test_embedded_window_icon_decodes_cleanly() {
    assert!(icons::window_icon().is_some());
}

#[test]
fn test_channel_modal_app_projection_and_picker_filtering() {
    let (mut app, _) = App::boot();
    let music = ChannelName::from("Music");
    let games = ChannelName::from("Games");

    app.snapshot.channels = vec![music.clone(), games.clone()];
    app.snapshot.apps = vec![
        ActiveAppInfo {
            display_name: AppDisplay::from("Spotify"),
            app_name: "spotify".to_string(),
            channel: Some(music.clone()),
            stream_count: 1,
        },
        ActiveAppInfo {
            display_name: AppDisplay::from("Discord"),
            app_name: "discord".to_string(),
            channel: Some(games.clone()),
            stream_count: 2,
        },
    ];
    app.snapshot.app_routes = vec![
        (AppDisplay::from("Spotify"), music.clone()),
        (AppDisplay::from("Audacity"), music.clone()),
    ];
    app.installed_apps = vec![
        InstalledAppInfo {
            id: AppId::from("com.spotify.Client"),
            name: "Spotify".to_string(),
            icon: Some("spotify-client".to_string()),
            exec_bin: Some("spotify".to_string()),
        },
        InstalledAppInfo {
            id: AppId::from("org.mozilla.firefox"),
            name: "Firefox".to_string(),
            icon: Some("firefox".to_string()),
            exec_bin: Some("firefox".to_string()),
        },
    ];

    let routed = app.routed_apps_for_channel(&music);
    assert_eq!(routed.len(), 2);
    assert_eq!(routed[0].display_name, "Audacity");
    assert_eq!(routed[0].active_streams, None);
    assert_eq!(routed[1].display_name, "Spotify");
    assert_eq!(routed[1].active_streams, Some(1));

    let groups = app.picker_apps(Some(&music), "");
    assert_eq!(groups.active.len(), 1);
    assert_eq!(groups.active[0].display_name, "Discord");
    assert_eq!(groups.active[0].current_channel, Some(games));
    assert_eq!(groups.installed.len(), 1);
    assert_eq!(groups.installed[0].display_name, "Firefox");

    let filtered = app.picker_apps(Some(&music), "fire");
    assert_eq!(filtered.active, []);
    assert_eq!(filtered.installed.len(), 1);
    assert_eq!(filtered.installed[0].display_name, "Firefox");

    let _ = app.update(Message::OpenChannelDetails(music.clone()));
    assert!(app.modal.is_some());
    let _ = app.update(Message::StateEventReceived(StateEvent::ChannelRemoved {
        channel: music,
    }));
    assert!(app.modal.is_none());
}

#[test]
fn test_resolve_app_icons_supports_absolute_svg_and_png_paths() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let svg_path = manifest_dir.join("../../assets/pipelink.svg");
    let png_path = manifest_dir.join("../../assets/icons/hicolor/16x16/apps/pipelink.png");

    let apps = vec![
        InstalledAppInfo {
            id: AppId::from("svg.app"),
            name: "SvgApp".to_string(),
            icon: Some(svg_path.display().to_string()),
            exec_bin: None,
        },
        InstalledAppInfo {
            id: AppId::from("png.app"),
            name: "PngApp".to_string(),
            icon: Some(png_path.display().to_string()),
            exec_bin: None,
        },
    ];

    let resolved = icons::xdg::resolve_app_icons(&apps);
    assert!(resolved.contains_key("svg.app"));
    assert!(resolved.contains_key("png.app"));
}

#[test]
fn test_channel_master_rename_reorder_and_create_for_app_state() {
    let (mut app, _) = App::boot();
    let music = ChannelName::from("Music");
    let games = ChannelName::from("Games");

    let _ = app.update(Message::StateEventReceived(StateEvent::ChannelAdded {
        channel: music.clone(),
    }));
    let _ = app.update(Message::StateEventReceived(StateEvent::ChannelAdded {
        channel: games.clone(),
    }));
    let _ = app.update(Message::StateEventReceived(
        StateEvent::ChannelMasterChanged {
            channel: music.clone(),
            percent: 72.0,
            muted: true,
        },
    ));
    assert_eq!(app.channel_volume(&music), (72.0, true));

    let _ = app.update(Message::StateEventReceived(
        StateEvent::DefaultSinkChanged {
            sink: NodeName::from("pipelink.channel.Music"),
        },
    ));
    let _ = app.update(Message::OpenChannelDetails(music.clone()));
    let _ = app.update(Message::StateEventReceived(StateEvent::ChannelRenamed {
        old_name: music,
        new_name: ChannelName::from("Tunes"),
    }));
    assert!(app.is_default_channel(&ChannelName::from("Tunes")));
    assert_eq!(
        app.modal.as_ref().map(|m| m.channel.as_ref()),
        Some("Tunes")
    );

    let _ = app.update(Message::StateEventReceived(StateEvent::ChannelsReordered {
        channels: vec![games, ChannelName::from("Tunes")],
    }));
    assert_eq!(app.snapshot.channels[0], "Games");

    let _ = app.update(Message::CreateChannelForApp {
        channel: ChannelName::from("Discord"),
        app: AppDisplay::from("Discord"),
    });
    assert!(app.pending_route_after_create.is_some());

    let personal = MixName::from("Personal");
    let main = MixName::from("Main");
    let _ = app.update(Message::StateEventReceived(StateEvent::MixAdded {
        mix: personal.clone(),
    }));
    let _ = app.update(Message::StateEventReceived(
        StateEvent::GlobalLayerSwitched {
            mix: personal.clone(),
        },
    ));
    let _ = app.update(Message::StartRenameMix(personal.clone()));
    assert_eq!(
        app.renaming_mix.as_ref().map(|(m, _)| m.as_ref()),
        Some("Personal")
    );
    let _ = app.update(Message::StateEventReceived(StateEvent::MixRenamed {
        old_name: personal,
        new_name: main.clone(),
    }));
    assert_eq!(app.snapshot.mixes, vec![main.clone()]);
    assert_eq!(app.snapshot.active_mix, Some(main));
    assert!(app.renaming_mix.is_none());
}
