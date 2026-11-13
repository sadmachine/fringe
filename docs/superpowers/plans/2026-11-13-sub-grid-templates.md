# Sub-grid templates implementation plan

> **For agentic workers:** Execute inline with superpowers:executing-plans. Check each unit before continuing.

**Goal:** Assemble recognizable transports and grouped hazard challenges from templates whose interiors remain intact.

**Architecture:** `pieces.rs` owns each template's geometry, hazard runs, and directed boundary ports. Its transforms move tiles and ports together. `generator/mod.rs` selects compatible orientations and joins ports through divider cells only. The solver validates authored hazards before rewards and exact minimum searches.

**Tech stack:** Existing Rust 1.75, rand, and crossterm. No new dependencies.

**Spec:** [Research and compatible initial scope](../../sub-grid-patterns.md).

## Global constraints

- Preserve the 80-column by 24-row minimum terminal and current movement/timing rules.
- Keep seeded generation deterministic, finite, and fallible with its existing recovery screen.
- Preserve exact ANY% and 100% slide minimums and forgiving pars.
- Do not carve through or prune authored template interiors.
- Keep all slide stops safe for waiting; forced timed multi-turn sequences remain outside this initial scope.
- Preserve existing uncommitted work and leave the new changes reviewable.

## 1. Template units

Files: `src/generator/pieces.rs`.

Interfaces:

```rust
struct Port { pos: Pos, direction: Direction }
struct Piece { board: Board, ports: Vec<Port> }
fn make(width: usize, height: usize, family: Family,
        required: &[Direction], rng: &mut impl Rng) -> Piece;
```

- [x] Add offset switchbacks, island returns, paired loop chains, reward pockets, touch runs, and mixed crossings.
- [x] Put ports at intended entry/exit faces while preserving the wall faces needed to stop and turn.
- [x] Add transpose and reflection transforms for both tiles and port positions/directions.
- [x] Check all eight transforms and entry ports with real slide traversal. Require boundary openings to equal declared ports.
- [x] Check adjacent multi-cell pulse/touch runs, real timed traversals with waits, and meaningful geometric variation.

Run: `rtk cargo test --locked --lib generator::pieces::tests`.

## 2. Composition and global proof

Files: `src/generator/mod.rs`, `src/generator/solver.rs`, `tests/generation.rs`.

- [x] Increase template footprints to 9x7 on easy/normal and 11x7 on hard; retain two/four/four pieces.
- [x] Include a mixed crossing in every stage; vary the remaining template families, adding another challenge on normal/hard.
- [x] Connect only declared boundary ports, routing along their shared divider. Test that joining ports leaves every template tile unchanged.
- [x] Remove post-assembly pointwise hazard scattering and unusable-pocket pruning.
- [x] Reject a candidate unless every floor and authored dynamic hazard belongs to a viable winning route.
- [x] Reuse the solver's safe-window and resting-cell rules to validate existing hazard tiles, then compute exact collection minima.
- [x] Require contiguous runs of both hazard types in final seeded boards and replay solved paths through `Game`.

Run focused generator tests, then `rtk cargo test --locked --test generation` for 100 seeds per difficulty and the independent minimum oracle.

## 3. Acceptance and documentation

Files: `README.md`, `docs/planned-features.md`, `scripts/terminal_smoke.py` only if a generated-fixture assumption needs updating.

- [x] Explain authored templates, ports, grouped hazards, and the continuing safe-stop timing contract.
- [x] Mark the initial sub-grid feature done while leaving forced timed multi-turn rooms explicitly pending.
- [x] Run formatting, Clippy, Rust tests, release build, and the existing real-terminal acceptance script.
- [x] Request a focused review of port geometry, transforms, grouping, seeded generation, and exact-minimum preservation; fix concrete findings.

Acceptance commands:

```sh
rtk cargo fmt --check
rtk cargo clippy --all-targets --locked -- -D warnings
rtk cargo test --locked
rtk cargo build --release --locked
rtk proxy python3 scripts/terminal_smoke.py target/release/fringe
```
