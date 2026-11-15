/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Tests for the tokenizer of the command line.

use pipelink_resolver::tokenize_cmdline;

#[test]
fn test_a_line_without_a_quote_is_split_at_the_spaces() {
    assert_eq!(
        tokenize_cmdline("/usr/bin/app --flag value"),
        vec!["/usr/bin/app", "--flag", "value"]
    );
}

#[test]
fn test_the_spaces_of_a_line_without_a_quote_are_not_a_token() {
    assert_eq!(tokenize_cmdline("  /usr/bin/app  "), vec!["/usr/bin/app"]);
    assert_eq!(tokenize_cmdline("   "), Vec::<String>::new());
}

#[test]
fn test_a_double_quote_closes_the_string() {
    assert_eq!(
        tokenize_cmdline("\"/usr/bin/my app\" --flag"),
        vec!["/usr/bin/my app", "--flag"]
    );
}

#[test]
fn test_a_single_quote_closes_the_string() {
    assert_eq!(
        tokenize_cmdline("'/usr/bin/my app' --flag"),
        vec!["/usr/bin/my app", "--flag"]
    );
}

#[test]
fn test_a_string_without_a_space_stays_one_token() {
    assert_eq!(tokenize_cmdline("\"/usr/bin/app\""), vec!["/usr/bin/app"]);
}

#[test]
fn test_a_quote_of_the_other_kind_stands_for_itself_inside_a_string() {
    assert_eq!(
        tokenize_cmdline("\"it's here\" --flag"),
        vec!["it's here", "--flag"]
    );
    assert_eq!(
        tokenize_cmdline("'say \"hi\"' --flag"),
        vec!["say \"hi\"", "--flag"]
    );
}

#[test]
fn test_a_backslash_escapes_the_next_character_outside_of_a_string() {
    assert_eq!(
        tokenize_cmdline("/usr/bin/my\\ app --flag"),
        vec!["/usr/bin/my app", "--flag"]
    );
}

#[test]
fn test_a_backslash_inside_a_double_quote_only_escapes_the_special_characters() {
    assert_eq!(
        tokenize_cmdline("\"C:\\Program Files\\app.exe\" --flag"),
        vec!["C:\\Program Files\\app.exe", "--flag"]
    );
    assert_eq!(tokenize_cmdline("\"a\\\"b\""), vec!["a\"b"]);
    assert_eq!(tokenize_cmdline("\"a\\\\b\""), vec!["a\\b"]);
    assert_eq!(tokenize_cmdline("\"a\\$b\""), vec!["a$b"]);
}

#[test]
fn test_the_command_of_a_desktop_entry_is_read() {
    // The form that the desktop entries use the most.
    assert_eq!(
        tokenize_cmdline("env FOO=1 /usr/bin/app --flag"),
        vec!["env", "FOO=1", "/usr/bin/app", "--flag"]
    );
}

#[test]
fn test_an_empty_string_is_no_token() {
    assert_eq!(tokenize_cmdline("\"\""), Vec::<String>::new());
}
