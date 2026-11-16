/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Unit tests for categorized logging.

use std::{
    io,
    sync::{Arc, Mutex},
};

use tracing::Level;
use tracing_subscriber::{
    fmt::{self, MakeWriter},
    layer::SubscriberExt,
};

use super::{
    CategorizedFormatter, LogCategory, LogConfig, LogMode, build_env_filter,
    categorized_layer::CategorizedLayer, category_for_target,
};

#[derive(Clone, Default)]
struct BufferWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl BufferWriter {
    fn contents(&self) -> String {
        let Ok(guard) = self.buf.lock() else {
            panic!("buffer mutex poisoned");
        };
        String::from_utf8_lossy(&guard).into_owned()
    }
}

struct BufferGuard {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl io::Write for BufferGuard {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let Ok(mut guard) = self.buf.lock() else {
            return Err(io::Error::other("buffer mutex poisoned"));
        };
        guard.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for BufferWriter {
    type Writer = BufferGuard;

    fn make_writer(&'a self) -> Self::Writer {
        BufferGuard {
            buf: Arc::clone(&self.buf),
        }
    }
}

#[test]
fn test_category_for_target_maps_all_subsystems() {
    assert_eq!(category_for_target("pipelink"), LogCategory::Daemon);
    assert_eq!(category_for_target("pipelink::daemon"), LogCategory::Daemon);
    assert_eq!(
        category_for_target("pipelink::ipc::server"),
        LogCategory::Ipc
    );
    assert_eq!(
        category_for_target("pipelink_engine::config::storage"),
        LogCategory::Config
    );
    assert_eq!(
        category_for_target("pipelink_engine::executor::bootstrap"),
        LogCategory::Engine
    );
    assert_eq!(
        category_for_target("pipelink_pw::loop_state"),
        LogCategory::Pipewire
    );
    assert_eq!(
        category_for_target("pipelink_jack::client"),
        LogCategory::Jack
    );
    assert_eq!(
        category_for_target("pipelink_resolver::service"),
        LogCategory::Resolver
    );
    assert_eq!(category_for_target("tokio::runtime"), LogCategory::General);
}

#[test]
fn test_default_config_and_filter_parse() {
    let cfg = LogConfig::default();
    assert_eq!(cfg.default_filter, "info");
    assert_eq!(cfg.mode, LogMode::Auto);
    assert_eq!(cfg.syslog_identifier, "pipelink");

    let Ok(filter) = build_env_filter("info,pipelink_pw=debug") else {
        panic!("valid filter directive should parse");
    };
    assert_eq!(filter.max_level_hint(), Some(Level::DEBUG.into()));
    assert!(build_env_filter("invalid====").is_err() || std::env::var_os("RUST_LOG").is_some());
}

#[test]
fn test_categorized_formatter_renders_plain_lines_with_category_badges() {
    let buffer = BufferWriter::default();
    let layer = fmt::layer()
        .with_writer(buffer.clone())
        .event_format(CategorizedFormatter::new(false));
    let subscriber = tracing_subscriber::registry().with(layer);

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(target: "pipelink_engine::executor", "Restored channel 'Music'");
        tracing::warn!(target: "pipelink_pw::loop_state", "Sink unavailable");
    });

    let output = buffer.contents();
    assert!(output.contains(" INFO [engine  ] Restored channel 'Music'"));
    assert!(output.contains(" WARN [pipewire] Sink unavailable"));
    assert!(!output.contains("\x1b["));
}

#[test]
fn test_categorized_layer_prefixes_message_and_preserves_fields() {
    let buffer = BufferWriter::default();
    let inner = fmt::layer()
        .with_writer(buffer.clone())
        .with_ansi(false)
        .without_time()
        .with_level(false)
        .with_target(false);
    let subscriber = tracing_subscriber::registry().with(CategorizedLayer::new(inner));

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(target: "pipelink_engine::executor", "Restored channel 'Music'");
        tracing::warn!(target: "pipelink_pw::loop_state", node = 42, "Sink unavailable");
    });

    let output = buffer.contents();
    assert!(output.contains("[engine] Restored channel 'Music'"));
    assert!(output.contains("[pipewire] Sink unavailable node=42"));
}
