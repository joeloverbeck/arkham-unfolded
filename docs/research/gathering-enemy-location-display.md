# Gathering enemy location display: research and design

Research date: 2026-11-05. Application fixed point:
`92ad202b052105bc88a69bdafd317c03363f41c3`. Supporting decision evidence for
[#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89), following the
owner's rejection of a collapsed enemy pile as the default. This report concerns
physical-play precedents, digital layouts, governing rules and current application
geometry. Joel selected the recommended design on 2026-11-05 with "Proceed as
you recommend" after interview question Q6. This records design acceptance,
not native readability or complete-scenario acceptance. Joel subsequently
authorized implementation; the [Gathering guide](../experiments/gathering-setup.md)
describes the local candidate. Historical observations below refer to the fixed
point, while the design requirements describe the selected behavior.

The design decision was how to keep every location-resident enemy and its
counters readable without covering the map or its four investigator standee
slots. The selected design places one matching marker above a location and a
named group of its enemy cards in a tall area to the left of the map. This bounded
research ends at that selected direction and its remaining evidence limits;
there is no asset production, runtime change, or GitHub mutation.

## Selected direction

Use the separate enemy area, grouped by location, with a small paired
marker above each represented location. Keep each enemy card individually visible
with its own counters. This is a digital adaptation of a repeatedly documented
physical practice: leave enemy cards and their bookkeeping beside the map, then
use small matching pieces to indicate position. It is a promising response to
this owner's constraints, not a demonstrated universally superior layout.

Pair each location marker with both a distinctive shape or short identifier and
the written location name in the enemy area; color can reinforce that identity.
Use one marker per represented location, with a count of the cards in that group,
rather than one map marker per enemy. This reduces the matching burden reported
by a player who abandoned individual enemy proxies. The aggregation is our
design inference; the retained physical examples predominantly pair markers
per enemy.

## What players actually report

These are firsthand statements in retrieved thread/comment text, not observations
of the researchers playing. Linked images were not visually inspected. Vote
counts are not treated as evidence of prevalence or effectiveness.

### Cards beside the map and paired pieces

In a March 2024 discussion, players describe colored cubes, spare chaos tokens,
and two matching sets of numbered coins. One places a coin on each off-map enemy
card and its match at a location or in a threat area. Others report using themed
miniatures or illustrated monster tokens. One player describes seven enemies,
including three at one location, plus three location effects; moving cards aside
made that crowded case easier to manage. The same discussion contains a useful
counterexample: another player had enough space around locations throughout
Dunwich, while another consistently places investigators left, enemies right,
and attachments below. Those accounts favor wider spacing when table space is
available. [March 2024 firsthand discussion](https://www.reddit.com/r/arkhamhorrorlcg/comments/1bnjuhw)

In July 2024, several players describe paired meeples or cubes specifically to
avoid covering location text and to simplify moving enemies carrying counters.
One uses different sizes for ordinary enemies, elites, and Massive enemies, with
orientation or a separate marker for exhaustion. Others use upright enemy stands
with token trays; one reports that large counters quickly fill the tray. A player
who tried upright full-card stands found that they obscured too much of the
board. These accounts support separating map position from counter-bearing card
space, while showing why merely standing full cards upright is not a complete
answer. [July 2024 firsthand discussion](https://www.reddit.com/r/arkhamhorrorlcg/comments/1dtha8t)

A December 2022 author keeps enemy cards still, moves colored meeples for hunters,
and adds matching pieces to investigator displays to show engagement. However,
one respondent abandoned a similar D&D-token method because identifying the
matching enemy became cumbersome with many enemies; another stopped using
printed paired tokens. This is material contrary evidence: reducing map clutter
can create a lookup burden. [December 2022 firsthand discussion](https://www.reddit.com/r/arkhamhorrorlcg/comments/zmgwrs)

### Upright cards and finite location organizers

The author of an enemy-card holder publishes the printable model and reports
printing green holders for ordinary enemies and red for hunters. This establishes
an actual small-base vertical-card solution, but does not establish that the
standing card remains readable from every camera angle or avoids occlusion.
[Maker's description, April 2022](https://cults3d.com/en/3d-model/game/arkham-horror-lcg-card-holder)

A May 2026 location-organizer author describes overlapping enemies and
attachments as the problem. Their holder reserves three extra-card positions:
left, right, and above. A later commenter reports printing the holders and using
them for two scenarios. The author also acknowledges that some irregular maps
need loose connectors. This is direct evidence for deliberately reserved card
space, with finite capacity and map-shape limits. Its side positions conflict
with this application's existing investigator standee slots, and three extra
positions cannot establish sufficient capacity for every crowded location.
[Maker and user discussion](https://www.reddit.com/r/arkhamhorrorlcg/comments/1t4rpu1/location_manager_small_gift_to_the_community/)

## Comparison for this application

The judgments in this table are design inferences from the accounts above and
the owner's stated constraints. They are not measured application results.

| Approach | Useful property | Material drawback here | Judgment |
| --- | --- | --- | --- |
| Spread full cards around each location | Direct spatial association; every card can remain readable | Reserves large space at every location and competes with four standee slots | Credible simpler comparator, but poor fit for the selected map boundary |
| Upright full-card stands | Smaller tabletop base; card stays near its location | Large vertical surfaces can hide map content; counters need another surface | Do not select as the default |
| Small standees or miniatures, cards elsewhere | Physical presence and compact map footprint | Each enemy needs a clear card match; identical enemies and busy locations increase lookup | Viable alternative if miniature presence is itself desired |
| Numbered or paired enemy tokens, cards elsewhere | Explicit correspondence; small map pieces | Still adds one map piece and a matching task per enemy | Strong precedent; improve correspondence through location grouping |
| Fanned or overlapping enemy cards | Compact initial footprint | Conceals some printed information and counter positions; inspection requires expansion | Does not satisfy the owner's always-visible-card preference |
| Location marker plus named side group | Preserves location text and standee slots; counters stay with separate cards | Wider overall scene; camera and group association need native inspection | Recommended bounded direction |

Bounded searches on Reddit and BoardGameGeek did not produce a retained firsthand
account demonstrating fanned enemy cards as a successful AHLCG location-display
method. The fanning assessment above is geometric reasoning, not a claim about
community practice. BoardGameGeek results inspected in this pass concerned rules
or other topics rather than a sufficiently documented layout solution. This is
an evidence limit, not evidence that no such examples exist.

## Inspected digital precedents

These are source and creator-screenshot observations, not native playtests of
the other applications. No inspected digital example establishes an exact
equivalent of the proposed location-grouped enemy board.

| Precedent | Inspected behavior | Consequence here |
| --- | --- | --- |
| [halogenandtoast/ArkhamHorror location renderer][digital-location], `bf708dd537d0925b3f70b5cc3e661559ba8b7f71` | Enemy cards appear beside their location at 80% ordinary width, overlapping vertically until hover spreads the column. The [scenario CSS][digital-scenario] also needs special hover layering to avoid neighboring locations obscuring the expanded cards. | Strong geographic association, but overlapping and hover expansion fail the selected always-exposed-card outcome. |
| [DragnCards Arkham three-player layout][dragn-layout], `132b2867ca16f5964eccf6a57cd9205d220edcb7` | A freely arranged location region occupies the left 40%; investigator/threat/play rows occupy the right. The [documented Off Map group][dragn-groups] is for concealed enemies. | Separating card regions is established practice, but its Off Map group has different rules meaning and is not evidence for ordinary enemy grouping. |
| [Chaos Portal creator page][chaos-portal] and [map screenshot][chaos-map] | The inspected image has a spacious map, purple connectors, investigator pawns and extra cards beside rooms, with a hand below. | Demonstrates the alternative of spending map space to preserve proximity; the screenshot does not show a location-indexed enemy sideboard. |

All six creator-linked Chaos Portal screenshots were inspected; the fifth
contains the relevant map view. Its retained research copy is
`/tmp/arkham-chaos-portal-layout-5.jpg`, SHA-256
`787658122235aadc45699cde81d8207c4ad7709970237bb186dd39ca1abf4944`;
the primary public image is linked above. That screenshot is evidence only of
the displayed arrangement, not every scenario or ordinary usability.
OCTGN's [own guide](https://ahlcgoctgn.wordpress.com/user-guide/) describes
manual manipulation and pan/zoom but did not provide enough enemy-layout
evidence to claim a solution to this particular problem.

## Current Gathering capacity and geometry

The supported preparation has nine encounter enemy copies: Flesh-Eater and
Icy Ghoul once each, three Swarms of Rats, three Ghoul Minions and one Ravenous
Ghoul; see [content.rs](../../src/gathering/content.rs#L113). Adding the set-aside
Ghoul Priest, Tony's Quarry and Watcher from Another Dimension gives twelve
physical enemy cards as a conservative inventory-derived layout pressure.
This is not a claim that twelve unengaged enemies together is a naturally
reachable play state. Setup has Study alone; Act 1 replaces it with four rooms.

There is no Massive enemy in the selected implementation. Tony's Quarry is
Aloof. Watcher in a hand is not an in-play location enemy and must not appear
on this board until actually spawned. The board consumes the displayed
observation's real enemy records, not a list of all enemy cards in the decks.

Current cards measure 1.35 × 1.89 world units; exhausted cards swap those
dimensions. A candidate cell of 2.6 × 3.2 leaves room for a card's maximum
rotated footprint, side counters, damage above and a short name. Three columns
need approximately 7.8 units of width. Across four occupied location groups,
twelve enemies can need six rows: `9 + 1 + 1 + 1` exercises more rows than
twelve in a single group. Six rows plus four headers and margins suggest a
sideboard approximately **8 × 22**, beside the current **18 × 22** scenario
surface, with about 0.8 separation. The combined width is approximately 26.8.
These are proposed dimensions to inspect, not a verified packing result.

The current [Scenario focus](../../src/tabletop_layout.rs#L366) uses distance
40; its [camera pose](../../src/tabletop_camera.rs#L50) has pitch 1 radian and
the default window is 1600 × 1000. Arithmetic with the default 45-degree
vertical field of view suggests the wider area can fit at that distance by
recentering about 4.4 units left. It need not automatically zoom farther out.
This does not establish projected visibility through the actual camera.
At depth 40, the card width is approximately 41 pixels, a damage disk 21 pixels
and a clue-sized token 15 pixels. **Every card exposed** is a different promise
from **every card's printed rules readable in one fixed view**. Preserve
ordinary zoom/Inspect for full text; native checking must judge names, numeric
labels and location associations at the full view.

The marker above Hallway would cross its connector to Attic unless its
footprint is reserved. Extend the existing
[connector-clearance calculation](../../src/tabletop_gathering_map.rs#L149)
for the marker, preserving the actual connection graph and the four standee
slots. A roughly 0.5-high marker appears to need about 0.35 more top clearance;
verify this beside location attachments as well as bare cards.

## Rules distinctions the display must preserve

The official Rules Reference distinguishes ordinary engagement from location
presence. An ordinary engaged enemy is in an investigator's threat area and is
also at that investigator's location. An unengaged enemy is placed at its
location. Ready unengaged enemies normally engage automatically; exhausted ones
do not. Aloof is an exception. Successful evasion exhausts and disengages the
enemy, returning its card to the location. These rules make the side area a
presentation arrangement, not a new rules zone.

Massive needs a separate distinction: **a ready Massive enemy is considered
engaged with each investigator at its location; an exhausted Massive enemy is
considered engaged with none.** It cannot enter an investigator's threat area,
and it does not follow an investigator who moves away. Future support should
therefore leave its single card in the location group, independently of that
engagement relationship. Calling the group merely “Unengaged enemies” would
misdescribe ready Massive enemies. [Official Rules Reference, pp. 4, 10–11,
14–15, 20](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf)

The inspected publisher PDF is retained locally at
`local/issue-1-evidence/sources/rules-reference.pdf`, SHA-256
`14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a`.
The inspected extracted text covered Enemy Engagement, Engage Action, Evade,
Exhaust, Location Cards, Massive, Threat Area, and Aloof. This narrow check does
not authorize implementing Massive or the rest of the catalog.

## Selected behavior

These decisions are recorded in the
[accepted presentation contract](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#enemy-sideboard).
They govern the local implementation; native appearance remains unverified:

1. Put one small enemy-group marker above the location, with a matching marker
   beside its group header on the left. The header spells out `STUDY`, `HALLWAY`,
   or the actual location name. Use an app-owned visual identity distinct from
   the printed connection symbols and from clue, doom, and bounty counters.
2. Show each enemy as an individual card, retaining its damage counter above and
   other counters on stable side positions. Include space for exhausted cards
   and their counters; reserve the rotated footprint rather than relying only
   on upright card width.
3. Keep the side group persistent and discoverable. When a row fills, wrap into
   another visible row rather than collapse cards, cover counters, or continually
   shrink them. The concrete candidate uses three columns and the footprint
   above; exact gaps and framing remain subject to native inspection.
4. Preserve ordinary engaged enemies in investigator threat areas. A card moves
   between its location group and the threat area as the authoritative game
   state changes; it never appears as two independently actionable copies.
5. The map count means “cards in this location's enemy group.” It must not imply
   that zero group cards means no engaged enemy shares that location. Use this
   meaning consistently in the marker's accessible name or inspection text.
6. When cards move or leave play, update the source and destination groups and
   their markers together. Preserve enemy identity and its existing counters.
   Keep group ordering stable so another room's cards do not jump unpredictably
   when one group empties. Cross-highlighting a marker and its group is a useful
   reversible enhancement if correspondence otherwise proves difficult.

The matching identity must not depend on color alone; the short label or shape
and full group name carry the same association. This follows the useful
principle in [W3C's color guidance][color], without claiming a native accessibility
certification. Do not borrow the printed location connection symbols: they
already carry rules meaning. The archive's Enemy-phase image and Fight/Engage
symbols likewise have existing meanings, so no presence artwork is selected
merely because those files exist. The eventual marker must remain clearly
recognizable as an enemy-group locator, with a readable English fallback.

The existing renderer and motion snapshots share
[enemy_position](../../src/gathering_tabletop/play.rs#L1129), allowing a local
layout change to preserve physical identity across engagement, evasion and
hunter movement. Each exposed card must get its own action and Inspect target;
the [current pile targeting](../../src/gathering_tabletop/controls.rs#L246)
offers the whole group even when a particular member was clicked. The marker
or group header can retain access to the group. Hover/focus may highlight the
matching marker/header, but should not move the manual camera. An explicit
focus action can navigate at the player's request.

Both marker counts and enemy positions must use the same displayed moment,
including during queued movement. Do not combine a final-state destination
count with cards still visibly at their old positions. Scenario focus,
initial Scenario view, optional following and Overview need the new bounds;
ordinary manual camera use remains manual. Existing captions disappear beyond
distance 40, so widening the layout must not silently remove the very counts
this design promises. Preserve terminal projection: retained enemy records
at locations moved to the victory display must not reappear on the board.

The important pressure cases for later inspection are three distinct enemies at
one location with different counters; identical enemy art with different damage;
one exhausted card among ready cards; engagement and evasion moving a card between
areas; a hunter moving between groups; and the last enemy leaving a group. A
future Massive case must show one card and separately legible engagement state.
These are proposed inspection cases, not test runs or claims of implemented
coverage.

## Owner decision and evidence limits

The research supports the owner's direction without establishing its precise
visual dimensions. The material human tradeoff is a wider overall table in
exchange for readable cards and an uncluttered map. A tall narrow area necessarily
needs wrapping when one location has more enemies than fit across its width;
requiring a single unbroken row for any count would defeat the narrowness goal.
Joel selected wrapping at a fixed card size, subject to the
geometry findings above. Exact colors, gaps, and internal identifiers are ordinary
reversible implementation choices, not independent owner decisions.

In response to interview question Q6, Joel instructed "Proceed as you recommend,"
selecting this complete direction:
a left physical sideboard with named three-column groups and wrapped rows,
paired stable color/short-label/count markers above locations, cross-highlighting,
individual enemy actions/Inspect, ordinary engaged enemies in threat areas,
future Massive location placement, preserved card size/manual camera, and
Scenario framing that includes the board. The material alternative was a wider
map with exposed card rows beside each room. This closes the interview's last
design choice. Neither that alternative nor the rejected top-only pile exception
is the selected implementation contract. Exact geometry remains subject to
verification; implementation and full-scenario acceptance remain with #89.

Later finite layout checks should include twelve enemies in one group and
`9 + 1 + 1 + 1`, mixed ready/exhausted cards, both side counters, duplicate
names with different damage, engagement/evade/hunter transfers, Study removal,
location attachments and terminal resolution. Those fixtures would test
geometry and identity; they would not prove natural gameplay reachability of
the inventory stress configurations or arbitrary future campaign capacity.

No prototype, screenshot, native play session, usability timing, or automatic
test was performed for this report. Firsthand reports are small, self-selected
accounts with differing table sizes and campaign demands. They justify a
candidate design and expose failure modes; they do not establish community
consensus, preference for this specific left-side arrangement, or the owner's
comfort using it.

Consulted repository rules for this research and proposal: `PURP-1`–`PURP-6`,
`WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`,
`AGENT-5`, `OPS-1`–`OPS-7`, and `GOV-1`–`GOV-3`. Foundation and skill reading used
the bounded context reader. The active work and acceptance authority remain in
GitHub; this supporting report is not a second work queue.

[digital-location]: https://github.com/halogenandtoast/ArkhamHorror/blob/bf708dd537d0925b3f70b5cc3e661559ba8b7f71/frontend/src/arkham/components/Location.vue
[digital-scenario]: https://github.com/halogenandtoast/ArkhamHorror/blob/bf708dd537d0925b3f70b5cc3e661559ba8b7f71/frontend/src/arkham/components/Scenario.vue
[dragn-layout]: https://github.com/morvael/dragncards-arkhamhorrorlcg-plugin/blob/132b2867ca16f5964eccf6a57cd9205d220edcb7/jsons/Core%20Layouts.json#L98
[dragn-groups]: https://github.com/morvael/dragncards-arkhamhorrorlcg-plugin/wiki/2.-Groups
[chaos-portal]: https://steamcommunity.com/sharedfiles/filedetails/?id=2495129405
[chaos-map]: https://images.steamusercontent.com/ugc/1753569137526966903/278031362796B0C9199D6BFD35AAE29919061956/
[color]: https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html
