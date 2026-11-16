/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Unit tests for GUI state synchronization.

use pipelink_common::{
    events::{OutputDeviceInfo, StateEvent},
    types::{ChannelName, MixName, NodeName},
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
