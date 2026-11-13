# Sub-grid pattern research

Research date: 2026-11-12. Scope: external visual evidence and design implications only. No game changes.

The inspected maps support reusable movement motifs with deliberate entrances, stopping walls, reward pockets, and hazard runs. They support more than randomly decorating generic rooms. They do **not** establish exact hazard timings, forced input sequences, or the original game's internal template system.

## Access and actual coverage

The requested [Stages 1-10 page, Stage 1 anchor](https://tomb-of-the-mask.fandom.com/wiki/Stages_1-10#Stage_1) returned HTTP 402 through the connected web tool. The same tool returned 402 for Stages 11-20 and Stages/Gallery. A normal public HTML fetch and an `action=raw` fetch returned 403. Public MediaWiki API requests with a standard browser user agent succeeded, as did public image downloads from Fandom's image CDN. No login or downloaded examples from the user were needed.

Live API discovery found:

- Three grouped stage pages: [Stages 1-10](https://tomb-of-the-mask.fandom.com/wiki/Stages_1-10), [Stages 11-20](https://tomb-of-the-mask.fandom.com/wiki/Stages_11-20), and [Stages 21-30](https://tomb-of-the-mask.fandom.com/wiki/Stages_21-30). The first two each reference ten maps. The third is an editing stub with no maps. The many later ranges in the navigation template are mostly links to pages that do not exist. [Grouped page inventory](https://tomb-of-the-mask.fandom.com/api.php?action=query&list=allpages&apprefix=Stages&aplimit=100&format=json), [first group links and images](https://tomb-of-the-mask.fandom.com/api.php?action=parse&page=Stages_1-10&prop=images%7Clinks&format=json), [second group](https://tomb-of-the-mask.fandom.com/api.php?action=parse&page=Stages_11-20&prop=images%7Clinks&format=json), [third group source](https://tomb-of-the-mask.fandom.com/api.php?action=parse&page=Stages_21-30&prop=wikitext&format=json).
- 46 individual numbered stage pages, with gaps, including a page titled Stage 651. Page existence does not establish map availability or current game content. [Individual page inventory](https://tomb-of-the-mask.fandom.com/api.php?action=query&list=allpages&apprefix=Stage%20&aplimit=100&format=json).
- [Stages/Gallery](https://tomb-of-the-mask.fandom.com/wiki/Stages/Gallery) references filenames for Stages 1 through 550, but that is not 550 accessible maps. The live file inventory contains 77 numbered `Stage_N.png` records. Direct file lookup reported Stage 36, Stage 340, and Stage 538 images missing, despite gallery references. [Gallery references](https://tomb-of-the-mask.fandom.com/api.php?action=parse&page=Stages%2FGallery&prop=images%7Clinks&format=json), [file inventory](https://tomb-of-the-mask.fandom.com/api.php?action=query&list=allimages&aiprefix=Stage&ailimit=500&aiprop=url%7Csize&format=json).

The 77 file records cover: 1-21, 24, 34, 40, 51-52, 57, 72-73, 79, 87, 90, 94, 97, 100-120, 184, 187, 203-204, 210, 217, 281, 301-306, 311, 316, 351, 364, 440, 526-527, 533, and 550. This is an inventory, not a claim that I downloaded or inspected all 77.

For navigation, use the first group's next-page link to reach Stages 11-20, then the gallery or API file inventory for later maps. An individual article may exist while its map is missing, as observed for Stage 340. Resolve actual image URLs through `action=query`, `prop=imageinfo`, and `iiprop=url|size`; do not guess CDN hashes.

## Images actually inspected

I downloaded and opened six wiki map images with `view_image`, plus the supplied local reference. These are samples across the available numbering, not a statistical survey. Stage numbers below come from the wiki file associations. They do not identify the supplied image.

| Evidence | Visible observations | Limits |
| --- | --- | --- |
| [User reference](/home/austinfishbaugh/.t3/userdata/attachments/db4f7b5d-72cf-404f-9160-bcc036d03cd4-8b32f4ac-16c9-4ee2-b33e-ee832b4182f7.png) | Purple walls; several small rectangular islands in the upper-left route; repeated routes around those islands; narrow connectors; a long bent route on the right; stars in side pockets; cyan striped wall runs and cyan marked islands. | No verified stage number or game version. No motion observed. |
| [Stage 1 map][m1] | A route through several bends and narrow passages, with wider collection pockets. Internal walls create return legs and stop positions. | Geometry only. |
| [Stage 5 map][m5] | Long bent corridors join larger pockets. A long cyan striped floor edge lies between the starting chamber and the lower-left region; another marked edge borders an upper bend. A small island sits beside a star in the lower region. | A marked edge's appearance does not prove its activation or reset schedule. |
| [Stage 14 map][m14] | Repeated rectangular chambers join through narrow necks. Cyan wall strips, cyan creature sprites, dashed yellow columns, and yellow diagonal corner pads share the map. Several yellow pads appear in facing arrangements. | Static positions do not establish movement trajectories, action order, or which mechanics are simultaneously active. |
| [Stage 114 map][m114] | The upper-right chamber contains two similar internal structures with yellow diagonal pads and cyan teeth. Left-hand corridors include multiple short marked corners and a long striped wall run. The lower-right chamber groups several cyan toothed edges around an internal structure. | The snapshot cannot establish the state or motion of the central structures, or whether the grouped hazards force a continuous sequence. |
| [Stage 301 map][m301] | Two long neighboring legs form a large return route around a central divider. Matching left/right star pockets have cyan wall marks. The route also contains offsets, a small island, and wider turning areas. | The visual pairing suggests a reusable pocket motif, but does not prove the game generates or stores it as a template. |
| [Stage 526 map][m526] | Long orthogonal lanes fold around many parallel wall fingers. Related winding patterns recur in different parts of the map with different handedness, lengths, and yellow corner-pad placements. | Long corridors alone do not establish time pressure. This sample demonstrates repetition at a much larger scale than a proposed small template. |

The map images are the visual artifacts used for the catalogue. Fandom is a fan wiki, not authoritative engine documentation. I also read the opening source excerpt of Stages 1-10 through its Stage 7 heading, the Stages 21-30 stub, and the source of Hidden Spikes, Spikes, and Stage 340. I read API link/image inventories for the grouped pages and gallery. Search-index results for Stages, Stage 1, and Category:Stages aided discovery only; their text was not treated as visual inspection. No gameplay video was watched.

## Small motif catalogue

These are abstractions of local relationships. They do not reproduce complete stage layouts. A transport here means a local movement pattern between ports, not a teleporter feature.

| Motif | Visual basis | Reusable relationship | Meaningful variation |
| --- | --- | --- | --- |
| Offset switchback | [Stage 1][m1], [Stage 5][m5], [Stage 526][m526] | Alternating wall ends force successive slide directions and move the player between offset ports. | Change the number of bends, leg lengths, or exit side while retaining the intended stops. |
| Island loop | User reference, [Stage 1][m1], [Stage 301][m301] | An internal island separates neighboring travel legs. Openings determine where the player can transfer between them. | Change island length or port placement; put a collection pocket on one leg. Verify actual slide access around the island. |
| Repeated loop chain | User reference, [Stage 526][m526] | Repeat one recognizable turn/return relationship, joined by short transfers. | Change repetition count or alternate handedness. Preserve each transfer's stopping wall and avoid a straight shortcut through the whole chain. |
| Reward alcove | User reference, [Stage 5][m5], [Stage 301][m301] | A star or reward sits off a through-route, creating a local collection task and a route back. | Move the pocket to the opposite side, alter its depth, or move the reward within it. Whether the detour adds slides must be checked. |
| Marked wall run | User reference, [Stage 5][m5], [Stage 114][m114] | A contiguous hazardous wall span belongs to a particular travel leg or turn, rather than isolated arbitrary floor sites. | Change run length, wall side, or placement before/after a bend. Timing claims require simulation. |
| Chamber with clustered obstacles | [Stage 14][m14], [Stage 114][m114] | A wider local chamber groups several obstacle relationships around islands or facing walls, connected to the next chamber through a neck. | Change the safe approach, internal obstacle arrangement, or reward branch. Distinct sprites prove visual grouping, not a timed solution. |
| Paired challenge pockets | [Stage 301][m301], [Stage 114][m114] | Similar local structures appear in related positions, so the player can recognize a repeated task. | Reflect one pocket or change its reward/approach while retaining its recognizable stopping geometry. |

## What the timing evidence establishes

The [Hidden Spikes article](https://tomb-of-the-mask.fandom.com/wiki/Hidden_Spikes), read through its [public source API](https://tomb-of-the-mask.fandom.com/api.php?action=parse&page=Hidden_Spikes&prop=wikitext&format=json), describes contact/proximity activation followed by a delay. It identifies striped flashing wall surfaces as a dormant appearance in newer versions. This supports interpreting the striped cyan marks as touch-triggered trap runs. Its prose is not a measurement of delay, reach, reset behavior, or how time advances during slides.

The [Spikes article](https://tomb-of-the-mask.fandom.com/wiki/Spikes), read through its [public source API](https://tomb-of-the-mask.fandom.com/api.php?action=parse&page=Spikes&prop=wikitext&format=json), describes contact lethality and grouped placements. The Stage 4 portion of the [first group's source](https://tomb-of-the-mask.fandom.com/api.php?action=parse&page=Stages_1-10&prop=wikitext&format=json) describes rising lava as time pressure. Those are wiki descriptions, not behavior verified in a play session.

None of the inspected static maps proves a forced timed multi-turn challenge. They do not reveal hazard phases, synchronization, trigger radius, input buffering, safe waiting positions, or whether rewards require revisiting an activated trap. Several static hazard clusters are visible. Their temporal interaction remains unverified. No original-game tick counts or exact timing constants are proposed here.

For closer timing study, the useful next evidence would be a short gameplay capture of one identified local cluster, including activation, attempted waiting, traversal, and reset. More static screenshots would improve geometric coverage but would not resolve these timing questions.

## Transformations and variation invariants

Rotations and reflections must transform the entire motif together: floor, walls, port positions and directions, stopping faces, wall-mounted hazards and their facing, rewards, and any directional actors. A rectangular footprint swaps dimensions under a quarter-turn. Deduplicate symmetric outcomes rather than counting eight transforms as eight distinct challenges.

Preserve the behavior that makes a motif recognizable:

- Entry and exit lead to the intended slide stops, not merely connected floor cells. A floor-connected alcove can still be unreachable under sliding movement.
- Connector carving preserves wall ends, islands, and hazard spans that define the task. A new opening can erase a switchback, create an escape from a timed section, or bypass a collection detour.
- Every intended reward is reachable and has a viable continuation. Check both direct completion and full collection after changing ports or pocket depth.
- Length changes affect travel time; width changes affect possible stops and turn access. Neither is a cosmetic variation.
- A hazardous variant retains its safe approach and recovery conditions, or explicitly defines the commitment interval it requires. Replacing safe turns with endangered turns changes the challenge's kind.

These are proposed design invariants derived from slide geometry, not claims about the original game's implementation.

## Sensible first template set

Start with six small, independently checkable patterns. The suggested variants are design proposals grounded in the motifs above, not copied stage fragments.

| Template | Initial purpose | Variants worth keeping |
| --- | --- | --- |
| Offset switchback | Recognizable transport with deliberate stops. | Two or more bends; short/long leg; offset exit. |
| Island return | Travel around a divider to another port. | Short/long island; alternate exit leg; optional reward pocket. |
| Short loop chain | Repeat one familiar local movement task. | Repetition count; alternating reflections; one changed transfer. |
| Reward pocket | Optional collection excursion with a clear return. | Mirrored pocket; shallow/deeper pocket; reward at a different reachable stop. |
| Touch-wall traverse | A contiguous marked span crossed between safe stops. | Span length and side; approach direction; reward beyond the span. |
| Mixed-hazard crossing | Several hazard sites form one readable crossing between safe stops. | Different ordering or spacing of pulse and touch sites, subject to a feasible shared crossing window. This combination is a Fringe proposal, not a verified original-game pattern. |

Choose the smallest footprint that preserves each task. Do not assume every motif fits every existing room size. An oversized loop chain should use a larger placement unit or fewer repetitions rather than lose its defining turns.

The parent supplied the following current Fringe constraints; this research did not independently audit the implementation. All reachable slide stops are excluded from hazards. Individual slides stay below the pulse safe window and touch delay, all pulse walls share a cycle, and waiting at each stop permits hazard reset/retraction. Under those constraints, the last two templates can group hazards into a single crossing, but every safe stop breaks temporal commitment. Rotation or extra hazard count cannot turn that into a required timed multi-turn sequence.

A later **corner relay** could require a turn before a touched wall becomes dangerous, followed by another hazard crossing before reaching safety. That is an unverified design proposal requiring a different temporal safety contract and a proof of the complete sequence. Keep it out of the initially compatible set. The external maps justify investigating clustered challenge geometry; they do not justify weakening Fringe's safety rules without that proof.

## Implementation update, 2026-11-13

The initial compatible set is now implemented in Fringe: switchbacks, island
returns, paired loop chains, reward pockets, touch-wall runs, and mixed crossings.
Each template owns its tiles and boundary ports. Transforms move geometry, ports,
coins, and hazards together; connections use divider cells without carving through
the templates. Invalid assemblies are rejected without pruning their interiors.

Dynamic hazards occur in authored runs of two to four cells. Every stage includes
a mixed crossing, and normal/hard add a touch-run switchback. The complete board
still requires safe stopping cells, traversable hazard windows, viable routes for
every floor/reward/hazard, and exact minimum solutions for both objectives. This
implements grouped crossings and repeated movement tasks, not the proposed forced
timed corner relay. No original-game timing measurements are claimed.

See [the implementation plan](superpowers/plans/2026-11-13-sub-grid-templates.md)
and [current feature status](planned-features.md#sub-grid-templates).

[m1]: https://static.wikia.nocookie.net/tomb-of-the-mask/images/5/58/Stage_1.png/revision/latest?cb=20240113170711
[m5]: https://static.wikia.nocookie.net/tomb-of-the-mask/images/b/b8/Stage_5.png/revision/latest?cb=20240113220244
[m14]: https://static.wikia.nocookie.net/tomb-of-the-mask/images/1/1b/Stage_14.png/revision/latest?cb=20240114120433
[m114]: https://static.wikia.nocookie.net/tomb-of-the-mask/images/8/82/Stage_114.png/revision/latest?cb=20240122191811
[m301]: https://static.wikia.nocookie.net/tomb-of-the-mask/images/1/19/Stage_301.png/revision/latest?cb=20240125202256
[m526]: https://static.wikia.nocookie.net/tomb-of-the-mask/images/5/5a/Stage_526.png/revision/latest?cb=20240127220330
