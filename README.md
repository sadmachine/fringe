# Fringe

A neon terminal maze game inspired by Tomb of the Mask. Pick a direction and slide until a wall stops you. Collect dots and coins, avoid spikes, and find the exit.

Stages are assembled from procedural switchbacks, loops, and chambers. Each piece varies in geometry and orientation. The assembled stage is checked using the actual slide rules before you play it.

## Run

Install [Rust](https://rustup.rs/) 1.75 or newer, then run:

```sh
cargo run --release
```

Linux and macOS are the targets. Use an interactive terminal with at least 80 columns and 24 rows, Unicode block characters, and true-color support. Kitty works; the game uses terminal cells rather than Kitty graphics. A Nerd Font is welcome but not required. Avoid fonts configured to give block characters double width.

Choose easy, normal, or hard with the arrows or `1`, `2`, `3`, then press Enter. Difficulty changes the board size, minimum solution length, and spike placement. Every accepted stage has a safe exit. Collecting every reward is optional, and the par shown is the shortest safe route in slides.

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

The Rust tests check movement, solver edge cases, piece variation, and 100 seeds at each difficulty. An independent test solver verifies the final boards and safe reward placement, then plays every solution through the game model. The Python script uses only the standard library and drives the real executable in a pseudo-terminal, including winning a stage, replay, pause, resize, quit, and terminal restoration. CI runs these checks on Linux and macOS.

## Code

- `src/model.rs` owns cell traversal, animated slides, collection, and outcomes.
- `src/generator/` owns seeded piece generation, assembly, slide-graph verification, and difficulty acceptance.
- `src/app.rs` owns menu and stage flow independently of the terminal.
- `src/terminal.rs` owns terminal lifecycle, input, resize handling, and simulation timing. `src/render.rs` draws screens.

This version implements random stages. Endless play and a fixed campaign are future work.
