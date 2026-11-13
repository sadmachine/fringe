# Planned features

Track upcoming work here. Unchecked features are planned, not implemented.

## Implementation order

1. [x] [Timed spike walls](#better-hazards). Done 2026-11-12.
2. [x] [Initial sub-grid templates](#sub-grid-templates). Done 2026-11-13.
3. [x] [Shield powerup](#powerups). Done 2026-11-13.
4. [ ] [Endless mode](#endless-mode).
5. [ ] [Moving enemies](#better-hazards).

This order moves better hazards ahead of endless mode so individual stages gain
more variety before they become longer runs. Shield follows spike walls to give
the first powerup a hazard to interact with.

The remaining powerups and fixed campaign stay on the backlog without a set
priority. The campaign still follows endless mode.

## Current baseline

Random stages are implemented with selectable difficulty, verified layouts,
ANY% and 100% minimums and forgiving pars, a color-matched legend, and a
stage-clear celebration. These changes are merged into `main`.

Pulse spike walls and touch traps are implemented with visible warnings,
distinct adjacent wall markings, pause and resize freezing, and safe placement
that preserves exact slide minimums. Shield pickups and inventory-aware exact
minimums are implemented. Endless mode is next.

The initial sub-grid template set is implemented with declared ports, preserved
interiors, rotations/reflections, and contiguous hazard runs. See the
[map research](sub-grid-patterns.md) for visual references and timing limits.

## Sub-grid templates

- [x] Add switchbacks, island returns, loop chains, reward pockets, touch-wall runs, and mixed crossings.
- [x] Transform tiles, ports, rewards, and hazard groups together.
- [x] Join declared ports through dividers without carving through template interiors.
- [x] Replace isolated hazard scattering with authored runs of two to four adjacent cells.
- [x] Verify every floor and hazard, exact minimums, and full collection on the assembled board.
- [ ] Add forced timed multi-turn challenges after defining and verifying their temporal safety contract.

Easy has two 9x7 templates; normal has four 9x7 templates; hard has four 11x7
templates. Every stage has a mixed crossing; normal and hard add a touch-run
switchback. Pure transport and reward templates vary the rest of the layout.
The generator rejects invalid assemblies instead of modifying their interiors.

The current safe-stop rule remains: players may wait for traps to reset and pulse
spikes to retract between slides. Hazard counts and rotations alone do not create
a forced timed multi-turn sequence. That later feature remains separate from the
completed initial set.

## Endless mode

- [ ] Build an endless run using the existing stage generator and movement model.
- [ ] Define how difficulty progresses between stages.
- [ ] Define how score carries between stages.
- [ ] Define how death ends a run and how restarting works.

Endless mode is next in the implementation order. Progression,
scoring, and death rules still need design before implementation.

## Fixed campaign

- [ ] Add a campaign of carefully tuned, fixed stages after endless mode.

Stage selection and progression still need design.

## Better hazards

- [x] Expand beyond the current static spike toward hazards closer to those in
  Tomb of the Mask.
- [x] Add dynamic spike walls. Done 2026-11-12.
- [x] Add touch-triggered spike traps and distinct adjacent wall markings. Done 2026-11-12.
- [ ] Add moving enemies.

Pulse spike walls use one shared cycle: 2.4 seconds retracted (`==`), 0.8 seconds
warning (`!!`), and 0.8 seconds active (`^^`). Warning cells are passable; active
spikes kill on contact or activation beneath the player.

Touch traps (`[]`) stay ready until crossed, then warn for two seconds, extend
spikes for 0.8 seconds, and reset. Each trap counts down independently; crossing
again while armed does not postpone activation. Both types now appear in grouped
runs owned by their challenge templates. Pause and terminal resize suspension
freeze all timers. Retry resets them.

Walls beside pulse or fixed spikes have orange edge strips; walls beside touch
traps have cyan strips, following the reference game's cyan touch-wall markings.
They retain a solid purple body, with strips facing adjacent spikes. Shared
walls show both colors on separate edges. Bright `==` and `[]` resting symbols
distinguish the types without color. Active spikes have dark symbols on solid
pink backgrounds so lethal cells stand out more than their mounts. Wall marks
apply to cardinal neighbors, remain visible in every phase, and retain normal
wall collision.

Waiting costs no slides. Generation excludes the start and every reachable
stopping cell from both types' placement, and requires every slide to fit within
the retracted pulse window and the touch countdown. Each solver edge can be
played after waiting for all touch traps to reset and for pulse spikes to retract,
preserving beatability and exact ANY% and 100% slide minimums and pars.

Moving enemies come after endless mode. Their behavior, difficulty placement,
and effect on stage verification and minimums still need design.

## Powerups

- [x] Add the first powerup, a shield inspired by Tomb of the Mask and similar games.

Powerups:

- [x] Shield: absorb one otherwise lethal hazard hit, with escape protection
  through the current slide. Done 2026-11-13.
- Magnet: collect nearby dots and coins without crossing their cells.
- Invincibility: ignore hazard damage for a short time.
- Freeze: temporarily stop moving enemies and dynamic hazards.

Every stage now has one automatic shield pickup (`{}`) at a safe stopping cell
on a winning route, before the final exit slide. It arms one charge
without awarding points and is optional for ANY% and 100%. Fixed spikes,
active pulse spikes, and active touch traps break the shield on lethal contact,
including activation underneath the player. Safe crossings retain it.

The armed player is green `OO`; a breaking shield shows `**` and protects the
remainder of that slide. Protection ends at a stop and cannot carry into another
slide. Ending on fixed spikes remains fatal on the next tick. Retry restores
the pickup and empty shield, and new stages start without a charge. Pause and
resize suspension freeze shield progress with movement.

The stage solver tracks pickup availability, armed shields, and spent shields,
allowing fixed-spike shortcuts while preserving exact ANY% and 100% minimums.
Final boards are reverified after pickup placement, including safe waiting
cells and full collection of dots and coins.

The other powerups remain proposals without a set implementation order. Their
durations, placement, stacking, effects on stage verification and minimums, and
freeze's interaction with hazard timing still need design.
