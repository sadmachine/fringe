# Random stages implementation plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Deliver playable, verified procedural stages with selectable difficulty in a Linux/macOS terminal.

**Architecture:** One Rust binary. The game model owns single-cell slide transitions; the generator and its solver use those same transitions. Terminal input, lifecycle, timing, and rendering consume the model without owning maze rules.

**Tech stack:** Rust, crossterm 0.27, rand 0.8, rand_chacha 0.3, standard-library Python for pseudo-terminal acceptance checks.

**Spec:** ../specs/2026-11-12-terminal-maze-design.md

## Global constraints

- Linux and macOS; all boards fit 80 columns by 24 rows.
- Random stages only. No endless mode, campaign, persistence, or Kitty transport yet.
- Reproducible seeds, bounded generation, final-stage slide verification.
- Static spikes kill in transit. Exit clears in transit. Collectibles are optional.
- Terminal cleanup on exit, Ctrl-C, errors, and unwind panics.

## Review focus

- Malformed CLI input must fail before raw mode or terminal escape output.
- A generation failure must offer retry and menu navigation.
- Pause and resize must discard accumulated simulation time.
- A reward must lie on a safe route that can subsequently reach the exit.
- Partial terminal initialization and panic paths must restore terminal state.

## Files and interfaces

- `src/model.rs`: `Board`, `Pos`, `Direction`, `Tile`, `Step`, `Outcome`, `Game`; `Board::step(Pos, Direction) -> Step` is authoritative for walls, bounds, spikes, and exits. `Game::launch(Direction)` and `Game::tick()` own animated movement and collection.
- `src/generator/pieces.rs`: variable switchback, loop, and chamber recipes filling rectangular piece interiors, with connection openings owned by assembly.
- `src/generator/mod.rs`: `Difficulty`, `Stage`, `generate(seed: u64, difficulty: Difficulty) -> Result<Stage, String>`, deterministic assembly, bounded attempts, difficulty selection.
- `src/generator/solver.rs`: reachable slide graph, shortest exit paths, and cells on safe routes to exits using `Board::step`.
- `src/app.rs`: terminal-independent menu/play/result/error state and key actions, providing testable pause and retry behavior.
- `src/terminal.rs`: RAII terminal session, rendering, event polling, bounded elapsed-time accumulator, resize suspension.
- `src/main.rs`: CLI parsing, TTY validation, entry point.
- `tests/generation.rs`: independent slide simulator, seeded stage batches and invariants.
- `scripts/terminal_smoke.py`: reusable PTY driver for the real executable and terminal cleanup proof.
- `README.md`, `.github/workflows/ci.yml`: usage and Linux/macOS checks.

### Task 1: movement model

- [x] Write model tests for wall stops, one-cell ticks, ignored direction changes, collection once, spikes and exit in transit, and out-of-board collision. Compile with API skeletons, observe failures.
- [x] Implement `Board::step`, `Game::launch`, `Game::tick`, and game restart from immutable stage data.
- [x] Run `rtk cargo test model`; expect all model tests to pass. Commit the model and manifest.

### Task 2: generator and solver

- [x] Write hand-checked solver fixtures, family variability tests, deterministic replay and zero-attempt failure tests. Observe failing tests before filling implementations.
- [x] Generate rectangular pieces with randomized local geometry and reflections; assemble a spanning connected grid with randomized openings. Explore the slide graph to choose sufficiently separated start/exit positions. Place spikes without invalidating acceptance, and populate only cells on safe winning routes.
- [x] Calibrate nonoverlapping route-length ranges for easy/normal/hard using seeded batches. Accept final candidates only after verification. Bound retries and return a clear error.
- [x] Independently verify 100 seeds per difficulty, replay every winning path through `Game`, check enclosed borders, rewards, family variation, seed stability, and failure bounds. Run `rtk cargo test`; expect all tests to pass. Commit.

### Task 3: terminal game and delivery

- [x] Write application state tests and CLI boundary tests, then observe failure. Implement difficulty selection, play, pause, retry, next stage, menu, generation errors, and controls.
- [x] Add crossterm RAII lifecycle and rendering with two-column cells. Limit per-frame catch-up and discard time on pause/resize. Fit every board at 80x24 and suspend smaller terminals. Validate CLI seed/difficulty before TTY setup.
- [x] Add a PTY smoke script that starts a seeded game, selects a difficulty, moves, pauses, retries, generates another stage, shrinks/restores the terminal, and quits. Assert screen responses and restored termios; also exercise Ctrl-C.
- [x] Add usage documentation and a Linux/macOS CI matrix running format, lint, tests, release build, and PTY checks.
- [ ] Run `rtk cargo test`, release build, formatting and lint checks, then `rtk proxy python3 scripts/terminal_smoke.py`. Obtain one fresh whole-change review, fix material findings, rerun affected checks, and commit.

## Execution record

- Initial repo contains only the approved design. No pre-existing application or test baseline.
- User's “do it” authorizes implementation without another planning checkpoint. Work in the supplied checkout on a feature branch so the result remains directly accessible.
- Native implementation, one final independent review. No parallel implementation tasks.
- Task 1 complete: six movement tests observed failing against skeletons, then passing. Commit b66ebf0.
- Task 2 complete: solver/family/budget tests observed failing against skeletons, then passing. The independent batch caught a connector ray leaving its room when facing a solid piece edge; connectors now carve bounded shortest paths to a floor inside each neighbor. All 300 seeds per batch pass, including exact replay and actual Game completion.
- Task 3 implementation verified: 19 Rust tests pass, including the 300-seed independent batch; release build succeeds with Rust 1.75; current stable formatting and strict Clippy pass. The release executable passes the PTY acceptance script. macOS execution is delegated to the configured CI matrix and has not run locally.
