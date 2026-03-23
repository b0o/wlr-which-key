# AGENTS.md — wlr-which-key

Keymap manager for wlroots-based compositors. Rust binary that displays a
which-key style popup menu over Wayland using the wlr-layer-shell protocol.

## Build Commands

```sh
# Build (debug)
cargo build

# Build (release, with LTO)
cargo build --release

# Type-check without building
cargo check --all --all-features

# Run
cargo run -- [config-name]
```

### Nix

A Nix flake is provided. `nix develop` gives a shell with cargo, rustc,
rustfmt, clippy, and rust-analyzer. The flake builds with
`nix build` (release) or `nix build .#debug` (debug).

## Test Commands

```sh
# Run all tests
cargo test --all --all-features

# Run a single test by name (substring match)
cargo test --all --all-features <test_name>

# Run tests in a specific module
cargo test --all --all-features <module>::
```

There are currently no tests in the codebase. When adding tests, place unit
tests in a `#[cfg(test)] mod tests` block at the bottom of the relevant source
file. Integration tests go in a top-level `tests/` directory.

## Lint / Format

```sh
# Format check (CI uses this exact command)
rustfmt --check --edition 2024 $(find src -name '*.rs')

# Format in place
rustfmt --edition 2024 $(find src -name '*.rs')

# Clippy (CI flags: warnings are errors, unknown lints allowed)
cargo clippy --all --all-features -- -D warnings -A unknown-lints
```

CI runs all four checks on every push/PR: `cargo check`, `cargo test`,
`rustfmt --check`, and `cargo clippy`. All must pass.

## Project Structure

```
src/
  main.rs          — Entry point, Wayland event loop, State struct
  config.rs        — Config loading, top-level Config struct
  config/
    anchor.rs      — ConfigAnchor enum (screen position)
    compat.rs      — Legacy config format support
    entry.rs       — Menu entry types (Cmd, Recursive)
    font.rs        — Font wrapper around pango::FontDescription
    namespace.rs   — Wayland namespace wrapper (CString)
  color.rs         — Color type with hex parsing, cairo integration
  key.rs           — Key/modifier parsing and matching
  menu.rs          — Menu pages, columns, rendering, action dispatch
  text.rs          — Text layout/rendering with pango+cairo
```

## Rust Edition and Toolchain

- **Edition**: 2024 (set in Cargo.toml)
- **Toolchain**: stable
- **Profile**: release uses `lto = "fat"`

## Code Style

### Imports

Organize imports in this order, separated by blank lines:

1. `mod` declarations (at file top)
2. `std` imports
3. External crate imports (anyhow, clap, serde, pangocairo, wayrs_*, etc.)
4. Internal crate imports (`crate::` and `self::`)

Use `pub use` re-exports in module root files to flatten the public API
(see `config.rs` re-exporting `ConfigAnchor`, `Entry`, `Font`, `Namespace`).

Import specific items, not globs — except for Wayland protocol modules where
`*` is acceptable (e.g., `use wayrs_client::protocol::*`).

### Formatting

- Standard `rustfmt` defaults
- Line endings: LF (enforced by `.editorconfig`)
- Final newline: yes
- Nix files: 2-space indent

### Naming Conventions

- **Modules**: `snake_case` (files and `mod` declarations)
- **Types/Enums/Structs**: `PascalCase`
- **Functions/Methods/Variables**: `snake_case`
- **Constants**: `UPPER_SNAKE_CASE` (e.g., `Color::TRANSPARENT`)
- **Static items**: `UPPER_SNAKE_CASE` (e.g., `DEBUG_LAYOUT`)
- Short variable names are fine in tight scopes (`r`, `g`, `b`, `a`, `dx`, `dy`)
- Prefix config-specific wrapper types with `Config` (e.g., `ConfigAnchor`)

### Error Handling

- Use **`anyhow`** for all fallible functions: `anyhow::Result<T>` as return type
- Use `anyhow::bail!` for early-return error conditions
- Use `.context("msg")` to add context to errors (from `anyhow::Context` trait)
- For `FromStr` impls on simple types, `()` or `String` as the error type is fine
- For validated conversions, implement `TryFrom` with `anyhow::Error`
  (see `RawEntry -> Entry` in `config/entry.rs`)

### Serde / Deserialization Patterns

- Derive `Deserialize` with `#[serde(deny_unknown_fields)]` on config structs
  to catch typos in YAML config
- Use `#[serde(default)]` at the struct level combined with `SmartDefault`
  derive for config defaults
- For newtypes (Color, Key, Font, Namespace): implement custom `Deserialize`
  using the visitor pattern with a local `struct XxxVisitor`
- Use `#[serde(untagged)]` for enums that must be inferred from structure
- Use `#[serde(try_from = "RawType")]` to separate parsing from validation
- Use `#[serde(rename_all = "kebab-case")]` for user-facing YAML enum variants

### Derive Macros

Common derive patterns used in this codebase:
- Config structs: `Deserialize, SmartDefault`
- Data types: `Clone, Copy, Debug, PartialEq`
- Enums: `Deserialize, Default, Clone, Copy`
- CLI args: `Debug, Parser` (from clap)

### Unsafe Code

Unsafe blocks are used for FFI (libc, cairo). Always include a `// Safety:`
comment explaining why the usage is sound. Keep unsafe blocks as small as
possible.

### General Patterns

- The main event loop lives in `main.rs` with a `State` struct holding all
  application state, passed to Wayland callbacks
- Wayland callbacks are standalone `fn` functions (not closures), named
  `<protocol_object>_cb` (e.g., `wl_surface_cb`, `layer_surface_cb`)
- Implement handler traits (`SeatHandler`, `KeyboardHandler`) on `State`
- Use `LazyLock` for process-wide static configuration read from env vars
- Rendering goes through pango+cairo: compute text layout once in
  `ComputedText::new`, then call `render()` with position/color options
- Config supports a legacy format via `config/compat.rs` with `From`
  conversion to the current format

### Dependencies

Key crates and their roles:
- `anyhow` — error handling
- `clap` (derive) — CLI argument parsing
- `serde` + `serde_yaml` — YAML config deserialization
- `smart-default` — declarative default values for config structs
- `pangocairo` / `pango` — text layout and rendering
- `wayrs-client` / `wayrs-protocols` / `wayrs-utils` — Wayland client protocol
- `indexmap` — ordered maps (legacy config compat)
- `libc` — low-level POSIX calls (poll, daemon)
