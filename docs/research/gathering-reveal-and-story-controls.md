# Gathering reveal and story controls

Date: 2026-11-06. Source fixed point:
`71c4c3e52752ca892f91c589579ec88d28a65ee1`.

This supports the owner-requested `grill-with-docs` discussion under
[#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89) and its
[current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060).
Findings describe source and supplied screenshot inspection. Joel accepted all
three recommendations during this interview, then authorized implementation once
the design was complete. The investigation itself made no application changes,
native reproduction, new tests or publication. Implementation and its evidence
are recorded separately below.

## Reveal button and attention cue

The screenshot `/home/joeloverbeck/Pictures/Screenshots/arkham-overlapping-bag-button.png`
shows the Premonition control covering the adjacent token panel. Its SHA-256 is
`2ff2e3dfb0e1e37ad8d8ff7d1343c169c81f89663181689d72d404ce913d45a5`.

`src/tabletop_reveal.rs:483–490` positions the control at logical x=16.
Its normal width is 118, but a special caption changes it to 240
(`tabletop_reveal.rs:547–563`). The adjacent Gathering panel starts at x=150
(`src/gathering_tabletop/ui.rs:506–518`). The normal gap is therefore 16 logical
pixels; the caption-specific width instead overlaps the panel by 106. Centering
the text alone cannot repair those bounds. Centering the text entity also does
not center its wrapped lines; rich text needs an explicit alignment choice.

The pictured action resolves a known sealed Premonition token with zero fresh
draws. The complete caption discloses sealed faces and any additional draws
(`src/gathering_tabletop/reveal.rs:7–33`). Keep that information visible in the
adjacent panel when shortening the button. Its current reveal-plan and draw-count
text does not independently carry the complete announcement (`ui.rs:119–127`).

The existing bag image is a 42-pixel child of the button
(`tabletop_reveal.rs:373–386`). Move it above the button as a decorative sibling;
animate only the image so the hitbox and neighboring gap stay stationary. Use the
existing motion preference and offer/modal suppression supplied by Gathering's
adapter (`gathering_tabletop/reveal.rs:43–61`). Reduced motion keeps a static cue.
The current 0.14-second fade/scale is only an appearance transition, not a reminder
loop (`tabletop_reveal.rs:438–447,573–595`). A smooth reminder must request redraw
while active; settled presentation otherwise uses a 250ms reactive cadence
(`src/tabletop_pacing.rs:64`). Stop those requests when the reminder is inactive
or motion is reduced; pause motion while unfocused. The reminder adds rendering
work while the player considers a pending reveal.

**Accepted Q1:** Joel answered "Yes, use this design": retain the fixed-width
button and 16-pixel gap; center a short **Reveal tokens** or **Resolve tokens**
label; show the full Premonition explanation beside it; place a decorative bag
above it with a gentle repeating bob. Only the button activates the action.
The image stays static with reduced motion, and the reminder stops when the action
is unavailable or another dialog takes focus. The existing readable absent-art
behavior remains required. This avoids wrapping a complete multi-token explanation
into a tall narrow button, and avoids motion implying a fresh draw when none occurs.

## Jacqueline's ability and the empty token-panel space

### What the screenshot establishes

`/home/joeloverbeck/Pictures/Screenshots/arkham-weird-ui-gap.png` shows a pending
skill test, one planned physical draw, a `Jacqueline / choose target` control,
substantial blank space, then reveal and undo controls along the panel bottom.
Its test summary names Jacqueline Fine. The owner separately reports encountering
the Jacqueline control during Tony's test; the screenshot itself does not record
that second situation. Screenshot SHA-256:
`181a9f64b66ef7ab1b378b388022887357d0a23bb89921e9c3d5693960f988b5`.

### Meaning and rules evidence

The button names the source ability. It does not select Jacqueline as the testing
investigator or the recipient of another investigator's reveal. Its first click
only toggles the visible ability-choice category in
`src/gathering_tabletop.rs:715–717`; it commits no game choice. The button is
created for every available `tokens:jacqueline...` category in
`src/gathering_tabletop/ui.rs:140–157`. The subsequent controls expose the offered
rules choices, including the scope and any Arbiter of Fates alternative
(`ui.rs:162–179`; `src/gathering/tokens.rs:1264–1285`).

Here, “target” means a pending reveal or a draw within it. The labels derive from
the reveal structure: `Skill test · whole reveal`, `Skill test · draw 1`, or
source-labelled nested scopes such as Dark Prophecy within another reveal
(`src/gathering/tokens/presentation.rs:212–238`). It is not a target investigator.
The existing glossary already distinguishes the testing investigator from other
contributors and defines a reveal group.

Jacqueline can use her reaction when an investigator at her location is about to
reveal chaos tokens. Her reaction adds two draws and then cancels two eligible
non-auto-fail results or one auto-fail, ordinarily once per round. This allows
Jacqueline to assist Tony while Tony remains the testing investigator. The
original English investigator card was inspected directly in the unchanged
external archive:

- Card `60401`, variant `ba5b602644842ee4`,
  `images/cards/official/en/sce-base-save/Investigator Packs/60401--Jacqueline Fine/ba5b602644842ee4/face.png`.
- Image SHA-256:
  `01c1bcf5bbef4356ee2c0224ff8b7ffad87ab1584a53533da53c0857eaa7b3da`.
- [Card text navigation](https://arkhamdb.com/card/60401); the inspected card face,
  rather than community commentary, supplies the relevant ability text.

Current code checks that Jacqueline has an unused reaction, is neither defeated
nor resigned, and shares the revealer's location
(`src/gathering/tokens.rs:1257–1262`). Consequently, merely seeing her ability
during Tony's test is consistent with the card and the implementation. This
inspection does not prove the exact saved Tony situation satisfied all conditions.

The publisher's [Legacy FAQ 2.5, page 17](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc_faq_v25_february_2026-web-1.pdf)
explains that nested reveal replacements must be declared before the simultaneous
reveal's faces are known, and that a nested result may remain one candidate for
its containing instruction. That supports preserving meaningful scope choices.
Its worked example uses Olive McBride and Grotesque Statue; applying that general
guidance to Jacqueline compositions remains the source-derived interpretation
recorded in the existing [nested-token investigation](issue-89-nested-tokens-followup.md),
not an exact publisher example of every supported composition.

Even a basic one-token test is represented by a whole-reveal node and its draw
child (`tokens.rs:1021–1027`). Both are enumerated as declaration positions
(`tokens.rs:749–755`), so a category can expose both whole-reveal and draw-1 uses.
Their immediate physical draw count may match. A presentation change must not
silently delete an offered scope on the assumption that the two remain equivalent
under later nested choices; such deduplication needs separate semantic evidence.

### Why the panel contains blank space

For token decisions, `src/gathering_tabletop/ui.rs:506–518` replaces the ordinary
decision-panel node with a fixed height of the smaller of 40% of window height or
330 logical pixels. The options body grows to consume available height
(`ui.rs:68–83`), and reveal/confirmation/undo controls occupy a separate footer
(`ui.rs:84–104`). With one collapsed ability category, unused body height remains
between that category and the footer. These source settings explain the supplied
layout; they are not evidence of a separately measured native reproduction.

### Accepted direction

**Accepted Q3:** Joel answered "as you recommend": retain the compact ability
chooser with explicit explanations, selected over showing all legal uses at once.
This completes the design frontier; exact wording, dimensions and local code
organization remain delegated implementation judgment.

Make this panel size to its actual content, retaining a viewport cap and scrolling
for longer option lists while keeping required controls reachable. Label the
category as an ability, for example `Jacqueline's ability…`, and identify the
testing investigator explicitly. When another investigator is testing, a concise
explanation can say that Jacqueline can assist because they share a location.
Expanded choices should name what is affected and what the ability does, retaining
costs, Arbiter alternatives, order and every meaningful reveal scope.

A sole fully specified legal use can be offered directly, but the current
whole-reveal/draw-child representation means that “one token planned” does not
establish “one legal use.” Collapsing equivalent choices must not be inferred from
matching labels or immediate token counts. The least risky first correction keeps
the existing offered choices and improves their explanation and layout.

This refines the accepted [guided physical token groups](../adr/0002-persistent-table-areas.md#guided-physical-token-groups)
decision. It preserves the explicit decision to invoke an optional ability and
the requirement to declare before seeing faces. Native readability, scrolling,
pointer and keyboard interaction, and the exact reported Tony state remain
unverified by this investigation.

## Act story modal

The screenshot `/home/joeloverbeck/Pictures/Screenshots/arkham-act-modal-issues.png`
shows short Act 1 reverse content above a large unused panel area and left-aligned
Continue. Its SHA-256 is
`4bd8ec3422afa93c1aab7f7d7c9528c31f1ed3bf6881c40354680a5f7854551d`.

The shared story renderer fixes the panel to 90% of viewport height
(`src/tabletop_ui.rs:425`), and its growing body fills that space (`:441`).
Gathering places Continue in an ordinary left-aligned controls row
(`src/gathering_tabletop/ui.rs:632`). Test Results now uses its own private
content-sized panel with a maximum height, scrolling body, and non-shrinking
footer (`src/gathering_tabletop/results.rs:386–451`); its Continue row is centered
(`:213`). The earlier results correction deliberately preserved other story
consumers, as recorded in the [preceding investigation](gathering-results-tokens-and-slots.md#result-dialog).

**Accepted Q2:** Joel answered "Yes, apply to Act and Agenda": both Gathering
reading panels fit their content, with centered Continue and body scrolling only
when necessary. Keep the heading and Continue visible, retain readable artwork
and text, and share the relevant layout behavior with Test Results.

The smallest credible refactor shares a private Gathering reading-panel shell,
keeping each caller's content, width and Continue action. Results currently cap
width at 760 pixels; stories at 1200, with artwork and text side by side when
space allows and stacked otherwise. Changing the shared `tabletop_ui::story_reading`
directly would also affect Party (`src/party_tabletop/ui.rs:338`). Helper placement
and exact dimensions remain implementation judgment. Layout extraction must not
change story disclosure, saved decisions, or cause Continue to repeat effects
(`src/gathering_tabletop/story.rs:65`; `src/gathering_tabletop.rs:626`).

## Decision and verification boundary

This is bounded research and design for the three supplied Gathering observations,
returning to the owner interview. It does not activate unrelated caption repairs,
broader token-rule work, or a general modal framework. The accepted presentation
refinements are recorded in [ADR 0002](../adr/0002-persistent-table-areas.md#gathering-reveal-cue-and-story-panel-refinements).
The existing glossary's testing investigator and reveal group distinguish the
relevant concepts; no new domain term is needed.

Future verification must exercise the actual UI geometry with ordinary and sealed
reveals, expanded ability choices, long content, narrow/scaled viewports and absent
artwork. It must preserve offered choices and check modal/focus boundaries,
reduced motion, and the reminder's redraw lifecycle. Native inspection is needed
to judge the loop's pacing and text/art readability. Source arithmetic and these
screenshots do not establish that the proposed experience is satisfactory.

Consulted for this investigation and design record: `AUTH-2`–`AUTH-3`,
`PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`,
`REP-1`–`REP-3`, `AGENT-5`, `OPS-1`–`OPS-7` and `GOV-1`–`GOV-3`.

## Local implementation activation

Joel instructed, "when you finish the design, I authorize implementation."
The design is complete. Implementation starts on 2026-11-06 at 15:59 UTC from
`71c4c3e52752ca892f91c589579ec88d28a65ee1`, on
`fix/gathering-reveal-story-controls`, carrying the two local design documents.
The default two-active-hour return and scope/non-convergence limits apply.
The selected increment implements Q1–Q3 and returns one tested local candidate;
publication and native owner acceptance remain separate.

The inherited presentation baseline is the source fixed point above and the
three supplied captures. Affected production paths are the shared reveal control,
Gathering's reveal adapter and decision panel, and its result/story panels.
Preserve the established palette, font sizes, modal input boundaries, offered
choices and continuity; the accepted changes affect layout, explanatory wording
and the decorative cue. The existing production UI layout/picking, offered-choice
and save/reopen seams own verification; no new test interface is required.
These checks establish geometry and behavior, not pleasant pacing or native
readability. `PURP`, `WORK`, `AGENT-5` and `OPS` were reread for activation.

## Implemented candidate and remaining native review

The local candidate uses a 118-pixel centered reveal/resolve button and the
existing 16-pixel gap. Its separate decorative bag moves vertically by at most
three logical pixels over a 2.4-second cycle. The cue requests redraws only while
available, focused, animated and outside its chooser. Reduced motion and missing
art leave a usable stationary text button. Legacy reveal consumers keep their
existing captions, embedded artwork and entrance treatment.

The token panel now fits short content and caps longer content with a scrolling
body. `Jacqueline's ability...` opens an explanation naming the testing/revealing
investigator and the affected reveal choices. Opening the category also marks
the panel dirty: the preceding handler changed the category state without
requesting a redraw. Offered choices, costs and Arbiter alternatives remain
unchanged. The declaration instruction describes the reveal being changed
instead of an unexplained "target."

Gathering's private `reading` module now supplies the content-sized shell for
Test Results and both Act and Agenda readings. Story artwork retains its
landscape orientation, larger width and responsive text arrangement. Continue
stays centered outside the scrollable body. Party's story shell is unchanged.

Regression coverage exercises production UI geometry, the sealed-token
explanation, the expanded Jacqueline chooser and its original committed choice,
redraw cadence, focus/reduced-motion/chooser/no-art stops, story continuity,
artwork, and 960 × 540 at 125% scale. The original overlap and oversized-panel
failures were reproduced before their corrections. Native readability, physical
input and whether the cue feels pleasant remain separate acceptance checks.

Four ordinary saves were generated and loaded through production startup by
`gathering_tabletop::tests::review_saves::export_reveal_and_story_review_saves`:

| Save under `/tmp/gathering-controls-native-saves/` | Revision | Expected opening |
| --- | --- | --- |
| `ordinary-reveal.json` | 9 | Tony testing; Reveal tokens; Jacqueline's ability |
| `sealed-reveal.json` | 9 | Resolve tokens; known Premonition face explained |
| `act-reading.json` | 61 | Act 1 reverse and Continue |
| `agenda-reading.json` | 74 | Agenda 1 reverse and Continue |

Preparation passed in `/tmp/gathering-controls-checks/prepare-native-2/run.json`.
The normal launch configuration resolves artwork to
`/home/joeloverbeck/.local/share/arkham-unfolded/gathering-assets`; all 143 roles
verified in `launch-assets/run.json`, and the real image/font decode test passed
in `decoded-assets/run.json`, both under `/tmp/gathering-controls-checks/`.
These records do not establish rendering through a native window.

The proposed remaining native route is ten minutes including launch, capture
and cleanup: inspect the ordinary and sealed reveals at the supplied framing;
open Jacqueline's ability and confirm the tester/scope explanation; inspect the
loop and reduced motion; inspect Act and Agenda with artwork and substitutes,
including a smaller window, and Continue once. Stop on a failed interaction
after at most one bounded diagnosis, or at the deadline. Source checks,
preparation and implementation permission do not grant a desktop-control
interval; that interval is still pending under the
[native verification procedure](../experiments/jacqueline-curse.md#launch-capture-and-interact).

The owning work remains [#89's current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060).
This local implementation neither closes full-scenario acceptance nor includes
publication or the separately deferred card/token-caption repairs.

## Native verification, 2026-11-06

Joel authorized a ten-minute desktop interval. It ran from 16:42:49 UTC with a
16:52:49 deadline; all diagnostic windows and grants were closed by 16:52:01.
The inspected implementation was `986b8c832c8fd4f91611a444b4592ea78298ffc5`, using
executable SHA256 `4aa8fd02411b8662cba810b62a54abf130573070ea6de32ef7dc0e3be67235db`.
Manifests, application logs, independent client captures and portal input events
are retained under `/tmp/gathering-controls-native-review/`.

- **Reveal and assistance:** the ordinary and Premonition screens retained the
  button/panel gap and centered short captions. The sealed skull explanation was
  visible in the adjacent panel. Clicking Jacqueline's category immediately
  showed Tony as testing, the same-location explanation and both reveal scopes.
  Selecting the whole reveal produced a three-token plan; the diagnostic save
  advanced to revision 10 and still named Tony as revealer. Captures are in
  `input-ordinary/` and `sealed/sealed.png`.
- **Cue and motion setting:** the menu hid the reveal control. Switching to
  reduced motion and closing the menu restored the stationary bag and button.
  The two independent `motion/frame-*.png` captures show different bag positions
  while the button stays in place. This supports native motion presence, not a
  continuous pacing measurement or the owner's judgement of pleasantness.
- **Story reading:** Act 1 and Agenda 1 were compact with readable landscape
  artwork, rules text and centered Continue at 3200 × 2000 client pixels. Act was
  also inspected at 1920 × 1080 and 1280 × 1000. At the latter size the body
  stacked and scrolled by actual pointer wheel input while the heading/footer
  stayed visible. The first upward wheel at the top had no effect; the downward
  wheel scrolled correctly. Continue dismissed both Act and Agenda without
  changing their saved-session bytes. `input-act/` and `input-agenda/` retain
  before/after captures and input acknowledgments.
- **Absent artwork:** `act-no-art/story.png` and `agenda-no-art/story.png` show
  compact readable substitutes and centered Continue. These static runs, and
  the sealed-token capture run, exited 0. The four interactive/motion launchers
  were deliberately interrupted for cleanup (supervisor exit 130; child
  returncode -15); all three portal helpers acknowledged `session-closed` and
  exited 0. These deliberate interruptions are not successful app-exit tests.

The personal dated save and all four source checkpoints retained their hashes.
No implementation correction was needed for the exercised controls. Physical
keyboard interaction, a dedicated native focus-loss/resume check, the native
mixed sealed-plus-fresh draw case, and the owner's pacing preference were not
tested in this interval. Headless coverage of those applicable behaviors keeps
its narrower meaning. At the deliberately very narrow 1280 × 1000 client size,
the underlying Scenario header/counter labels overlap after the Act closes;
that separate observation has no matched baseline reproduction and is not a
confirmed regression from this patch. No adjacent caption repair was activated.
Full Gathering play and owner acceptance remain open on #89; no publication was
performed.
