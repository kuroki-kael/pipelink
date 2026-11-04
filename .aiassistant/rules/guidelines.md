---
apply: always
---

# Rust Code Style & Architecture Guidelines

## 1. Code Formatting & Tooling

- The codebase follows standard `rustfmt` formatting (max width: 100, 4 spaces).
- Never use wildcard imports (e.g. `use foo::*`). Import items explicitly.
- Group and order imports cleanly:
    1. Standard library (`std::...`)
    2. External dependencies (`pipewire::...`, `tokio::...`)
    3. Internal crate modules (`crate::...`, `super::...`)

## 2. File & Module Structure

- Maintain strict single-responsibility per file:
    1. Module-level documentation (`//!`)
    2. Imports
    3. Public types / structs / enums
    4. Implementations (`impl Foo`)
    5. Trait implementations (`impl Bar for Foo`)
    6. Private helper functions
    7. Unit tests (`#[cfg(test)] mod tests`) at the bottom
- Prefer creating a new submodule instead of letting a single file grow beyond ~300 lines.

## 3. Idiomatic Rust & Error Handling

- Never use `.unwrap()` or `.expect()` outside of tests.
- Always bubble up errors using `?` and descriptive custom error types (using `thiserror` for internal modules).
- Prefer borrowing (`&str`, `&[T]`) over taking owned types (`String`, `Vec<T>`) in function arguments unless ownership
  transfer is strictly required.
- Derive common traits for structs and enums (`Debug`, `Clone`, `PartialEq`, `Eq`) where applicable.

## 4. Documentation & Public API

- All public types, traits, and functions must have doc-comments (`///`) explaining:
    - What it does and brief context.
    - `# Errors` section if it returns a `Result`.
    - `# Panics` section if an internal panic is possible.

## 5. Clippy & Linter Compliance

All generated Rust code must strictly comply with `clippy::all` and `clippy::pedantic` without triggering warnings:

- **No Needless Clones:** Avoid unnecessary `.clone()` or `.to_owned()` calls (`clippy::redundant_clone`,
  `clippy::clone_on_ref_ptr`). Use borrowing or references where possible.
- **Allocation Efficiency:** Prefer stack over heap. Use `&str` instead of `&String`, `&[T]` instead of `&Vec<T>`, and
  avoid allocating format strings (`format!()`) when string slices or write macros suffice (`clippy::ptr_arg`,
  `clippy::useless_format`).
- **No Cast Truncation:** Do not use `as` for lossy numeric conversions. Use `try_into()` or `From/Into` to avoid silent
  data loss (`clippy::cast_possible_truncation`, `clippy::cast_sign_loss`).
- **Clean Mutability:** Never declare variables `mut` unless they are actually mutated (`clippy::needless_borrow`,
  `unused_mut`).
- **Iterator Best Practices:** Favor iterator chains over raw indexing loops, and do not collect iterators prematurely
  if they can be iterated lazily (`clippy::needless_collect`).
- **Lint Silencing Policy:** Never suppress lints via `#[allow(...)]` in generated code unless explicitly instructed
  with a documented justification.