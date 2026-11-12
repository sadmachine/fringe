# Planned features

Track upcoming work here. Unchecked features are planned, not implemented.

## Implementation order

1. [Timed spike walls](#better-hazards).
2. [Shield powerup](#powerups).
3. [Endless mode](#endless-mode).
4. [Moving enemies](#better-hazards).

This order moves better hazards ahead of endless mode so individual stages gain
more variety before they become longer runs. Shield follows spike walls to give
the first powerup a hazard to interact with.

The remaining powerups and fixed campaign stay on the backlog without a set
priority. The campaign still follows endless mode.

## Current baseline

Random stages are implemented with selectable difficulty, verified layouts,
ANY% and 100% minimums and forgiving pars, a color-matched legend, and a
stage-clear celebration. These changes are merged into `main`.

## Endless mode

- [ ] Build an endless run using the existing stage generator and movement model.
- [ ] Define how difficulty progresses between stages.
- [ ] Define how score carries between stages.
- [ ] Define how death ends a run and how restarting works.

Endless mode follows the shield powerup in the implementation order. Progression,
scoring, and death rules still need design before implementation.

## Fixed campaign

- [ ] Add a campaign of carefully tuned, fixed stages after endless mode.

Stage selection and progression still need design.

## Better hazards

- [ ] Expand beyond the current static spike toward hazards closer to those in
  Tomb of the Mask.
- [ ] Add dynamic spike walls.
- [ ] Add moving enemies.

Start with timed spike walls:

- Use a fixed cycle with a clear warning before spikes activate.
- Freeze hazard timing during pause and terminal resize suspension.
- Keep generation able to prove stages are beatable.

Before implementation, settle how waiting and hazard timing affect solvability
and exact ANY% and 100% minimums and pars. Moving enemies come after endless mode.

The goal is more varied hazards that make movement and timing matter. Specific
hazard behaviors, timing, and difficulty placement still need design, including
how stage verification and exact ANY% and 100% minimums account for hazards that
change over time.

## Powerups

- [ ] Add powerups, starting with a small set of familiar arcade effects inspired
  by Tomb of the Mask and similar games.

Proposed starter set:

- Shield: absorb one otherwise lethal hazard hit.
- Magnet: collect nearby dots and coins without crossing their cells.
- Invincibility: ignore hazard damage for a short time.
- Freeze: temporarily stop moving enemies and dynamic hazards.

Implement shield first, after timed spike walls and before endless mode. The
other powerups remain proposals without a set implementation order.

The starter set is a proposal. Durations, pickup placement, stacking, and the
distinction between shield and invincibility still need design. Define how
powerups affect stage verification and exact ANY% and 100% minimums, and how
freeze interacts with hazard timing, before implementation.
