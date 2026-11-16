/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Output device formatting and normalization helpers for `pick_list` display.

use std::fmt;

use pipelink_common::events::OutputDeviceInfo;

/// Wrapper allowing [`OutputDeviceInfo`] to render cleanly inside an
/// `iced::widget::pick_list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputOption(pub(crate) OutputDeviceInfo);

impl fmt::Display for OutputOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&normalize_output_name(self.0.display_name()))
    }
}

/// Normalizes verbose ALSA/`PipeWire` sink descriptions into compact human-readable
/// labels.
pub(crate) fn normalize_output_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "Unknown output".to_string();
    }

    let cleaned = if !trimmed.contains(' ')
        && (trimmed.starts_with("alsa_output.") || trimmed.starts_with("alsa_input."))
    {
        let without_dir = trimmed
            .strip_prefix("alsa_output.")
            .or_else(|| trimmed.strip_prefix("alsa_input."))
            .unwrap_or(trimmed);
        let without_bus = without_dir
            .strip_prefix("usb-")
            .or_else(|| without_dir.strip_prefix("pci-"))
            .unwrap_or(without_dir);
        let base = without_bus.split('.').next().unwrap_or(without_bus);
        let base = base.strip_suffix("-00").unwrap_or(base);
        base.replace('_', " ")
    } else {
        trimmed.split_whitespace().collect::<Vec<_>>().join(" ")
    };

    for suffix in [
        " Digital Stereo (IEC958)",
        " Analog Stereo",
        " Digital Stereo",
        " Pro Audio",
        " Stereo",
    ] {
        if let Some(prefix) = cleaned.strip_suffix(suffix) {
            let candidate = prefix.trim();
            if !candidate.is_empty() {
                return candidate.to_string();
            }
        }
    }

    cleaned
}
