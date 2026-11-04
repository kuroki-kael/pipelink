/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PipewireError {
    #[error("pipewire: {0}")]
    Pipewire(#[from] pipewire::Error),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("operation timed out: {0}")]
    Timeout(String),

    #[error("invalid argument: {0}")]
    Invalid(String),

    #[error("object not found: {0}")]
    NotFound(String),
}
