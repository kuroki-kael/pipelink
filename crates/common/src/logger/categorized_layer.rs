/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Layer wrapper that prepends subsystem categories to `tracing` event messages.

use std::fmt;

use tracing::{
    Event, Metadata, Subscriber,
    field::{DebugValue, Field, FieldSet, Value, Visit, debug},
    span::{Attributes, Id, Record},
};
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

use super::category_for_target;

/// Wraps an inner [`Layer`] (such as `tracing_journald::Layer`) and prefixes each
/// event's `message` field with `[<category>]` before forwarding.
#[derive(Debug)]
pub(super) struct CategorizedLayer<L> {
    inner: L,
}

impl<L> CategorizedLayer<L> {
    /// Creates a new category-prefixing wrapper around `inner`.
    pub(super) const fn new(inner: L) -> Self {
        Self { inner }
    }
}

impl<S, L> Layer<S> for CategorizedLayer<L>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    L: Layer<S>,
{
    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, S>) -> bool {
        self.inner.enabled(metadata, ctx)
    }

    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        self.inner.on_new_span(attrs, id, ctx);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        self.inner.on_record(id, values, ctx);
    }

    fn on_follows_from(&self, id: &Id, follows: &Id, ctx: Context<'_, S>) {
        self.inner.on_follows_from(id, follows, ctx);
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let meta = event.metadata();
        let fields = meta.fields();
        let Some(msg_idx) = fields.iter().position(|f| f.name() == "message") else {
            self.inner.on_event(event, ctx);
            return;
        };

        let mut collector = FieldCollector::new(fields);
        event.record(&mut collector);

        let category = category_for_target(meta.target()).as_str();
        if let Some(CapturedValue::Str(msg) | CapturedValue::Debug(msg)) =
            &mut collector.slots[msg_idx]
        {
            *msg = format!("[{category}] {msg}");
        } else {
            self.inner.on_event(event, ctx);
            return;
        }

        let prepared: Vec<Option<PreparedValue>> = collector
            .slots
            .into_iter()
            .map(|slot| slot.map(CapturedValue::into_prepared))
            .collect();
        let refs: Vec<Option<&dyn Value>> = prepared
            .iter()
            .map(|slot| slot.as_ref().map(PreparedValue::as_value))
            .collect();
        let value_set = fields.value_set_all(&refs);

        let rewritten = if let Some(parent) = event.parent().cloned() {
            Event::new_child_of(parent, meta, &value_set)
        } else if event.is_root() {
            Event::new_child_of(None, meta, &value_set)
        } else {
            Event::new(meta, &value_set)
        };

        self.inner.on_event(&rewritten, ctx);
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        self.inner.on_enter(id, ctx);
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        self.inner.on_exit(id, ctx);
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        self.inner.on_close(id, ctx);
    }
}

struct RawDebug(String);

impl fmt::Debug for RawDebug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

enum CapturedValue {
    Str(String),
    Debug(String),
    I64(i64),
    U64(u64),
    I128(i128),
    U128(u128),
    F64(f64),
    Bool(bool),
}

enum PreparedValue {
    Str(String),
    Debug(DebugValue<RawDebug>),
    I64(i64),
    U64(u64),
    I128(i128),
    U128(u128),
    F64(f64),
    Bool(bool),
}

impl CapturedValue {
    fn into_prepared(self) -> PreparedValue {
        match self {
            Self::Str(v) => PreparedValue::Str(v),
            Self::Debug(v) => PreparedValue::Debug(debug(RawDebug(v))),
            Self::I64(v) => PreparedValue::I64(v),
            Self::U64(v) => PreparedValue::U64(v),
            Self::I128(v) => PreparedValue::I128(v),
            Self::U128(v) => PreparedValue::U128(v),
            Self::F64(v) => PreparedValue::F64(v),
            Self::Bool(v) => PreparedValue::Bool(v),
        }
    }
}

impl PreparedValue {
    fn as_value(&self) -> &dyn Value {
        match self {
            Self::Str(v) => v,
            Self::Debug(v) => v,
            Self::I64(v) => v,
            Self::U64(v) => v,
            Self::I128(v) => v,
            Self::U128(v) => v,
            Self::F64(v) => v,
            Self::Bool(v) => v,
        }
    }
}

struct FieldCollector<'a> {
    fields: &'a FieldSet,
    slots: Vec<Option<CapturedValue>>,
}

impl<'a> FieldCollector<'a> {
    fn new(fields: &'a FieldSet) -> Self {
        Self {
            fields,
            slots: (0..fields.len()).map(|_| None).collect(),
        }
    }

    fn set(&mut self, field: &Field, value: CapturedValue) {
        if let Some(idx) = self.fields.iter().position(|f| f == *field) {
            self.slots[idx] = Some(value);
        }
    }
}

impl Visit for FieldCollector<'_> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.set(field, CapturedValue::F64(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.set(field, CapturedValue::I64(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.set(field, CapturedValue::U64(value));
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.set(field, CapturedValue::I128(value));
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.set(field, CapturedValue::U128(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.set(field, CapturedValue::Bool(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.set(field, CapturedValue::Str(value.to_string()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.set(field, CapturedValue::Debug(format!("{value:?}")));
    }
}
