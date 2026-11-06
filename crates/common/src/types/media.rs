/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! `PipeWire` `media.class` property of a node.

use serde::{Deserialize, Serialize};

/// `PipeWire` `media.class` property of a node.
///
/// The vocabulary is open (e.g. `Audio/Sink`, `Stream/Output/Audio`,
/// `Audio/Source/Internal`, `Midi/Bridge`), so the raw value is kept and the
/// classification lives in the predicates below.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MediaClass(pub String);

impl From<&str> for MediaClass {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl MediaClass {
    /// Parses a `media.class` value as reported by `PipeWire`.
    #[must_use]
    pub fn from_pw_str(s: &str) -> Self {
        Self::from(s)
    }

    /// Returns whether this is an audio sink node (e.g. `Audio/Sink`).
    #[must_use]
    pub fn is_audio_sink(&self) -> bool {
        self.0 == "Audio/Sink"
    }

    /// Returns whether this is an audio source node (e.g. `Audio/Source`).
    #[must_use]
    pub fn is_audio_source(&self) -> bool {
        self.0.starts_with("Audio/Source")
    }

    /// Returns whether this is a stream output node (e.g. `Stream/Output/Audio`).
    #[must_use]
    pub fn is_stream_output(&self) -> bool {
        self.0.starts_with("Stream/Output/")
    }

    /// Returns whether this is a stream input node (e.g. `Stream/Input/Audio`).
    #[must_use]
    pub fn is_stream_input(&self) -> bool {
        self.0.starts_with("Stream/Input/")
    }
}

impl std::fmt::Display for MediaClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
