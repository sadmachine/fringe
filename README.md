# Fringe

A neon terminal maze game inspired by Tomb of the Mask. Pick a direction and slide until a wall stops you. Collect dots and coins, avoid spikes, and find the exit.

Stages are assembled from procedural switchbacks, loops, and chambers. Each piece varies in geometry and orientation. The assembled stage is checked using the actual slide rules before you play it.

## Run

Install [Rust](https://rustup.rs/) 1.75 or newer, then run:

```sh
cargo run --release
```

Linux and macOS are the targets. Use an interactive terminal with at least 80 columns and 24 rows, Unicode block characters, and true-color support. Kitty works; the game uses terminal cells rather than Kitty graphics. A Nerd Font is welcome but not required. Avoid fonts configured to give block characters double width.

Choose easy, normal, or hard with the arrows or `1`, `2`, `3`, then press Enter. Difficulty changes the board size, minimum solution length, and spike placement. The HUD groups two objectives:

- `ANY%`: reach the exit safely, with collectibles optional.
- `100%`: collect every dot and coin, then reach the exit safely.

Each objective shows a bright `PAR` target and a dimmer `min` value. Minimum is the exact fewest slides. Par adds 25% to that minimum, rounding the allowance up, with at least two extra slides for detours.

Entering the exit still ends the run immediately. Generation proves both objectives are possible and retries layouts whose full-clear minimum cannot be proved within the search budget. Difficulty ranges apply to the `ANY%` minimum.

Finishing a stage brings up a gold `STAGE CLEAR!` panel with a brief sparkle animation and your score and slide totals. The result stays visible until you continue. Press Enter or N for the next stage, R to replay, or M for the menu, including during the animation.

Easy uses two pieces; normal and hard use four, with larger interiors on hard. The start and exit are separated by at least half the interior width and height. Generation rejects layouts where less than three quarters of any piece is usable, then turns small unplayable pockets into solid walls. Every remaining floor cell lies on a safe route that can still reach the exit, including after spike placement. Decorative space is wall rather than empty corridors.

| Key | Action |
| --- | --- |
| Arrows, WASD, HJKL | Slide; directions entered during a slide are ignored |
| P or Space | Pause or resume |
| R | Retry the same stage with a fresh score |
| N | Generate the next seed at the same difficulty |
| Enter after a result | Generate the next seed |
| M or Escape | Return to difficulty selection |
| Q or Ctrl-C | Quit |

The yellow `oo` mask is you. Dots give 1 point, `$` coins give 10, `/\` spikes kill on contact, and `<>` is the exit. The game suspends movement if the terminal becomes too small.

## Replay a stage

The stage seed appears above the board. To replay it:

```sh
cargo run --release -- --seed 42 --difficulty hard
```

Press Enter to start with the selected difficulty. Seeds are unsigned 64-bit decimal numbers. A seed and difficulty reproduce the same stage across launches of this game version. Generator changes in future versions may change layouts. New-stage controls increment the seed, wrapping after the maximum value.

Generation uses a finite retry budget. If no suitable stage is found, the error screen offers a new seed or a return to the menu.

## Verify

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
python3 scripts/terminal_smoke.py target/release/fringe
```

The Rust tests check movement, solver edge cases, playable piece geometry, par allowances, and 100 seeds at each difficulty. An independent test solver verifies the final boards, every floor cell's usability, start/exit separation, and safe reward placement, then plays every ANY% solution through the game model. Full-clear tests replay complete-collection solutions through the game, check hand-calculated optima, and compare against an independent exhaustive search on small stages. Tests use light optimization to keep exact route searches practical. The Python script uses only the standard library and drives the real executable in a pseudo-terminal, including each objective's par and minimum, winning a stage, replay, pause, resize, quit, and terminal restoration. CI runs these checks on Linux and macOS.

## Code

- `src/model.rs` owns cell traversal, animated slides, collection, and outcomes.
- `src/generator/` owns seeded piece generation, assembly, slide-graph verification, and difficulty acceptance.
- `src/app.rs` owns menu and stage flow independently of the terminal.
- `src/terminal.rs` owns terminal lifecycle, input, resize handling, and simulation timing. `src/render.rs` draws screens.

This version implements random stages. See [planned features](docs/planned-features.md)
for endless mode, a fixed campaign, better hazards, and powerups.
