# Shield powerup

Add the next planned feature: an automatic shield pickup that absorbs a lethal
hazard hit and permits shortcuts through fixed spikes. The user approved the
automatic pickup, escape protection, and shortcut direction in conversation.

## Rules

- Each generated stage has exactly one shield on a safe floor stopping cell on
  a winning route, other than the start or exit. It can be collected before the
  final exit slide. Touching it removes the pickup and arms one charge.
- The shield has no score value and is optional for both ANY% and 100%.
  Full collection continues to mean all dots and coins.
- All three spike types consume the charge on otherwise lethal contact,
  including activation beneath the player. Safe crossings do not consume it.
- Implementation choice: escape protection covers the remainder of that slide,
  including adjacent hazards, and ends when movement stops. It never carries
  into another slide. A stationary player gets protection for the current tick,
  then is vulnerable again. Ending on a fixed spike remains fatal on the next
  tick, so a valid shortcut must reach safe floor or the exit.
- Shield collection does not change slide motion, hazard tiles, or trap clocks.
  Pause and resize suspension freeze movement and shield progress together.
- Retry reconstructs the original board, pickup, and empty shield. New stages
  do not carry a shield over.
- The pickup, armed player, breaking shield, and empty shield have readable
  shapes and HUD text, in addition to color, within 80 columns by 24 rows.

## Approach

A shield that absorbs only one cell's damage fails on adjacent hazard runs.
A fixed grace timer carries protection across turns and would require temporal
state in both exact solvers. Protection through the current slide fits the
existing movement model, covers hazard runs, and bounds escape protection.
This is an implementation choice made here to realize the approved brief grace.

The model owns pickup and damage. The solver extends its slide graph with three
inventory states: pickup available, shield armed, and shield spent. A pickup
cannot be collected twice. Damage protection exists only within an edge, so it
does not add a cross-edge clock. Full-clear reward dominance applies only at
the same position and inventory state.

Generation first proves the existing shield-free board, places the pickup,
then recomputes all verification and exact minimums on the final board. Every
reachable stopping cell remains safe for waiting, and every slide still fits
within the pulse and touch safe windows. Waiting for fresh retraction realizes
the solver's edges without consuming shields on dynamic hazards. Deliberately
taking dynamic damage cannot create shorter routes than this graph permits.

The mixed-crossing template has a fixed-spike bridge between two offset turns.
The existing island-face spikes could not yield a traversable shortcut. This
authored replacement keeps a longer shield-free outer loop, the pulse and touch
runs, and all declared ports. It is verified under every transform before
assembly, whose rules still leave template interiors intact.

No new dependencies, additional powerups, endless mode, or forced timed rooms.
Seeded layouts may change in this game version as documented by the seed policy.
