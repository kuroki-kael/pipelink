# Project Guidelines for PipeLink

> Mirrors JetBrains rules in `.aiassistant/rules/git.md` and `.aiassistant/rules/guidelines.md` for Antigravity and CLI agents, plus release/CI invariants.

## 1. Git Commit Guidelines

All Git commit messages must strictly follow Conventional Commits (v1.0.0):

```text
<type>(<scope>): <short summary in imperative mood, lowercase>

<body: explain the "why" and context/constraints, not a diff summary>

[optional footer(s): BREAKING CHANGE, Closes #123]
```

- **Allowed types:** `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `style`, `chore`, `build`, `ci`
- **Scopes:** Meaningful lowercase crate or subsystem names (e.g. `common`, `engine`, `pipewire`, `jack`, `resolver`, `cli`, `daemon`, `config`, `default-sink`, `packaging`, `release`, `cargo`).
- **Header rules:** Imperative mood, start summary lowercase, no trailing period, max 72 characters (ideally `<= 50`).
- **Commit body:** Include a concise body explaining the motivation, technical constraint, or why the change is needed (not just repeating the diff).
- **Atomic commits:** One logical change per commit; never bundle unrelated refactorings with new features.
- **Working tree hygiene:** Never stage or commit `TODO.md` (keep it untracked). Never push (`git push`) unless explicitly requested.

## 2. Rust Code Style & Architecture

- **Formatting & imports:** Standard `rustfmt` (`max_width = 100`, 4 spaces). Never use wildcard imports (`use foo::*`). Group imports in order: `std`, external crates, internal (`crate::` / `super::`).
- **Module layout & size limit:** Keep a strict single responsibility per file (`//!` module doc -> imports -> public types -> `impl` blocks -> trait impls -> helpers -> `#[cfg(test)] mod tests`). Split into submodules rather than letting any `.rs` file exceed ~300 lines.
- **Error handling & ownership:** Never use `.unwrap()` or `.expect()` outside `#[cfg(test)]`. Bubble errors with `?` and `thiserror`. Prefer borrowing (`&str`, `&[T]`) over owned args (`String`, `Vec<T>`) unless ownership transfer is required. Derive `Debug`, `Clone`, `PartialEq`, `Eq` where applicable.
- **Documentation:** All public types, traits, and functions require `///` doc-comments (plus `# Errors` / `# Panics` sections where applicable).
- **Clippy compliance:** Must pass `cargo clippy --workspace --all-targets -- -D warnings` (`clippy::all` and `clippy::pedantic`). Avoid needless `.clone()`/`.to_owned()`, `format!()` where slices suffice, lossy `as` casts (prefer `try_into()` / `From`), unused `mut`, and premature `.collect()`. Never add `#[allow(...)]` without explicit instruction and justification.

## 3. Releases, Packaging & CI Guardrails

- **Versioning:** Keep a single workspace version in root `Cargo.toml` (`[workspace.package].version` and internal `[workspace.dependencies]` entries); all workspace crates use `version.workspace = true`.
- **Main-only releases:** PR checks (`.forgejo/workflows/pr.yaml`) run on pull requests to `main`. Release tags (`v*`) must point to commits on `main` (enforced by `.forgejo/scripts/verify-tag-on-main.sh`).
- **Cargo & Registry publishing:**
  - Crates must be published in topological order (`pipelink-common` -> `pipelink-ipc` -> `pipelink-gui` -> `pipelink-resolver` -> `pipelink-jack` -> `pipelink-pw` -> `pipelink-engine` -> `pipelink`) via `.forgejo/scripts/publish-cargo.sh`.
  - Codeberg Cargo registry requires the `Bearer ` prefix on `CARGO_REGISTRIES_CODEBERG_TOKEN`.
  - Do not hardcode sensitive credentials; CI relies on `PACKAGE_TOKEN`, `CARGO_REGISTRY_TOKEN`, and optional `GH_RELEASE_TOKEN`.
