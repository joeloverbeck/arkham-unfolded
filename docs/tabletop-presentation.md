# Tabletop presentation contract

This document consolidates the accepted presentation requirements for supported
Gathering play. [ADR 0002](adr/0002-persistent-table-areas.md) records the spatial
decision and its rationale; the [play contract](foundation/play-contract.md)
owns the broader player, continuity and artwork boundaries. [Issue #89](https://github.com/joeloverbeck/arkham-unfolded/issues/89)
owns active work and acceptance. Requirements here do not claim implementation,
native verification, full-scenario completion or support for unselected content.

The [original decision record](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md)
preserves the interviews, superseded choices and dated evidence. Replay,
missing-art substitute play and the standalone experiments have been retired by
ADRs [0003](adr/0003-retire-replay.md), [0004](adr/0004-require-artwork-for-3d-play.md)
and [0005](adr/0005-retire-experiments-and-select-scenarios.md), respectively.
English names, effective text, ordinary captions and meaningful empty components
remain part of the required artwork presentation.

## Persistent scene and camera

Keep one persistent 3D scene with a shared Scenario surface, a separate shared
Test surface and a centered row of stable investigator surfaces nearer the
viewer. Their surfaces share the same height. Establish their positions at setup;
camera navigation does not move cards into a replacement investigator dock.
Keep useful space between surfaces without wasting navigation on large gaps.
Investigator surfaces match Scenario's width; Test is narrower and centered
between Scenario and the investigator row. Keep the gaps around Test equal.

Overview supports orientation. Focused views support readable play; simultaneous
readability of every card in Overview is not the aim. Do not shrink cards to fit
more investigator areas. A permanent map inset or split view is not selected.
Scenario and Overview framing include the enemy sideboard. Frame each view within
the usable viewport, accounting for headings and bottom controls; retain pan,
zoom and Inspect for details. Finite layout examples are not gameplay limits.

A persistent side strip uses investigator portraits to focus their areas, with
Scenario, Test and Overview shortcuts. Portrait buttons retain accessible names.
An investigator focus gives that area most of the scene view; the map may leave
view. A compact display retains the current decision, its answering investigator
and any ongoing test during ordinary navigation and action selection. Full card
inspection may temporarily cover it. Looking at another investigator changes
neither the active investigator nor the actor answering the decision.

Manual camera control is the default. Optional Follow may frame relevant play,
including an investigator's resolution tray. Manual pan, orbit or zoom overrides
Follow. Unmodified WASD pans on the table plane relative to the camera: W forward,
S backward, A left and D right. Preserve tilt and zoom, stop on release, and keep
continuous panning across UI updates without requiring a background click.
An ordinary focused button does not consume WASD. Starting manual navigation
dismisses a transient action list without making a choice or losing commitments.

Tab/Shift+Tab move menu focus, Enter/Space activate and Escape dismisses. Those
operations must not also move the camera; arrow-key menu navigation is not
selected. Inspect, History, Menu, story-reading panels, confirmation dialogs and
folder dialogs explicitly block camera input. Closing them restores WASD
immediately. Right-drag orbits; empty navigation-container space permits its
start, visible controls block its start, and an already captured drag continues
across UI. Cancel orbit on release, focus loss, cursor exit or a blocking modal.

Wheel zoom changes camera distance proportionally, using reciprocal inward and
outward factors and the existing distance bounds. Approximately five percent per
line-wheel notch is the selected starting sensitivity, subject to native feel.
Normalize continuous pixel-wheel input separately. Exact framing, spacing,
camera limits and motion timing remain implementation choices within these
relationships and readability requirements.

## Mats, cards and investigator areas

Use restrained playmat shading, thin boundaries and quieter permanent markings
than interactive feedback. Investigator mats, standee bases/holders and portrait
accents use muted class colors: Guardian blue, Seeker yellow/orange, Rogue green,
Mystic purple, Survivor red and Neutral gray/white. Tony remains Rogue despite
his secondary Guardian deckbuilding choice; Patrice is Survivor and Jacqueline
Mystic. Labels and shapes carry distinctions alongside color.

Print permanent area names flat on their own surfaces, in fixed positions and
orientations: Scenario, Test, investigator names, Assets, Hand, Threats, Deck,
Discard, Map, Victory display and Committed cards. Reserve header/border space
clear of components. Investigator names and Scenario/Test titles sit above their
own upper borders with a clear gap. Threats shares the Deck/Discard heading line.
These names follow perspective and natural occlusion; they do not face the
camera, pin to screen edges or move with another surface's projected gap.
Focused readability is required; constant pixel size at every zoom is not.
Keep changing counts, card captions, interactive enemy-group headings and
decision controls in their readable dynamic presentation.

Place each investigator card at upper left with permanent Damage/Horror space
above it and personal resource/clue/action markers below. Keep the card close
to its harm captions with readable clearance. Put deck and discard together
at upper right, discard to the right. Give piles outlined footprints. Separate
the conspicuous upper-middle threat band from the Assets region; its established
five-card arrangement is a layout example, not a threat-area rules limit.
Match Hand's width and side margins to Assets, and align the right edge of
Discard with Assets by moving the upper Threats/Deck/Discard grouping together.

Assets use flexible rows rather than equipment-slot boxes. Fill from the upper
left with modest margins and compact gaps, reserving actual ready/exhausted
footprints, harm/use counters and readable wrapped names. Exhaustion must not
rearrange neighboring cards. Keep assets clear of threats and the hand. Each
hand is one centered horizontal row along the near edge, with full-size,
individually visible, nonoverlapping faces: a single card is centered and an
even pair straddles the center. Examining a hand normally means focusing its
investigator; there is no screen-fixed hand strip that changes with focus.

Round physical card fronts, backs, piles and inspection images consistently with
the supplied artwork. Do not remove arbitrary black pixels from images. Each
physical Arcane Research copy has its own `Arcane Research` caption, preserving
its identity and effects instead of an aggregate first-copy/blank-second label.
Gathering assets and enemies exhaust clockwise from their upright orientation,
including exhausted unengaged enemies. Landscape orientation is separate.

Print a centered dynamic asset-slot strip between Assets and Hand. Show occupied
types and any type whose capacity is modified; inspection exposes all capacities
and their modifiers, including unused types. Count slot units, not asset cards:
slotless assets consume none, and exhaustion does not release a slot. Restricted
extra capacity has a visible qualifier, such as `Arcane: 3/3 (+1 Spell/Ritual)`.
Use the same committed rules state as the assets through save/reopen and Undo.
Keep the strip on the table plane and retain flexible asset rows. Replacement
actions name and highlight departing assets and explain resulting usage before
commitment. Preserve genuine replacement and discard-order choices without a
second confirmation for an already explicit single replacement. After costs or
intervening attacks commit, do not promise a cost-free cancellation.

Show `Mask: 1/1` beside that strip when the investigator controls a Mask.
This is an asset control limit, not an equipment slot or replacement permission.
Inspection explains the restriction; exhaustion does not release it.
Separate entries with spaced vertical bars, for example
`Hand: 1/2 | Body: 1/1 | Mask: 1/1`, preserving slashes inside the fractions.

## Component actions, inspection and availability

Selecting a physical card or component normally opens its currently legal
actions; selection alone makes no game choice. Use a compact nearby screen-space
panel with one full-width clickable row per option and Inspect last. Rows can
have a title and wrapped details. Keep actor, costs, test information,
opportunity-attack warnings and other consequences readable before activation.
Each option states its effective cost once, as currently modified (**Spend two
actions:**), followed by its effect; the effect does not restate the cost, and
counts agree in number. Keep panels inside the viewport without shrinking text or hiding details, and
retain predictable ordering as legality changes. Actions without a useful
physical source, including ending a turn and declining an effect, remain in the
decision controls. Compound selections retain all legal alternatives and an
explicit completion step.

Use a neutral resting action background. Hover brightens every available option
consistently; keyboard focus has its own outline without masking hover. Escape
or empty-space clicks dismiss menus without action. Observation revisions and
mode changes invalidate transient choices; prevent stale/double activation and
input through blocking dialogs or unfinished consequence presentation.

Exactly one Inspect operation opens inspection directly, never a lone gameplay
action. Multiple distinct inspection targets retain a chooser. This includes
single-card piles and the idle chaos bag's scenario-reference inspection. A
single enemy's action is simply **Inspect**; preserve exact physical identity
and full content. Inspection remains the existing artwork/effective-text modal
with Close/Escape. It neither acts nor moves the camera, discards pending choices
or loses a presentation selection. Opening it closes the contextual menu;
returning cannot dispatch a stale action. Routine actions do not require Inspect.
Inspection permission follows the displayed face and revealed history, including
stale callbacks; it must not disclose an unrevealed reverse early.

The availability cue means that clicking the component offers at least one
gameplay choice available now. Include present reactions, commitment/removal and
token choices, not only actions spending an allowance. Inspection/dismissal alone,
an unused allowance, matching icon, later window or absent reaction trigger does
not qualify. Derive the cue from the same offered choices and activation guards
as contextual interaction. Blocking dialogs, admission and consequence intervals
must not advertise unavailable choices; an ordinary contextual menu can retain
the cue. Investigator cards and standees agree. Personal resource/clue counters
are passive totals: investigator actions/cues belong to the card and standee.

Available cards have a faint, slowly breathing teal edge; models, piles and
tokens have a matching footprint/rim halo with a visible low-intensity baseline.
Reduced motion uses a steady cue. Keep pale hover, blue menu-source identification,
gold selection, commitment badges and ignored/canceled/spent marks distinct.
Do not rely only on color. The unified cue replaces the earlier violet
player-window bar, separate teal commitment edge and blue token-availability ring.
An inspectable component remains hoverable without a gameplay choice: use a brief
fade to a steady restrained outline, clearing on pointer exit, loss of focus or
blocked interaction. Reduced motion uses a static hover treatment.
Menu-source feedback persists while the pointer moves to
the menu, then clears with it; leaving the card does not close the menu.

## Player decisions and commitments

Retain explicit Begin/End turn controls, a persistent named turn indicator and a
distinct text-bearing badge on the active investigator's portrait. Keep these
independent of camera focus, testing investigator and supporting actor. Name the
performer of supporting actions; an ended-turn investigator can still supply a
legal commitment or reaction. Remove redundant resource/clue/action totals under
the turn heading while keeping setup/mulligan prompts understandable.

Omit the commitment prompt only when no investigator has a legal commitment for
the actual test. Check every eligible contributor, location, limits, restrictions
and card exceptions. Ordinary matching/wild icons and the one-card limit for
another investigator at the same location do not replace those complete checks.
Advance to the next genuine decision without skipping available abilities,
mandatory effects or windows, or revealing a token prematurely. Keep meaningful
Undo to the originating choice and omit empty-step administrative destinations.

When any legal commitment exists, retain Commit no cards and explicit selected
confirmation. Selecting the last eligible card does not empty the choice: the
player can still remove, decline or confirm. A supporter who selected their
permitted card must unselect it before adding another; do not silently swap it.
Keep selection through inspection/menu dismissal. Show legal addition/removal
through the common availability cue; selected cards keep gold corners and
explicit selected status. Clear live commitment cues outside the opportunity.

Each option and selected total shows its effective `+N` contribution followed
by the tested skill glyph, including wild contributions without changing their
printed wild identity in Inspect. Legally committable cards carry readable
top-edge badges with that effective contribution and tested-skill glyph;
selected cards retain their contribution badges. Keep owners and other effects, such as Promise
of Power's curse, visible. Ineligible cards remain inspectable. Watch This!
immediately requests an explicit editable wager per selected copy: zero for
icons only, or one to three resources up to available funds. Confirmation waits
for a choice; never silently choose zero. Pay once on confirmation, not editing.

Player-window pauses name actually playable cards/abilities and their investigators,
with matching card cues and explicit Pass when a genuine opportunity exists.
An enemy phase advances automatically only when it has neither effects nor legal
choices, retaining its phase-entry announcement and an **Enemy phase — no activity**
history entry. No enemies alone does not establish that condition. Automatically remove
Old Keyring's key after an ordinary success and discard the empty asset, while
retaining genuine result-order choices directly in the result panel; Resourceful
can make removing the last key before recovery matter.

Crystal Pendulum uses a compact choice grid headed **Exhaust Crystal Pendulum**:
direct **Predict 0** through **Predict 10** choices and **Other margin…** for direct
numeric entry. The quick choices are not a rules cap. Selecting a number exhausts
once and records that prediction; dismissal before selection spends nothing.
Do not expose separate cancel-prediction or one-step increment/decrement actions.
This local numeric picker is an exception to the ordinary full-width action rows.
Retain supported pending predictions through save/reopen without resetting their
number. After selection, automatically close an empty test-start reaction window.
If legal reactions remain, name them and offer **Skip remaining reactions**.
Continue to the next genuine decision, preserving commitments and other legal
abilities before token reveal.

Patrice's Violin offers one **Use Patrice's Violin…** entry instead of a row for
every discard-card, recipient and benefit combination. Open one compact nearby
panel with three independently editable groups: each eligible discard card once,
with artwork, name and inspection; eligible investigator portraits and names; and
**Draw 1 card** or **Gain 1 resource**. Keep all legal alternatives, including
distinct physical copies, without choosing strategy for the player. An unusually
large hand may scroll within the card group; never turn combinations back into
repeated rows or require a wizard to revisit a selection.

Show a short cost/result summary and an explicit **Use Violin** button. Require a
complete legal selection before confirmation. Editing selections neither discards
a card nor exhausts the Violin; confirmation commits the existing complete game
choice once. Cancel/Escape spends nothing. Retain the common inspection, keyboard,
revision and save-before-install safeguards. This is a local compound chooser,
not a new game timing window or a replacement resolution surface.

## Test area, planned reveals and results

Test is dedicated to skill tests, with a stable source, commitments and physical
chaos-token presentation. Keep the scenario/difficulty reference at upper left
and the bag at upper right between tests, reserving separate source-card space.
Keep the curse supply there, aligned with the bag and its caption clear of
Committed cards. Reference inspection remains available through its component
and existing bag/token shortcuts.

The chaos-pouch replacement selected on 2026-11-08 uses the supplied static,
open, empty petrol-blue velvet pouch, with dark lining, ivory drawstrings and
brass tips. Preserve the intended velvet sheen under the game's lighting;
successful model loading or factor parsing alone does not establish that
appearance. Compare the in-game result with the supplied intended preview.
The [source contract](foundation/sources-and-verification.md#presentation-asset-preparation)
owns the external artwork and preparation boundary.

Anchor the model's floor origin on the table, fit it to the bag's existing home,
and keep its cords clear of neighboring components and labels. Preserve bag
selection, inspection, availability/hover/menu cues, labels and token reveals.
Keep those controls separate from the authored materials. The existing
breathing teal footprint halo appears when the pouch offers a current gameplay
choice beyond Inspect; inspection alone leaves it off. Reduced motion keeps the
available cue steady, and blocking dialogs suppress unavailable choices.
The existing
screen-fixed reveal control remains independent. This selection adds no cloth
simulation, rigging or new bag animation and does not change game decisions or
save continuity. In-game appearance and interaction still require verification.

Use one token region as wide as the commitment region above it. Fill upper-left
to right with comfortable compact gaps, wrapping inside the region and including
selection marks in footprints. Tokens cannot spill onto an investigator mat.
An ordinary Jacqueline three-token reveal forms one row; the original draw is
not privileged over the additions. Preserve meaningful nested groups with compact
cues instead of a whole row per source. Current selection scope can cross origin
groups; distinguish it, provide one clear instruction and show selection progress.
Do not retain separate old lower revealed/canceled regions. Canceled and ignored
states remain clear through shape/text as well as color and preserve custody.

Mark a currently selected chaos token with a bold green check on an opaque dark
circular badge overlapping its lower-right edge. Keep the printed modifier or
symbol unobstructed and retain the gold selection rim. The check's shape and
contrast against its own backing must remain readable at the actual camera scale;
color alone does not carry selection. Include the badge in the token's layout
footprint so it neither overlaps neighboring tokens nor crosses the region edge.
Keep availability, hover, ignored slashes and canceled X marks distinct. The
check means selected for the current choice, not revealed, resolved or successful;
the instruction states whether the player is choosing tokens to cancel or resolve.

Duplicate faces remain distinct physical targets. Remove routine face-name and
ordinal captions; retain meaningful group/state information, instructions and
names through inspection and decision controls.

Before faces are known, present available abilities and their legal targets as a
visible plan. Name source and scope, such as **Voice of Ra — entire reveal** or
**Voice of Ra — draw 1**, without raw paths or zero-based positions. Preserve legal
orders, costs and pre-reveal declaration. Show the declared plan, for example
**Olive active — reveal 3, resolve 2**, with **Use Jacqueline...** separately visible
when available. Main Reveal executes that plan directly, forgoing unused optional
abilities rather than reopening their chooser. Adding an ability updates the plan
before revealing. Opening/dismissing its chooser neither activates nor undoes it.
Preserve pre-reveal Undo, supported save/resume and stale-action rejection.

For a proven-equivalent ordinary singleton reveal, offer one genuine **Use
Jacqueline — reveal 3 tokens** action with the cancellation explanation. Distinct
scopes and Arbiter costs use an aligned chevron disclosure with consequences
explained. Equal physical draw counts alone do not justify collapsing choices.
Explain who tests and each affected scope; Jacqueline helping someone else does
not become the testing investigator. Guide cancellation and Olive's selection at
their actual stages, never creating a response opportunity after learning faces.

During resolution, expose direct legal token/group selection controls with all
eligible targets highlighted together. Do not require exploratory clicking or
discovering a bag menu; retain contextual access alongside the visible controls.
A nested group may be one parent candidate containing several physical tokens.
In the researched Voice of Ra case, show Dark Prophecy's five-token group first,
then Jacqueline's scope across five remaining parent results, not only her two
additions. Explain relevant unavailable abilities, including Katarina's restriction
to her controller's skill tests. Keep test tokens in Test and non-test tokens in
their investigator tray; use no replacement token modal.

The screen-fixed reveal control remains at lower left, with the existing
16-logical-pixel gap to its adjacent panel and fixed-width centered **Reveal tokens**
or **Resolve tokens** text. Keep a decorative pouch above it with a gentle bob;
only the stationary button acts. Reduced motion holds the image still. Stop its
reminder and input when unavailable or another dialog takes focus, even if a
decorative disappearance is unfinished. The pouch must not imply a particular
token result. Keep the physical bag's controls and their existing camera behavior.
Submit through the existing revision-bound save-before-install choice; a skill-test
reveal shortcut focuses Test only after successful commitment. Failed save or
stale rejection leaves game, randomness and camera unchanged. The adjacent panel shows the complete
Premonition explanation; fit it to content with a viewport cap and scrolling.

Keep explicit token Resolve followed by automatic consequences/cleanup only when
no genuine decision remains. Preserve cancellation, resolution order, additional
curse reveals, eligible reactions, Lucky!, Patrice's elder-sign choice and Act
advancement at their proper timings. Optional effects such as Quick Thinking
appear directly in the test flow with explicit use/decline; Continue never
silently declines them. A generic administrative finish prompt is unnecessary.

Every completed test, including encounter tests, has an untimed result modal and
explicit centered Continue. Show prominent outcome/margin, one total-versus-
difficulty comparison, quieter investigator/source context and aligned contribution
rows. Full meaningful detail is visible by default: base skill, each card and owner,
abilities/paid bonuses, each revealed token, final total, difficulty and margin.
Keep zero-valued tokens and separate canceled/ignored states; omit empty groups
and duplicate outcome text. Show unknown chaos as pending, not zero. Show special
outcomes such as automatic failure honestly, not as ordinary `+0` arithmetic.

Numeric tokens whose face equals their contribution show one value; symbols show
explicit resolved values. Distinguish a modified numeric contribution from its
face. Keep one row per physical revelation. Fit height to content up to the usable
viewport; long details scroll with outcome/controls visible, short reports have
no forced blank space. History uses the same frozen report, not later state.
The result precedes its consequences; acknowledgment cannot reroll, repeat effects
or create a rules opportunity. Preserve actual timing of result-changing effects.

## Investigator resolution trays and Premonition

Route searches, encounter/weakness revelations, non-test card/token effects and
pending discard-order choices to the originating investigator's temporary 3D
drawer extending toward the viewer. A tray is presentation, not a rules zone.
It retains that investigator association through other investigators' nested
responses. A test already underway does not make a nested search/revelation
test content: preserve the parent test while it resolves separately. A revelation
that starts a test uses Test without duplicating its physical source.

After each actual encounter-deck draw arrives, pause for an untimed reading
overlay: dim the table, center a large upright intact card, label it **Encounter
card**, name its actual drawer, and show its title and printed type. When that
encounter still awaits resolution, show its actual legal responses (such as Ward
of Protection or A Test of Will) on this screen. Explain that **Continue** resolves
the encounter unless a response is chosen first. Continue commits the existing
resolution choice directly, so a mandatory test starts without another Resolve
click in the investigator's drawer. Bind choices to the live revision and this
physical encounter; retain the screen and display the error if saving fails.
Already committed encounters need only reading acknowledgment. Preserve the
existing tray/Test resolution afterward, including each enemy's own Spawn and
engagement rules. This surface hosts reading and existing pre-revelation choices;
actual effect resolution retains ADR 0002's surfaces, with no new rules window or zone.

Hold the captured draw view before presenting consequences that the rules engine
may already have committed in the same command. Never split or repeat that command
to support reading. Bind the stop to the actual draw occurrence and physical card,
not its title or a later response revision. An unchanged scene rebuild or in-place
reopen retains acknowledgment. A fresh process may re-offer an unresolved,
unstarted encounter for reading, but must not invent a draw or rewind a started
resolution. Search candidates, selected On the Hunt enemies, and investigator-deck
weaknesses are outside this reading surface; On the Hunt's actual fallback draw
uses it normally.

Keep card, heading, text and controls still. A few dark, low-contrast tendrils bend
slowly in clipped side margins, clear of all reading content. Joel's 2026-11-08
feedback removes the local Pause/Resume buttons; global Reduced motion still uses
a still surround. Pause decoration when unfocused and resume without catching up.
Keep responses, Continue and **Read rules** outside the scrolling body, with button
labels contained and focus outlines separated. Read rules provides readable
effective text at small viewports. There is no countdown, flashing, camera movement
or audio cue.

Give Hornet's nine visible candidates a 3-by-3 arrangement with its event alongside,
and Backpack's six-card search its explicit completion. Display all cards the
search permits seeing but offer game actions only for legal candidates. Preserve
selection, decline and order; searching is not drawing/resolving every revealed
card. Individual faces use their normal contextual menu and Inspect; the drawer
surface retains aggregate search/resolution/discard-order access without an
artificial summary card. Physical motion follows actual identities/destinations.

A temporary Resolve shortcut frames the tray. Opening it does not force a camera
jump; optional Follow may frame it. Save/reopen reconstructs pending resolution
without repeating effects; reduced motion settles immediately. Retract the tray
after its contents reach their destinations and no resident/resolving component
needs it. A pending effect's investigator association alone does not keep an empty
drawer open: when its only card moves to Test, retract it after that transfer.
Its Resolving caption and Resolve shortcut disappear with that empty content.
Open it again for incoming content before arrival; retain any other resident
cards or tokens, including through nested tests. Exact dimensions and motion must
support readable interaction.

Premonition and its sealed token/group remain together in the controller's open
drawer until consumed; later temporary effects use space alongside them. Tokens
remain visibly on the card after sealing animation, with the existing **Sealed**
caption and no selection check merely because they are sealed. Reserve checks for
current selections. Announce the known sealed face/group on the existing reveal
control with prepared symbolic glyphs or signed
numeric values, and state any additional fresh draws, without another acknowledgment.
For example, **Resolve sealed [tablet] from Premonition**. Use actual tablet and
auto-fail glyphs, not emoji or invented numeric modifiers.

On consumption, animate the token/group from the card to the appropriate resolution
area, then Premonition to its controller's discard. Retract only when nothing else
needs the drawer, including a consuming non-test effect. Token cleanup follows
its real destination/timing, not the event's discard animation. Reduced motion
preserves destinations and explanation without requiring travel.

The [qualified Premonition interpretation](research/issue-89-premonition-followup.md)
replaces one reveal position with the entire sealed group. Eligible Jacqueline
can modify the initial sealing reveal but cannot reopen a position already
replaced by Premonition; remaining fresh positions retain legal opportunities.
This is a source-derived interpretation, not a located exact publisher ruling
for every pairing. Presentation selection does not strengthen that evidence.

## Scenario, locations and enemy sideboard

Show Gathering's fixed initial Scenario/Test arrangement from the first fresh-start
or restart setup screen: Study, initial clues/standees, initial Agenda/Act faces,
face-down encounter deck, empty pile footprints, reference, bag and curse pool.
Lead selection and mulligans remain the actual choices; no early ordinary action,
hidden deck order, later face or set-aside content is exposed. This is early display,
not a change to authoritative setup order, randomness or outcomes.

Place encounter deck/discard at upper left, deck left; Victory display at upper
right. Label their footprints. Center joined Agenda/Act along the far edge,
Agenda left and Act right, with touching inner edges forming an open book.
Both remain landscape, almost upright and leaning slightly back, independently
selectable/inspectable. Keep them near the far edge and clear of the flexible
Map region, its locations, connections and standees.

Center a low, broad altar in the open band in front of the Agenda/Act pair,
leaving clear space before both the cards and the Map region. Use a rectangular
dark-slate plinth with two shallow tiers, clipped corners and restrained brass
edging. Keep its surface, labels and token low enough to preserve card reading,
selection and inspection in Scenario focus and representative oblique views.
Exact dimensions remain native-tuning choices; the altar is presentation for
agenda doom, not a game zone or new gameplay action.

Print **Total doom: N/threshold** on the mat beneath the altar, centered on its
axis and fixed in the Scenario mat's orientation, in warm ivory using the existing
readable font. It remains selectable for its breakdown. The altar's counted token
already shows agenda doom; omit the duplicate Agenda doom line, including at zero
when the altar is empty.
Preserve readability and clearance in normal and oblique Scenario views, including
populated encounter piles, Agenda/Act cards and Map controls.

Standees retain rounded bases and rectangular holders, scaling as complete objects.
Use up to four slots outside each location card, two along each long side, with
positive clearance from the card and narrow gaps between neighboring bases.
Keep deterministic investigator order, filling left then right; empty slots have
no visible markings. The arrangement reflects investigator location, not human
player count. Scenario's normal view leaves cards unobscured; natural overlap
during orbit does not make figures fade or reposition. Inspect remains clear.

Show the three reciprocal Hallway links from visible connection symbols, keeping
Parlor's entry restriction separate. Brass arrows are restrained/non-pickable and
route around cards, reserved standee and caption footprints. Routing responds to
layout, not camera movement. The normal Gathering arrangement puts Hallway at
the hub, Attic above, Cellar below and Parlor beside it, with footprint-based
spacing. This is authored layout, not a source-prescribed diagram; later actual
source-prescribed arrangements take precedence. No force simulation is selected.

Location-resident enemies use an exposed physical sideboard left of Map, grouping
cards beneath full location names in three columns with further exposed rows.
Keep full card size, stable group/copy ordering and clearance for exhaustion,
harm, side counters and names. Do not collapse, overlap, scroll or continually
shrink cards to solve crowding. Keep the compact board the same depth as the
Scenario/Map board, with distinct navy/steel-blue treatment and ordinary readable
text. Full text remains available through zoom/Inspect; do not hide needed names
or counters merely because framing widens.

Use the same thick table body and inset top as the other playing surfaces,
retaining the sideboard's navy/steel-blue palette and footprint.

Each nonempty group has a marker above its location, repeating stable color,
short identifier and group count beside its sideboard heading. Identity survives
emptying/refilling. Keep markers distinct from printed symbols/gameplay tokens,
outside investigator slots; route links around them without changing connections.
Counts cover sideboard cards only. Ordinary engaged enemies stay in investigator
threat areas. If Massive content becomes supported, its single location-resident
card belongs on the sideboard ready or exhausted; only a ready Massive enemy is
considered engaged with everyone there. This convention adds no content support.

Card transfers preserve identity/counters and synchronize positions, markers and
counts to the same displayed moment. Each face offers its own actions/Inspect;
hover or keyboard focus highlights matching marker/heading without camera travel.
Markers/headings retain aggregate group access. A moving enemy uses a temporary
individually named marker along its route; permanent location-group identity stays
fixed. Do not add permanent enemy standees or occupy investigator slots.

Encounter discard is ordered and face-up, with top face, count and access to every
card; one card inspects directly, several use a chooser and empty is visibly empty.
Respect actual destinations: player weaknesses belong to investigator discard,
Victory/card-specific effects retain their exceptions, and an unfinished revelation
does not become complete merely by leaving play. Keep final harm presentation
before transferring a defeated enemy. The compact Victory pile separately shows
card count and summed Victory points, including `0 cards · 0 VP`, and allows every
card's inspection. Exclude cards still on Map and resolution bonuses; this is not
earned experience. Victory locations move only when scenario-end rules require;
Victory is distinct from encounter discard and its reshuffle lifecycle.

Act advancement saves payment/disclosure before its dedicated reading stage.
For Act 1b, show **Act advances**, **The Door on the Floor**, story and effects;
Continue applies the mandatory transition and reveals Act 2. No dismiss-to-play
or Undo across newly revealed information is allowed. Do not repeat committed
payment/effects on reopening. Story panels fit content up to the viewport, with
heading and centered Continue visible while longer text scrolls. Retain readable
artwork/text and the existing flow's continuation semantics.

## Counters and allowance markers

Use the owner-selected clean 512px red-and-black Doom Counter face for doom;
the [round-flow investigation](research/gathering-round-flow-and-pendulum.md#doom-artwork-candidates)
retains its exact source identity and comparison. Preserve the distinction
between doom on the agenda, doom on individual enemies and total doom in play.
Artwork selection does not change those rules-defined owners or threshold timing.

Show agenda doom on the altar as one solid Doom token with a readable count and
**Agenda doom** label. Each actual addition visibly arrives and increments the
count on arrival; do not build a tall token stack. At zero, retain the empty altar
and **Agenda doom: 0**, without a physical token. Enemy doom stays beside its
bearer. Show a separate **Total doom: N/threshold** readout near the agenda, with
an inspectable breakdown such as **2 on agenda + 1 on enemies**. This summary
neither duplicates physical doom nor changes advancement timing. On advancement,
remove doom at each actual display location without moving enemy doom to the
altar. Preserve reduced-motion equivalents and nonduplicating save/reopen behavior.

Use one physical counter with a separate readable number per total, retaining the
selected artwork/emblem and unobscured full-card inspection. Clues use the intact
green/gold emblem on a solid round token, including number `1`; locations center
it over the printed LOCATION/set area. Remove only the redundant clue-count
suffix, keeping name and statuses. Zero clues means no counter/number. Resources
use the shaped crate with a solid profile following its silhouette, left of clues
below the investigator card. Their stable slots hide independently at zero.
Count labels use compact opaque dark backings fitted to rendered text for static
and moving counters; keep marker outlines and other labels clear.

Damage/Horror use thick solid disks with corresponding artwork above investigator
and eligible asset cards; show accrued harm over effective capacity, including
zero. Omit a type only if the asset lacks that capacity. Use the accepted compact
ammo/bounty-sized disks, keeping captions clear of card edges; harm fractions
have light text along the lower edge on a fitted nearly opaque black badge with
small padding and restrained rounding. Preserve distinct printed card information.

Card use types retain rules meaning: ammo, bounty and charges use their distinct
graphics; charges are deep-violet disks with the exact supplied pale Spell motif,
restrained rim and separate count. Supplies share resource artwork but remain
named **Supplies** in Inspect/context; they are not resources or charges. Keys use
the selected neutral key artwork with its entire silhouette and **Keys** name,
not Charges. Gregory Gry shows resource uses on one shaped resource counter,
decreasing immediately when wagers are spent rather than after the test result.

Offerings use the unchanged white Chalice drops icon on a compact solid rounded
shield in fixed slate blue (`#40566D`), independent of investigator class, with a
separate readable count retained at zero. Remove Sparrow Mask's redundant
`Uses N` caption while preserving its name, status and full inspection text.
Menu → Credits provides the icon's linked Lorc/CC BY 3.0 attribution alongside
the existing Swap bag credit; the recipe and prepared manifest retain provenance.

Retain ammo, charge, Supplies, Gregory Gry and Bounty Contracts counters at zero
while their cards remain in play. Other card-attached counters leave with their
card. Center Bounty Contracts' counter; place ammo/charges consistently. Enemy
bounty is left and doom right, omitted at zero; assign added supported types a
stable balanced side without switching when another disappears. Protect printed
damage/horror icons. Center occupied side counters with gaps. Enemy damage sits
above the card, including on the sideboard, as accrued/effective maximum health
with zero retained. Remove only genuinely redundant count captions, not titles
or unrelated state.

Center class action markers beneath resource/clue counters. Spent markers remain
crossed and dimmed; unspent markers after an early turn end are subdued, not
crossed. An uncrossed marker before a turn does not permit interruption. Tony's
restricted action has an additional Rogue disk separated by a small gap, with a
raised `+` rim badge; center the full row. Retain allowance spending/eligibility
display but no permanent **Fight / Engage — Enemy with bounty** caption. Inspect
and legal action wording explain the restriction. These are reminders, not action
buttons or payment-choice controls; do not fabricate unrepresented allowances.

Jacqueline's ability spark sits to the right of her investigator card, centered
along that edge with a clear gap, matching action-marker diameter/thickness,
class background and spent cross. Use the selected prepared inner-spark glyph.
The marker is noninteractive and shows the once-per-round allowance, not current
legality. Preserve it off turn; ordinary use spends it, an Arbiter-assisted use
preserving the allowance does not, and the rules' round reset restores it.
This establishes the convention for supported once-per-round abilities without
adding investigators. Artwork identities/provenance stay in the preparation recipe.

## Movement, draws, shuffling and automatic bookkeeping

Announce each entered phase with a prominent navy/teal-and-brass banner carrying
the phase name and round number. Start around two seconds total: a brief entrance,
stationary reading interval and brief exit, with timing subject to native tuning.
Present it before the phase's visible processing continues; it needs no click.
Keep current round and phase clearly visible after it leaves. Reduced motion uses
stationary text. Announce an empty Enemy phase and the first Investigation after
setup; the first round skips Mythos. Round-end effects do not create a fifth phase.
Prevent input through the announcement from activating an underlying control.

Perform compulsory Upkeep and Mythos administration automatically with visible
consequences, preserving genuine decisions, reactions, ordering choices, tests,
hand-limit discards and story-reading stops. Banner timing has no rules authority:
save/reopen, Undo, inspection and scene rebuild must neither repeat effects nor
lose a pending choice. Between investigator turns, clearly name the remaining
turn selection instead of claiming there is no pending player decision.

Animation presents successfully committed rules consequences; it has no rules
authority. Preserve identities, actual origins/destinations, ordering, legal
choices, hidden information, randomness, supported save/reopen and Undo. Do not
repeat effects after interruption, scene rebuild, camera movement or reopening.
Reduced motion reaches the same final state with meaningful explanation and
immediate placement. Cosmetic steps do not split simultaneous rules effects.

A moving investigator lifts with its full base/holder onto the midpoint of the
connecting brass arrow, rests there briefly at base height, then lifts into the
destination slot. Engaged enemies travelling with it hop and rest in step beside
it; others remain stationary. First entry then flips the actual location faces in
place and lowers its counted clue token with a brief gold/green glint. Target
about two and a half seconds for a first entry; the hop, rest, flip and clue
timings are starting values for native tuning. Revisits omit flip/clue placement
and no extra Continue is added. Zero-clue revelation creates no clue token,
enemies do not reveal rooms, and scenario placement invents no connection. Use
location names in destination labels rather than raw identifiers.

After clue gain commits, lift a marker, carry it in a gentle raised arc and lower
it to the investigator's clue slot. Decrease source count on departure, increase
destination count on arrival; the final source clue leaves no stationary token
and the first destination clue creates its counter. A multi-clue transfer uses
one marker labelled with quantity. Animate only actual supported transfer
provenance, not guessed count deltas. No rendered hand or additional resource-
transfer/clue-spending animation is selected by this convention.

Each drawn card slides from its actual deck, lifts in a shallow arc with restrained
tilt, turns face-up and settles while existing hand cards make room. Finish one
card before the next; target three to four seconds for five, dealing opening hands
one investigator at a time in lead order. Cover opening hands, mulligan replacements,
action/upkeep/card-effect draws, Patrice's refill and encounter draws. Mulligans
leave hand for a temporary set-aside pile until replacements finish, then return
to deck. Encounter draws reveal on the investigator tray, with actual tests in
Test; cards awaiting choices remain at their resolving home. Search placement
is distinct from a top-deck draw and does not invent hidden deck order.

After each actual shuffle, animate a compact face-down two-packet interleave,
square and settle at that deck, initially about 1.3 seconds. Return travel comes
first. Preserve shuffle/draw/search/nested-effect order and randomness without
revealing hidden cards. Reduced motion settles immediately with a readable shuffle
account. Exact trajectories and timing adjustments remain native-tuning choices.

A held physical chaos token has one current destination and cannot be drawn again
while outside the bag. At cleanup move it visibly to the bag or applicable pool,
removing its Test/tray representation. Resolved curses go to the pool; canceled
curses follow the accepted return-to-bag rule. A frozen result/history token face
is not evidence that its physical token is still held.

Ordinary live consequences finish automatically and return to the next genuine
choice: no Next event, Pause/Resume presentation or Show pending choice controls,
and no generic caption-length delay after routine actions. Navigation never
pauses this progress. Inspection/blocking dialogs may suspend it temporarily;
closing them must not leave a persistent pause. Preserve readable motion, draws,
harm, token results and cleanup, all real reactions and explicit story/result
reading stops. After Read the Signs finishes its test and genuine response
choices, perform existing event-to-discard cleanup automatically, including a
save resumed at that step, without repeating costs or effects. Saving failure
preserves the committed boundary and requires safe explicit retry rather than
an automatic retry loop.

After Hornet's Nest spawns its selected enemy, award the available clue and
enemy-health resource reward automatically before the optional Bounty Contracts
reaction, following Then priority. Preserve that real choice and automatically
finish remaining shuffle/discard cleanup. Retain no-match disclosure and retract
the drawer only after motion and any other resident content have finished.

## Text, symbols and diagnostics

Use prepared willpower, intellect, combat and agility glyphs consistently in
visible semantic labels. Render recognized card-text symbol notation without
surrounding brackets, retaining ordinary action words such as **Fight**, original
artwork and readable English inspection/history/agent descriptions. Printed wild
identity remains wild. Use generic difficulty where it is not specifically shroud;
keep the word shroud until a genuine verified glyph is selected, not another
stat's symbol or an invented icon. The font has no bounty glyph; do not replace
card-text “bounty” with an approximate symbol. Use supported characters for Restart
and preserve full focus-outline clearance on confirmation controls.

Retain the initial graphics-check notice and actionable failures. Routine healthy
conditions, sampling rates, warmup and successful admission details belong in
opt-in debug logging, while monitoring and play-admission behavior remain active.
