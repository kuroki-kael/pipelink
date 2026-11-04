/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use std::ffi::CString;

use pipewire::{context::ContextBox, core::Core, node::Node, properties::properties};

use crate::error::PipewireError;

pub(crate) fn create_null_sink(core: &Core, name: &str) -> Result<Node, PipewireError> {
    let props = properties!(
        *pipewire::keys::FACTORY_NAME => "support.null-audio-sink",
        *pipewire::keys::NODE_NAME => format!("pipelink.channel.{name}"),
        *pipewire::keys::NODE_DESCRIPTION => name,
        *pipewire::keys::MEDIA_CLASS => "Audio/Sink",
        *pipewire::keys::AUDIO_CHANNELS => "2",
        "audio.position" => "[ FL FR ]",
        *pipewire::keys::NODE_LATENCY => "256/48000",
        *pipewire::keys::AUDIO_FORMAT => "F32LE",
        *pipewire::keys::NODE_AUTOCONNECT => "false",
    );

    core.create_object::<Node>("adapter", &props)
        .map_err(Into::into)
}

#[allow(
    dead_code,
    unsafe_code,
    reason = "PipeWire C API pw_context_load_module does not have a safe wrapper in pipewire-rs"
)]
pub(crate) fn create_loopback(
    context: &ContextBox,
    name: &str,
) -> Result<*mut pipewire_sys::pw_impl_module, PipewireError> {
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(PipewireError::Invalid(format!(
            "Invalid mix name '{name}': only alphanumeric, '-', and '_' are allowed"
        )));
    }

    let args = format!(
        r#"{{
                capture.props = {{
                    media.class = "Audio/Sink"
                    node.name = "pipelink.mix.{name}"
                    node.description = "{name} Mix"
                    node.autoconnect = false
                    audio.rate = 48000
                    audio.format = "F32LE"
                    audio.channels = 2
                    audio.position = [ FL FR ]
                }}
                playback.props = {{
                    media.class = "Audio/Source"
                    node.name = "pipelink.mix.{name}.source"
                    node.description = "{name} Mix (Virtual Mic)"
                    node.autoconnect = false
                    audio.rate = 48000
                    audio.format = "F32LE"
                    audio.channels = 2
                    audio.position = [ FL FR ]
                }}
            }}"#
    );

    let c_args = CString::new(args)
        .map_err(|e| PipewireError::Invalid(format!("Invalid loopback args: {e}")))?;
    let c_name = c"libpipewire-module-loopback";

    let module_ptr = unsafe {
        pipewire_sys::pw_context_load_module(
            context.as_raw_ptr(),
            c_name.as_ptr(),
            c_args.as_ptr(),
            std::ptr::null_mut(),
        )
    };

    if module_ptr.is_null() {
        let last_err = std::io::Error::last_os_error();
        return Err(PipewireError::Invalid(format!(
            "Failed to load loopback module for mix '{name}': {last_err}"
        )));
    }

    Ok(module_ptr)
}
