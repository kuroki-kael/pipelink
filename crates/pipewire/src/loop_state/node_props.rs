/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Safe SPA `Props` pod encoding and decoding for channel null-sink volume and mute.

use std::io::Cursor;

use pipewire::spa::{
    pod::{
        Object, Pod, Property, PropertyFlags, Value, ValueArray, deserialize::PodDeserializer,
        serialize::PodSerializer,
    },
    sys::{SPA_PARAM_Props, SPA_PROP_channelVolumes, SPA_PROP_mute, SPA_PROP_volume},
    utils::SpaTypes,
};

/// Serializes stereo `channelVolumes`, scalar `volume`, and `mute` into a `Props` SPA
/// pod.
pub(super) fn build_props_pod(linear_volume: f32, muted: bool) -> Option<Vec<u8>> {
    let clamped = if linear_volume.is_finite() {
        linear_volume.max(0.0)
    } else {
        1.0
    };

    let obj = Object {
        type_: SpaTypes::ObjectParamProps.as_raw(),
        id: SPA_PARAM_Props,
        properties: vec![
            Property {
                key: SPA_PROP_mute,
                flags: PropertyFlags::empty(),
                value: Value::Bool(muted),
            },
            Property {
                key: SPA_PROP_channelVolumes,
                flags: PropertyFlags::empty(),
                value: Value::ValueArray(ValueArray::Float(vec![clamped, clamped])),
            },
            Property {
                key: SPA_PROP_volume,
                flags: PropertyFlags::empty(),
                value: Value::Float(clamped),
            },
        ],
    };

    PodSerializer::serialize(Cursor::new(Vec::new()), &Value::Object(obj))
        .ok()
        .map(|(cursor, _)| cursor.into_inner())
}

/// Extracts `(linear_volume, muted)` from an incoming `Props` SPA pod, if present.
pub(super) fn parse_props_pod(pod: &Pod) -> Option<(Option<f32>, Option<bool>)> {
    let (_, Value::Object(obj)) = PodDeserializer::deserialize_any_from(pod.as_bytes()).ok()?
    else {
        return None;
    };

    let mut channel_vol = None;
    let mut scalar_vol = None;
    let mut muted = None;

    for prop in obj.properties {
        match prop.key {
            k if k == SPA_PROP_mute => {
                if let Value::Bool(flag) = prop.value {
                    muted = Some(flag);
                }
            },
            k if k == SPA_PROP_channelVolumes => {
                if let Value::ValueArray(ValueArray::Float(vals)) = prop.value
                    && !vals.is_empty()
                    && let Ok(count) = u16::try_from(vals.len())
                {
                    let sum: f32 = vals.iter().copied().sum();
                    channel_vol = Some(sum / f32::from(count));
                }
            },
            k if k == SPA_PROP_volume => {
                if let Value::Float(val) = prop.value {
                    scalar_vol = Some(val);
                }
            },
            _ => {},
        }
    }

    let linear_volume = channel_vol.or(scalar_vol);
    if linear_volume.is_none() && muted.is_none() {
        return None;
    }

    Some((linear_volume, muted))
}
