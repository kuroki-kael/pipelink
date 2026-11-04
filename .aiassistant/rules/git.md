---
apply: always
---

## Git Commit Guidelines for AI Agents

All generated Git commit messages must strictly follow the Conventional Commits specification (v1.0.0).

### 1. Commit Message Structure

<type>(<scope>): <short summary in imperative mood, lowercase>

[optional body: explain the 'why' and context, not the 'what']

[optional footer (s): BREAKING CHANGE, Closes #123]

### 2. Commit Types

- `feat`: A new feature or public API capability
- `fix`: A bug fix or panic prevention
- `refactor`: Code changes that neither fix a bug nor add a feature (e.g. restructuring, simplifying)
- `perf`: Performance improvements (e.g. eliminating allocations, zero-copy parsing)
- `test`: Adding or correcting unit/integration tests
- `docs`: Documentation updates, rustdoc additions, README changes
- `style`: Formatting, missing semicolons, rustfmt runs (no code logic change)
- `chore`: Maintenance tasks, Cargo.toml dependency updates, CI/CD, build scripts

### 3. Scopes (Project Modules)

Use meaningful, lower-case scopes reflecting the crate module or subsystem:

- Examples: `core`, `app`, `config`

### 4. Style & Constraints

- **Imperative & Lowercase:** Use imperative mood in the header ("add support for...", not "added" or "adds"). Start
  summary with a lowercase letter.
- **No Trailing Dot:** Do not end the subject line with a period.
- **Max Length:** Subject line must be under 72 characters (ideally <= 50).
- **Atomic Commits:** A commit message must only describe one logical change. Never bundle refactorings with new
  features in one commit.
- **The "Why" Rule:** The body must explain the reasoning, motivation, or technical constraint behind the change, not
  rephrase the diff.