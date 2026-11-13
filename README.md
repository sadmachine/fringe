# Fringe

A neon terminal maze game inspired by Tomb of the Mask. Pick a direction and slide until a wall stops you. Collect dots and coins, avoid spikes, and find the exit.

Stages are assembled from six sub-grid template families: switchbacks, island returns, paired loop chains, reward pockets, touch-wall runs, and mixed-hazard crossings. Geometry, ports, and grouped hazards rotate and reflect together. The assembled stage is checked using the actual slide rules before you play it.

## Run

Install [Rust](https://rustup.rs/) 1.75 or newer, then run:

```sh
cargo run --release
```

Linux and macOS are the targets. Use an interactive terminal with at least 80 columns and 24 rows, Unicode block characters, and true-color support. Kitty works; the game uses terminal cells rather than Kitty graphics. A Nerd Font is welcome but not required. Avoid fonts configured to give block characters double width.

Choose easy, normal, or hard with the arrows or `1`, `2`, `3`, then press Enter. Difficulty changes the board size, minimum solution length, and spike placement. The HUD groups two objectives:

- `ANY%`: reach the exit safely, with collectibles optional.
- `100%`: collect every dot and coin, then reach the exit safely.

Each objective shows a bright `PAR` target and a dimmer `min` value. Minimum is the exact fewest slides, with waiting allowed at no slide cost. Par adds 25% to that minimum, rounding the allowance up, with at least two extra slides for detours.

Entering the exit still ends the run immediately. Generation proves both objectives are possible and retries layouts whose full-clear minimum cannot be proved within the search budget. Difficulty ranges apply to the `ANY%` minimum.

Finishing a stage brings up a gold `STAGE CLEAR!` panel with a brief sparkle animation and your score and slide totals. The result stays visible until you continue. Press Enter or N for the next stage, R to replay, or M for the menu, including during the animation.

Easy uses two 9x7 templates; normal uses four 9x7 templates, and hard uses four 11x7 templates. The start and exit are separated by at least half the interior width and height. Connections follow declared ports through the dividers between templates. Template interiors stay intact: generation rejects a layout if any floor, reward, or dynamic hazard lacks a viable route to the exit. It never carves through a challenge or fills in its inaccessible cells to make it pass verification.

Every stage includes a mixed crossing with pulse and touch runs on opposing legs and fixed spikes on an island face. Normal and hard also include a touch-run switchback. Runs contain two to four adjacent hazard cells. Other templates provide recognizable transport, separate loop returns, and optional coin pockets. Variants change leg lengths, island bounds, pocket depth, and hazard run length and position while retaining the intended stopping geometry.

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

Pulse spike walls share a four-second cycle: `==` retracted for 2.4 seconds,
`!!` warning for 0.8 seconds, then lethal `^^` spikes for 0.8 seconds. The warning
is safe to cross, but spikes kill on contact or if they activate beneath you.

Touch traps (`[]`) wait until you cross them. Contact starts a two-second `!!`
warning, followed by 0.8 seconds of lethal `^^` spikes, then the trap resets.
Crossing again during the countdown does not restart it. Each trap has its own
timer. Pause and terminal resize suspension freeze both types; retry resets them.
Hazards belong to their challenge templates rather than being scattered one cell
at a time after assembly.

The surrounding walls identify the spike type even when it is retracted:

| Wall marking | Hazard beside it |
| --- | --- |
| Orange edge strip | Pulse or fixed spikes |
| Cyan edge strip | Touch trap |
| Both colored edge strips | Both types |

Only walls directly above, below, left, or right of a spike get these permanent
markings. They retain a solid purple body, with thin colored strips facing the
adjacent spikes, and have the same movement rules as ordinary `██`. Shared walls
show both colors on separate strips. Bright resting symbols (`==` and `[]`)
distinguish trap types without relying on color. Lethal spikes use dark symbols
on a solid pink background so the active hazard stands out more than its mount.

Wait for touch traps to reset and pulse spikes to retract before launching a
slide. Generation keeps both types off all reachable stopping cells and makes
every slide shorter than the pulse safe window and touch countdown. You can
therefore wait safely between slides without changing either objective's exact
minimum or par.

This first template set allows safe waits between slides. It creates grouped
crossings and repeated movement challenges; forced timed sequences across several
turns remain planned and need additional verification of hazard timing.

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

The Rust tests check movement, pulse and touch-trap timing, persistent wall markings, solver edge cases, all eight template transforms, slide access from every port, connector preservation, contiguous hazard runs, par allowances, and 100 seeds at each difficulty. An independent test solver verifies the final boards, every floor cell's usability, start/exit separation, safe reward placement, and safe waiting cells, then plays every ANY% solution through the game model with waits from different cycle offsets. Full-clear tests replay complete-collection solutions through the game, check hand-calculated optima, and compare against an independent exhaustive search on small stages. Tests use light optimization to keep exact route searches practical. The Python script uses only the standard library and drives the real executable in a pseudo-terminal, including both spike types, their adjacent wall markings and pause/resize freezing, each objective's par and minimum, winning a stage, replay, quit, and terminal restoration. CI runs these checks on Linux and macOS.

## Code

- `src/model.rs` owns cell traversal, animated slides, collection, and outcomes.
- `src/generator/` owns seeded piece generation, assembly, slide-graph verification, and difficulty acceptance.
- `src/app.rs` owns menu and stage flow independently of the terminal.
- `src/terminal.rs` owns terminal lifecycle, input, resize handling, and simulation timing. `src/render.rs` draws screens.

This version implements random stages. See [planned features](docs/planned-features.md)
for endless mode, a fixed campaign, better hazards, and powerups.
