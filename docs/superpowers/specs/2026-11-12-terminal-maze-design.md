# Fringe: procedural terminal maze stages

Status: approved by the user on 2026-11-12; implementation authorized.

## Intent

Build a terminal game inspired by Tomb of the Mask's fast, four-directional movement and neon maze presentation. The player chooses a direction and slides until a wall stops them. The first release provides randomly generated, finite stages with selectable difficulty.

The repository is currently empty. Linux and macOS are required targets. Nerd Fonts and Kitty graphics are available to the user, but neither must be required to play the first version.

## Agreed direction

- Start with random stages, then consider endless mode, then a carefully tuned campaign of fixed stages. Only random stages are in this implementation.
- Offer selectable stage difficulty.
- Generate stages, then verify them using the actual movement rules.
- Assemble stages from variable procedural pieces. A piece has a recognizable general layout, but its geometry changes between instances. Pieces are not fixed room maps.

## Approaches considered

1. Generate a maze and solve it afterward. This was selected because it allows varied layouts while independently checking playability.
2. Construct a maze around a predetermined solution. This offers direct route control but adds construction complexity and risks repetitive layouts. It was not selected.

The user's piece-based proposal refines the selected approach: generate variable pieces, assemble them, then verify the complete stage. Verifying each piece separately is insufficient because joining pieces changes where slides can stop.

## Proposed first release

The details below are builder proposals, rather than additional user requirements.

### Play loop

Launch into a simple difficulty selection screen with easy, normal, and hard. Enter starts a generated stage. During play, arrows, WASD, or HJKL initiate slides. Movement advances through cells visibly over time; the player cannot turn until a wall stops them. Direction commands received during a slide are ignored rather than queued.

Collect dots and occasional higher-value coins along the route. Collectibles add to the stage score and disappear after collection. They are optional; reaching the exit clears the stage. Static spikes kill on contact, including when crossed during a slide. The exit clears the stage on entry, including during a slide. These interactions are resolved cell by cell in traversal order.

A clear or death screen shows the result and controls. The player can retry the same stage, generate another stage at the same difficulty, or return to difficulty selection. There is no life limit. Pause is available during play; quitting is available on every screen. Display the seed and difficulty so a layout can be reproduced. A command-line seed and difficulty allow replay across launches of the same game version; permanent seed compatibility across generator revisions is not promised.

### Procedural pieces

Begin with three families: switchbacks, loops, and chambers with internal obstacles. Each family is a small generation algorithm, not an ASCII template library. Vary dimensions where the assembly allows, passage lengths, obstacle positions, and orientation. Each family retains its defining shape. Pieces occupy rectangular regions with explicit boundary openings to adjacent pieces.

The assembler partitions the stage into regions, chooses and generates pieces, and connects them through openings in shared boundaries. The connection graph spans all pieces, with extra connections when suitable. Boundary openings and nearby stopping walls are part of generation; a physical opening alone does not guarantee a usable slide connection. Outer stage boundaries stay closed.

Piece families may share internal grid helpers. They do not need plugins, a generic grammar language, external definition files, or a public generator interface. Start with ordinary functions and data owned by the generator.

### Verification and difficulty

Use the same cell traversal and collision rules for gameplay and generation verification. A solver explores reachable resting positions and four slide directions, recording intermediate cells, lethal moves, and exits reached in transit. Wall stops define the graph nodes. Treat a spike-crossing move as lethal, not as a safe stop before the spike.

After assembly, choose a start and exit, place hazards, and verify the final stage. Accept only stages with a safe route to the exit and a useful route length for the selected difficulty. Use shortest safe solution length in slides, layout size and branching, and hazard density to distinguish difficulty. Hard must require more navigation than easy; difficulty is not merely a palette change. Initial ranges are builder decisions to calibrate with generated samples, not a claim that shortest-route length fully predicts human difficulty.

Place collectibles only on cells traversed by a safe path from the start that can still lead to the exit. This prevents rewards advertising unwinnable traps. Since collecting every dot is optional, verification need not solve a collect-everything puzzle.

Generation attempts are bounded. If the configured attempt budget is exhausted, show an actionable generation error with retry and return-to-menu controls. Do not hang, accept an unverified stage, or silently substitute a fixed map. Seeded generation must be deterministic for a given game version and difficulty, including retries. Keep generation and validation independent of terminal input and rendering.

### Terminal presentation and portability

Use Rust and `crossterm`, with a small random-number dependency for reproducible generation. Crossterm owns cross-platform terminal input, resize events, raw mode, and alternate-screen management. Use Rust's standard library for timing and other routine platform needs. A full widget framework and Kitty image transport are unnecessary for the initial playfield.

Render high-contrast neon walls, a distinct masked player glyph, dots, coins, spikes, and an exit. Give tiles distinguishable shapes as well as colors. Use a fixed two-column cell width and avoid glyphs with ambiguous terminal width. Unicode block and simple geometric characters are sufficient; a Nerd Font-specific glyph is optional only if its width is reliable.

Target a complete board plus a compact HUD on an 80-by-24 terminal at every difficulty. Board dimensions and piece counts can vary within that bound; exact sizes are builder decisions. Center the board when more space is available. If the terminal is too small, show the required size and suspend simulation until it fits. Resizing cannot move the player or advance a slide while suspended.

Use a fixed simulation cadence with elapsed-time handling separate from rendering so machine speed does not change movement. Bound catch-up work after stalls. Pause and size suspension reset elapsed-time accumulation on resume. Animation is a traversal effect, not a teleport followed by cosmetic feedback.

Restore terminal mode, screen, and cursor on normal exit, Ctrl-C, initialization or rendering errors, and unwind panics. Report non-interactive launch errors without emitting game control sequences.

### Ownership

- The game model owns board tiles, player state, slide traversal, score, stage outcomes, and the authoritative movement rules.
- The generator owns piece recipes, assembly, seeded randomness, candidate selection, the slide solver, and acceptance criteria. It calls the model's movement rules rather than duplicating them.
- The terminal application owns menu flow, key mapping, simulation timing, terminal lifecycle, resize handling, and rendering.

Keep these as small modules in one Rust application. A stage is ordinary board data, so later modes can supply stages without changing movement rules. Do not implement mode registries, campaign loaders, endless progression, or their UI in advance.

## Acceptance and proof

- Run the game in a real pseudo-terminal and exercise difficulty selection, movement, pause, retry, new stage, resize suspension, and clean exit.
- Test wall stopping, intermediate-cell collection, spike collision, exit crossing, ignored mid-slide direction changes, and terminal-independent state transitions.
- Generate batches of seeded stages at each difficulty. Independently explore slide paths to confirm safe exits, enclosed bounds, useful difficulty-specific route lengths, valid collectibles, and deterministic replay. Check each initial piece family produces variation while respecting its boundary openings.
- Prove the generation budget has a finite failure path and the application can recover from generation failure.
- Run Rust formatting, linting, and focused tests. Verify a release build on Linux. Check macOS in CI if repository hosting supports it; disclose if macOS execution was unavailable locally.
- Document run instructions, controls, seed replay, supported terminals, and verification commands.

## Non-goals

Endless play, fixed campaigns, persistence, online features, upgrades, shops, moving enemies, timed traps, sound, Kitty sprites, and a level editor are outside this first release. The initial visual identity should be original rather than using the source game's assets.

## Builder discretion

Choose module filenames, exact colors and glyphs, per-cell slide cadence, key aliases outside the primary controls, piece generation algorithms, connection placement, safe seed parsing, and measured difficulty thresholds. Prefer the smallest complete implementation that satisfies the rules and proof above. Changes to the agreed mode order, movement rules, or piece-based generation require revisiting the design.
