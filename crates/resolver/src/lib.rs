/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

mod desktop;
mod proc;
pub mod service;
mod wrappers;

pub use proc::is_process_alive;
pub use service::ProcessResolver;

/// Splits a raw command line into its arguments, honouring shell quoting.
///
/// A single quote and a double quote open a string that is closed by the same
/// quote. Outside of a string a backslash escapes the next character. Inside
/// of a double quote a backslash only escapes `$`, `` ` ``, `"`, `\` and the
/// newline, everywhere else it stands for itself.
///
/// # Examples
///
/// ```
/// use pipelink_resolver::tokenize_cmdline;
///
/// assert_eq!(
///     tokenize_cmdline("\"/usr/bin/my app\" --flag"),
///     vec!["/usr/bin/my app", "--flag"]
/// );
/// ```
#[must_use]
pub fn tokenize_cmdline(raw: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut chars = raw.chars().peekable();

    while let Some(ch) = chars.next() {
        match (quote, ch) {
            // Inside a single quote every character stands for itself, until
            // the quote closes the string.
            (Some('\''), ch) => {
                if ch == '\'' {
                    quote = None;
                } else {
                    cur.push(ch);
                }
            },
            // Inside a double quote a backslash only escapes these
            // characters. Everywhere else it stands for itself.
            (Some('"'), '\\') => match chars.peek() {
                Some(&next @ ('$' | '`' | '"' | '\\')) => {
                    cur.push(next);
                    chars.next();
                },
                Some('\n') => {
                    chars.next();
                },
                _ => cur.push('\\'),
            },
            (Some('"'), ch) => {
                if ch == '"' {
                    quote = None;
                } else {
                    cur.push(ch);
                }
            },
            // Outside of a string a backslash escapes the next character.
            (None, '\\') => {
                if let Some(next) = chars.next() {
                    cur.push(next);
                }
            },
            (None, ch) if ch == '\'' || ch == '"' => {
                quote = Some(ch);
            },
            (None, ch) if ch.is_whitespace() => {
                if !cur.is_empty() {
                    tokens.push(std::mem::take(&mut cur));
                }
            },
            // A string that was opened with another kind of quote than the
            // ones above, and a character outside of a string.
            (Some(_) | None, ch) => cur.push(ch),
        }
    }

    if !cur.is_empty() {
        tokens.push(cur);
    }

    tokens
}
