/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

pub mod desktop;
mod proc;
pub mod service;
mod wrappers;

pub use proc::is_process_alive;
pub use service::ProcessResolver;

#[must_use]
pub fn tokenize_cmdline(raw: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for ch in raw.chars() {
        if escaped {
            cur.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' if !in_single_quote => {
                escaped = true;
            },
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            },
            '"' if !in_double_quote => {
                in_double_quote = !in_double_quote;
            },
            c if c.is_whitespace() && !in_single_quote && !in_double_quote => {
                if !cur.is_empty() {
                    tokens.push(std::mem::take(&mut cur));
                }
            },
            c => {
                cur.push(c);
            },
        }
    }

    if !cur.is_empty() {
        tokens.push(cur);
    }

    tokens
}
