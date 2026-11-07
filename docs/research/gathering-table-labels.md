# Gathering: labels printed on the table

2026-11-07. Source fixed point:
`21127d0e87a85b00c7b38c2eba98e2e25739c19c`.

This source investigation supports the owner's `grill-with-docs` request under
[#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89). Joel selected a
Scenario heading attached to its area and delegated the design judgment about
making permanent area labels look printed on the playmat. This note recommends
the rendering boundary; the accepted design belongs in
[ADR 0002](../adr/0002-persistent-table-areas.md), and GitHub remains the active
work-control plane. No application changes or native observations were made for
this investigation.

## Why the labels currently float

The area names use the same label helper as card captions and changing counts.
It creates absolute-positioned screen UI at 18 pixels for headings and 13 pixels
for other labels. A world position supplies the screen anchor, but the text itself
remains upright, with screen-sized lettering. The production `world_labels`
system projects those anchors and sets each UI node's position. This is a direct
source observation, rather than evidence of an owner decision that permanent
markings should float. See [the shared label helper](../../src/tabletop_pieces.rs)
at lines 37–61 and [Gathering's projection system](../../src/gathering_tabletop.rs)
at lines 1483–1567.

The source history identifies the shared helper's introduction in
`120035f92254a854c02cd1d7bd3ff7f820a7cce8`, “reuse Party presentation for
Gathering.” The existing design calls for restrained playmat shading, borders
and readable zone names, and distinguishes orientation in Overview from readable
play in focused views. It does not require screen-facing permanent names.
[ADR 0002](../adr/0002-persistent-table-areas.md), “Roomier table arrangement,”
records those requirements. Constant-size, upright letters provide a readability
advantage during orbit; that advantage is an inference from the implementation,
not a documented reason for choosing it.

Scenario has additional screen-position logic: it derives its clearance from the
projected gap between the Test and Scenario surfaces, then applies that clearance
above Scenario's far edge. Its own world heading anchor is bypassed. This is the
code path to exercise for the reported apparent takeoff; the exact symptom has
not been reproduced. Existing heading checks cover normal focus presets at one
pitch, not the reported close zoom and pan. See
[tabletop_layout.rs](../../src/tabletop_layout.rs), lines 122–166 and 665–716.

## Rendering inventory and recommended boundary

| Text | Current rendering | Recommendation |
| --- | --- | --- |
| Scenario; Test area; Tony Morgan, Patrice Hathaway and Jacqueline Fine | Projected screen UI | Print flat on their own surfaces. |
| Each investigator's Threats, Assets, Hand, Deck and Discard | Projected screen UI | Print flat in the corresponding border/header bands. |
| Scenario Deck, Discard, Victory display and Map; Test's Committed cards | Projected screen UI | Print flat as permanent zone markings. |
| Asset-slot usage strip | Text rendered to a texture on a flat table mesh | Retain this existing presentation. |
| Card and standee names; counters; pile totals; bag/pool counts; changing token-group and resolution captions | Projected screen UI | Retain readable annotations in this increment. |
| Enemy-sideboard location/group identifiers and counts | Projected screen UI, with interactive group headers | Retain current annotations and interaction. These communicate live groups, not permanent mat zones. |
| Turn heading, instructions, action menus, inspection, settings and other controls | Screen UI | Retain their screen presentation. |

The permanent inventory comes from [scene.rs](../../src/gathering_tabletop/scene.rs),
lines 98–141, and [tabletop_layout.rs](../../src/tabletop_layout.rs), lines 273–310.
Gathering takes only the first `test_zone_labels` entry: the historical static
Revealed/Canceled entries are not currently spawned there. Dynamic token groups
own their current captions. See [play.rs](../../src/gathering_tabletop/play.rs),
lines 1236–1273 and 1317–1327, and
[enemy_board.rs](../../src/gathering_tabletop/enemy_board.rs), lines 196–235.

The distinction follows what the text explains. Permanent names describe the
table's organization; changing counts and instructions explain the current game.
The existing slot strip is a deliberate exception already accepted as printing:
its information belongs to the investigator mat and has an established reading
position. This proposal does not convert every changing caption merely because
it has a world anchor.

## Smallest credible implementation direction

Reuse the local technique already implemented by
[slots.rs](../../src/gathering_tabletop/slots.rs): Bevy text is rendered through
an offscreen camera into a transparent image, then that image is displayed on a
flat, unlit mesh. Its existing test checks that the surface lies flat and fits the
reserved mat band ([tests/slots.rs](../../src/gathering_tabletop/tests/slots.rs)).
The actual renderer path is therefore already present; no new font library,
generated artwork collection, shader family or generalized label framework is
needed for this proposal. Cache and share static label surfaces where practical;
do not introduce a permanent render pass for each repeated zone word by default.
Exact texture packing and resource lifetime remain implementation judgment.

Use restrained ink color, fixed world size and orientation, and no raised plaque,
dark UI badge, emissive glow, camera-facing rotation or screen-edge pinning.
Place labels on the visible felt/board surface with only the depth separation
needed to avoid surface fighting. They should be naturally occluded by real
components, so reserve a narrow header band where ordinary placement will not
cover them. Scenario/Test and investigator names belong along their own upper
edges; smaller zone names belong in their own region borders.

Reserve those bands as part of the layout before moving the assets. In
particular, center Assets in its upper border band rather than using the newly
recovered upper-left card space for text. Start the first asset immediately below
the reserved harm-counter clearance and close to the left boundary, with stable
cells that account for exhausted width and wrapped names. The heading band must
not recreate the large left inset or force all cards downward solely to retain
the old floating label coordinates. The existing region and surface geometry is
in [tabletop_layout.rs](../../src/tabletop_layout.rs), lines 416–579; current asset
anchors and card dimensions are at lines 224–230 and
[tabletop_pieces.rs](../../src/tabletop_pieces.rs), lines 10–11.

## Competing reading, example and limits

The strongest alternative is to retain screen-facing labels but anchor Scenario
only to its own edge, with bounded clearance. That is the smallest isolated repair
for the heading and preserves fixed-size lettering at oblique angles. It would
still look like UI hovering over the board. My recommendation is flat printing:
it fits the physical-table presentation of permanent zone names, has an existing
local precedent, and answers the owner's requested comparison.

For example, begin in Scenario focus, zoom toward the far edge, then pan sideways
and tilt. With printing, Scenario remains at the same position relative to that
edge; it grows or shrinks, tilts and becomes foreshortened on screen with the
board. It leaves the window when its physical position leaves view. With the
overlay alternative, the anchor moves but the letters retain their upright
screen presentation. **Fixed in the world does not mean fixed on the screen.**
Neither design should borrow displacement from the offscreen Test area or attach
the title to the top of the window.

World printing can become hard to read at shallow angles or overview distance.
Choose world sizes and contrast from actual rendered focused views, retain larger
area names for orientation, and preserve the existing camera controls and
readable dynamic information. Do not promise constant readable pixel size at
every zoom. Microsoft's [Xbox Accessibility Guideline 101](https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/101)
emphasizes text size, spacing and contrast and measuring the rendered letters;
it does not select between this application's world printing and overlays. It is
guidance for verification, not certification of the proposed design.

Remaining verification must cover actual text/texture loading, crispness and
contrast at the supported window sizes and scaling, focused and Overview views,
close zoom and shallow tilt, crowded assets and their counters, natural
occlusion, and no interference with pointer selection. Source inspection and the
existing slot-strip test do not establish those results or owner visual
acceptance. The Scenario report still needs a reproduction before a repair claim;
replacing the special projection path alone does not prove its reported cause.
