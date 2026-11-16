/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Structured and categorized logging setup for `PipeLink`.
//!
//! Automatically categorizes `tracing` targets into subsystems (`daemon`, `ipc`,
//! `config`, `engine`, `pipewire`, `jack`, `resolver`, `general`), supports `RUST_LOG`
//! filtering via `EnvFilter`, and emits native `systemd-journald` events when running
//! under `systemd`.

mod categorized_layer;
mod format;
#[cfg(test)]
mod tests;

use std::{
    env, fmt,
    io::{self, IsTerminal},
};

use categorized_layer::CategorizedLayer;
pub use format::CategorizedFormatter;
use thiserror::Error;
use tracing_journald::{Priority, PriorityMappings};
use tracing_subscriber::{
    EnvFilter,
    filter::ParseError,
    fmt as subscriber_fmt,
    layer::SubscriberExt,
    util::{SubscriberInitExt, TryInitError},
};

/// Subsystem category derived from a `tracing` event target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogCategory {
    /// Daemon lifecycle and CLI daemon startup/shutdown.
    Daemon,
    /// Local IPC server and client socket handling.
    Ipc,
    /// XDG configuration and runtime state persistence.
    Config,
    /// Audio routing executor and output failover logic.
    Engine,
    /// `PipeWire` graph discovery, linking, and metadata backend.
    Pipewire,
    /// Real-time JACK audio client backend.
    Jack,
    /// Application and process-wrapper resolver.
    Resolver,
    /// Shared utilities or external crate logs.
    General,
}

impl LogCategory {
    /// Returns the lowercase label used in formatted log output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Daemon => "daemon",
            Self::Ipc => "ipc",
            Self::Config => "config",
            Self::Engine => "engine",
            Self::Pipewire => "pipewire",
            Self::Jack => "jack",
            Self::Resolver => "resolver",
            Self::General => "general",
        }
    }
}

impl fmt::Display for LogCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Output backend mode for `PipeLink` logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogMode {
    /// Uses native `journald` when `JOURNAL_STREAM` is set, otherwise terminal output.
    #[default]
    Auto,
    /// Forces human-readable categorized output on `stderr`.
    Terminal,
    /// Forces native `systemd-journald` logging and fails if the socket is unavailable.
    Journald,
}

/// Configuration used to initialize the global `tracing` subscriber.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogConfig {
    /// Fallback filter directive when `RUST_LOG` is unset.
    pub default_filter: String,
    /// Selected output backend mode.
    pub mode: LogMode,
    /// Syslog identifier tagged on `journald` records.
    pub syslog_identifier: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            default_filter: "info".to_string(),
            mode: LogMode::Auto,
            syslog_identifier: "pipelink".to_string(),
        }
    }
}

/// Errors raised while initializing the global logger.
#[derive(Debug, Error)]
pub enum LoggerInitError {
    /// The fallback filter directive could not be parsed.
    #[error("Invalid log filter directive: {0}")]
    InvalidFilter(#[from] ParseError),

    /// A global `tracing` subscriber was already registered.
    #[error("Failed to install global logger: {0}")]
    Install(#[from] TryInitError),

    /// `LogMode::Journald` was explicitly requested, but the socket could not be opened.
    #[error("Journald socket unavailable: {0}")]
    JournaldUnavailable(#[from] io::Error),
}

/// Maps a `tracing` target path to a [`LogCategory`].
#[must_use]
pub fn category_for_target(target: &str) -> LogCategory {
    if target.starts_with("pipelink::ipc") {
        LogCategory::Ipc
    } else if target == "pipelink"
        || target.starts_with("pipelink::daemon")
        || target.starts_with("pipelink::cli")
    {
        LogCategory::Daemon
    } else if target.starts_with("pipelink_engine::config") {
        LogCategory::Config
    } else if target.starts_with("pipelink_engine") {
        LogCategory::Engine
    } else if target.starts_with("pipelink_pw") {
        LogCategory::Pipewire
    } else if target.starts_with("pipelink_jack") {
        LogCategory::Jack
    } else if target.starts_with("pipelink_resolver") {
        LogCategory::Resolver
    } else {
        LogCategory::General
    }
}

/// Builds an [`EnvFilter`] from `RUST_LOG` or falls back to `default_filter`.
///
/// # Errors
/// Returns [`LoggerInitError::InvalidFilter`] if `default_filter` is not a valid
/// directive.
pub fn build_env_filter(default_filter: &str) -> Result<EnvFilter, LoggerInitError> {
    match EnvFilter::try_from_default_env() {
        Ok(filter) => Ok(filter),
        Err(_) => Ok(EnvFilter::try_new(default_filter)?),
    }
}

/// Initializes the global `tracing` subscriber according to `config`.
///
/// In [`LogMode::Auto`], `PipeLink` connects to `systemd-journald` when `JOURNAL_STREAM`
/// is present and falls back to [`CategorizedFormatter`] on `stderr` if the journal
/// socket cannot be reached.
///
/// # Errors
/// Returns an error if `config.default_filter` is invalid, if [`LogMode::Journald`] is
/// forced without an accessible journal socket, or if a global subscriber is already set.
pub fn init(config: &LogConfig) -> Result<(), LoggerInitError> {
    let filter = build_env_filter(&config.default_filter)?;

    match config.mode {
        LogMode::Journald => {
            let journald = build_journald_layer(&config.syslog_identifier)?;
            tracing_subscriber::registry()
                .with(filter)
                .with(journald)
                .try_init()?;
            Ok(())
        },
        LogMode::Auto if env::var_os("JOURNAL_STREAM").is_some() => {
            match build_journald_layer(&config.syslog_identifier) {
                Ok(journald) => {
                    tracing_subscriber::registry()
                        .with(filter)
                        .with(journald)
                        .try_init()?;
                    Ok(())
                },
                Err(err) => {
                    init_terminal(filter)?;
                    tracing::warn!(
                        "Journald socket unavailable, falling back to terminal logger: {err}"
                    );
                    Ok(())
                },
            }
        },
        LogMode::Auto | LogMode::Terminal => init_terminal(filter),
    }
}

/// Builds a `tracing-journald` layer wrapped with category-prefixed messages.
fn build_journald_layer(
    syslog_identifier: &str,
) -> io::Result<CategorizedLayer<tracing_journald::Layer>> {
    let inner = tracing_journald::layer()?
        .with_syslog_identifier(syslog_identifier.to_string())
        .with_priority_mappings(PriorityMappings {
            error: Priority::Error,
            warn: Priority::Warning,
            info: Priority::Informational,
            debug: Priority::Debug,
            trace: Priority::Debug,
        });
    Ok(CategorizedLayer::new(inner))
}

/// Installs the categorized terminal formatter writing to `stderr`.
fn init_terminal(filter: EnvFilter) -> Result<(), LoggerInitError> {
    let ansi = io::stderr().is_terminal();
    let fmt_layer = subscriber_fmt::layer()
        .with_writer(io::stderr)
        .event_format(CategorizedFormatter::new(ansi));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .try_init()?;
    Ok(())
}
