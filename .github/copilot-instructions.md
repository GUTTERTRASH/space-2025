## Quick orientation for AI agents

This repository is a small Bevy-based game/project in Rust. The goal of this file is to give an AI coding agent immediate, actionable knowledge about structure, build/runtime flows, project-specific conventions, and concrete code examples so you can make useful changes quickly.

### Big picture
- Main crates: the binary entrypoint is `src/main.rs` and the playable/game code is split into a library in `src/lib.rs` which re-exports modules: `combat`, `common`, `movement`, `projectile`, `reticule`, `utils`.
- The project is an ECS-style Bevy app: components live in `src/common.rs` (examples: `Player`, `Enemy`), systems live in module files like `movement.rs`, `combat.rs`, `projectile.rs`.

### Build / run / examples
- Standard builds: `cargo build` or `cargo build --release`.
- Run the main binary: `cargo run` (runs `src/main.rs`). There is also `examples/basic.rs` under the `examples/` folder; run with `cargo run --example basic`.
- If you need more logging or backtraces while running: set environment vars, e.g. `RUST_BACKTRACE=1 RUST_LOG=debug cargo run`.
- Note: `Cargo.toml` contains macOS-specific dependency overrides and a `patch.crates-io` entry for `winit`. Be careful when changing winit or macOS-targeted features.

### Important Cargo / profile choices to respect
- `Cargo.toml` sets `edition = "2024"` and uses Bevy 0.15.3.
- Dev profile: `opt-level = 1` and `opt-level = 3` for dependencies. Changes that affect performance or debugability (inlining, optimizations) should keep this in mind.

### Project-specific conventions and patterns (concrete)
- Module registration: `src/lib.rs` exposes the modules. Adding a new subsystem usually requires:
  1. creating `src/<name>.rs` with a `pub fn` or `Plugin`,
  2. adding `pub mod <name>;` to `src/lib.rs`,
  3. registering the system/plugin in the app (look for `App::new()` in `main.rs` or other run scripts).
- ECS components use `#[derive(Component)]` in `src/common.rs`. Example: `pub struct Enemy { pub health: f32 }` with `impl Default for Enemy`.
- Utility return types: `src/utils.rs::generate_targets(len: usize) -> Box<[(Vec3, Color, String)]>` — code often uses `Vec3`, `Color` and Bevy types; preserve these shapes when changing APIs.
- Random/utility code sometimes uses raw `rand::Rng` and uses `r#gen()` escaping for identifiers (preserve this pattern if you refactor).

### Integration points & external dependencies
- Bevy & related crates: `bevy`, `bevy_third_person_camera`, `bevy-inspector-egui`, `big-brain` (AI utility). Changes that touch rendering, input or scheduling should be validated at runtime.
- Platform-specific features: `objc2` and `objc2-foundation` are enabled for macOS with `relax-sign-encoding` features to mitigate macOS 26 crashes. Avoid changing macOS target blocks without testing on macOS.

### Editing guidance for common tasks (concrete examples)
- Add a new system: create `src/<feature>.rs`, add `pub mod <feature>;` to `src/lib.rs`, then in `main.rs` register the system with the Bevy `App` (or add a Plugin type). Search repository for existing registration patterns.
- Change a component: update the `#[derive(Component)]` struct in `src/common.rs`. If you add fields, update any places that construct `Enemy::default()` or expect specific layout.
- Add a spawn utility: follow `utils::generate_targets` signature style: return boxed slices when returning fixed collections to match existing call sites.

### Tests & CI
- There are no discoverable test files in the repo root. Use `cargo test` to run tests if/when added.

### Troubleshooting / quick checks
- If builds fail on macOS related to winit/objc, check the `target.'cfg(target_os = "macos")'.dependencies` block in `Cargo.toml` and the `patch.crates-io` entry for `winit`.
- If a Bevy-related crash occurs, run with `RUST_BACKTRACE=1` and `RUST_LOG=debug` and test on macOS for platform-specific issues.

### Where to look in the tree (quick links)
- `Cargo.toml` — dependency & profile config (critical).
- `src/lib.rs` — canonical list of modules to extend.
- `src/common.rs` — shared components (Player, Enemy).
- `src/utils.rs` — helper utilities (example: `generate_targets`).
- `examples/basic.rs` — runnable example.

If any section is unclear or you want more examples (e.g. an exact sample of adding a Plugin or modifying `Enemy`), tell me which task you want automated and I will expand this file with step-by-step code edits and tests.
