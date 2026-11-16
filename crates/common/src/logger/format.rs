/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Terminal event formatter displaying subsystem categories consistently.

use std::fmt;

use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    fmt::{
        FmtContext, FormattedFields,
        format::{self, FormatEvent, FormatFields},
        time::{FormatTime, SystemTime},
    },
    registry::LookupSpan,
};

use super::category_for_target;

const ANSI_RESET: &str = "\x1b[0m";
const ANSI_DIM: &str = "\x1b[2m";
const ANSI_RED: &str = "\x1b[31m";
const ANSI_YELLOW: &str = "\x1b[33m";
const ANSI_GREEN: &str = "\x1b[32m";
const ANSI_BLUE: &str = "\x1b[34m";
const ANSI_MAGENTA: &str = "\x1b[35m";
const ANSI_CYAN: &str = "\x1b[36m";

/// Compact log event formatter rendering timestamp, level, category badge, and fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategorizedFormatter {
    ansi: bool,
}

impl CategorizedFormatter {
    /// Creates a new formatter with optional ANSI colorization.
    #[must_use]
    pub const fn new(ansi: bool) -> Self {
        Self { ansi }
    }

    /// Formats the event log level with fixed 5-character alignment.
    fn write_level(self, writer: &mut format::Writer<'_>, level: Level) -> fmt::Result {
        if !self.ansi {
            return write!(writer, "{level:>5}");
        }
        let color = match level {
            Level::ERROR => ANSI_RED,
            Level::WARN => ANSI_YELLOW,
            Level::INFO => ANSI_GREEN,
            Level::DEBUG => ANSI_BLUE,
            Level::TRACE => ANSI_MAGENTA,
        };
        write!(writer, "{color}{level:>5}{ANSI_RESET}")
    }

    /// Formats the padded subsystem category badge.
    fn write_category(self, writer: &mut format::Writer<'_>, target: &str) -> fmt::Result {
        let category = category_for_target(target).as_str();
        if self.ansi {
            write!(writer, "{ANSI_CYAN}[{category:<8}]{ANSI_RESET}")
        } else {
            write!(writer, "[{category:<8}]")
        }
    }

    /// Formats active span names and fields if the event occurs inside a span.
    fn write_spans<S, N>(ctx: &FmtContext<'_, S, N>, writer: &mut format::Writer<'_>) -> fmt::Result
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
        N: for<'a> FormatFields<'a> + 'static,
    {
        let Some(scope) = ctx.event_scope() else {
            return Ok(());
        };
        for span in scope.from_root() {
            write!(writer, "{}", span.name())?;
            let ext = span.extensions();
            if let Some(fields) = ext.get::<FormattedFields<N>>()
                && !fields.is_empty()
            {
                write!(writer, "{{{fields}}}")?;
            }
            write!(writer, ": ")?;
        }
        Ok(())
    }
}

impl<S, N> FormatEvent<S, N> for CategorizedFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        if self.ansi {
            write!(writer, "{ANSI_DIM}")?;
        }
        SystemTime.format_time(&mut writer)?;
        if self.ansi {
            write!(writer, "{ANSI_RESET}")?;
        }
        write!(writer, " ")?;

        let meta = event.metadata();
        self.write_level(&mut writer, *meta.level())?;
        write!(writer, " ")?;
        self.write_category(&mut writer, meta.target())?;
        write!(writer, " ")?;

        Self::write_spans(ctx, &mut writer)?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}
