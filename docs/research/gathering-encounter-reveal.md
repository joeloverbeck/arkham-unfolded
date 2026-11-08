# Gathering encounter reveal and doom readout: design investigation

Investigated 2026-11-08 against candidate `fda03a5368775b2234b6fa5a4ec0c2b11012b86d`, following Joel's playtest feedback on [#137](https://github.com/joeloverbeck/arkham-unfolded/issues/137). This note proposes an encounter reading surface; it does not activate or claim implementation. Joel reported everything else passed apart from Patrice's final forced discard-order prompt and the doom labels. Those findings and their disposition remain in the issue's [current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/137#issuecomment-6722015709).

## Recommendation

**Owner refinement after playing candidate `946c1d2`, 2026-11-08:** Joel reported
that the encounter modal and moving tentacles look great, but asked to remove the
overlapping Pause motion button and the extra Resolve click after Continue. The
active presentation contract therefore removes local Pause/Resume controls and
shows actual legal pre-revelation responses on the reading screen. Continue
commits the existing resolution choice when that encounter remains unresolved;
already committed consequences still need only acknowledgment. Tests and other
meaningful decisions retain their existing surfaces. The Rotting Remains input
has Ward of Protection available, so moving those responses alongside Continue
preserves the choice while removing the redundant handoff. The original proposal
below remains historical where it describes separate acknowledgment or a local
pause button. See the [dated feedback and correction](https://github.com/joeloverbeck/arkham-unfolded/issues/137#issuecomment-6724663314).

Joel accepted these recommendations with “Proceed as you recommend” on
2026-11-08. The implementation follows the existing committed presentation
snapshots. A later production-path check clarified an implementation detail below:
`Game::settle` can resolve an encounter with no cancellation opportunity within
the draw command itself. The reading stop therefore holds its captured draw view
before displaying those committed consequences; it does not split the command,
change journals, or make Continue a new rules choice. Unresolved reactions still
wait normally. Source/test evidence and native experiential acceptance remain
separate, as recorded in #137's current handoff.

Give each actual encounter-deck draw an untimed, screen-centered reading stop: a large intact card, **Encounter card**, the drawing investigator's name, the card's title/type, and **Continue**. Darken the table behind it. Continue acknowledges reading and returns to the existing legal response/resolution path; it must not silently decline a cancellation or generically engage an enemy.

Use a restrained moving surround: a few dark teal, tapered tendrils slowly bending at the screen edges behind the card. Keep the card and all readable controls still. My recommendation favors this over a rotating vortex, large fleshy tentacles or a pulsing red wash: those effects compete with reading and can make every card feel like the same monster. This is creative judgment, not a researched guarantee of tension. A static ink-and-brass surround is the simpler credible alternative and the Reduced-motion presentation.

## What the rules establish

**Encounter card** is the correct umbrella label. Encounter cards include enemies, treacheries and story assets; **Treachery** is a card type. Mythos draws proceed in player order. Draw, applicable draw-time restrictions/responses, Revelation, and card-type placement remain ordered. Revelation precedes entry/discard. A no-Spawn enemy ordinarily engages its drawer; Spawn instructions, illegal destinations and Aloof can change that result. Surge follows the current encounter's resolution. A presentation pause creates no new rules window. [Original Rules Reference, pp. 3–4, 6, 10, 18–20, 24](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf).

Ward of Protection (0) acts when its controller draws a non-weakness treachery. On the Hunt instead replaces a would-draw occurrence before the encounter's identity is known; its searched candidates are not nine draws. The original printed cards, including On the Hunt's no-match fallback, were inspected. The publisher's [The Oncoming Storm](https://www.fantasyflightgames.com/en/news/2018/2/22/the-oncoming-storm/) corroborates these distinctions; its upgraded Ward discussion must not replace the selected level-0 card.

The temporary visual surface is not a new game zone. The Legacy FAQ's §1.23 distinguishes a resolving treachery's limbo from being in play, in hand or discarded. Keep that custody intact. [Legacy FAQ 2.5](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc_faq_v25_february_2026-web-1.pdf).

## Proposed experience

1. Complete the existing committed card draw/reveal movement, then place a readable inspection-sized image at the center. In Reduced motion, place it immediately. Do not reveal an unknown card before a replacement such as On the Hunt is decided.
2. Show **Encounter card** above the image and **Drawn by Jacqueline Fine** or the actual drawer beneath that heading. Put **Rotting Remains · Treachery** beneath the image. Use the actual card type rather than calling every encounter a treachery. The image retains its original art and text.
3. The card uses the available viewport height without cropping. Header and footer remain visible; small viewports retain scrolling or the existing readable effective-text inspection. Do not make zooming a prerequisite for reading the basic encounter.
4. Keep one prominent **Continue** at the bottom. A quiet explanation can say **Read the card, then continue to any available responses.** No countdown or automatic dismissal. Retain keyboard focus, accessible card/drawer labels, and protection against the click/key that opened the surface immediately dismissing it.
5. Continue removes only this reading stop. If Ward of Protection, A Test of Will or another real opportunity is available, the existing controls appear next. If no real alternative remains, compulsory resolution can continue automatically. Tests still use Test; searches and non-test choices still use the originating investigator's tray. Actual card destinations follow the rules, not the overlay.

For the first implementation, include actual encounter-deck draws during Mythos and their supported extra-draw continuations. Do not label cards inspected during searches as draws or show a succession of nine modal cards. A selected On the Hunt enemy is already a deliberate search selection; adding a second dramatic reading stop there is unnecessary for this first scope. Weakness draws from investigator decks are a separate presentation extension, not silently included under the new encounter-deck label.

### Motion treatment

Start with three or four peripheral silhouettes, low contrast, with independent slow bend phases of roughly 8–12 seconds. Keep a clear exclusion region around the card, heading, title and controls; nothing crosses the printed rules. No camera movement, card rocking, flashes, rapid rotation or audio is selected. These starting values need native tuning.

Honor the existing Reduced-motion setting with a still surround. Add a quiet **Pause motion** / **Resume motion** control inside the overlay, available without closing the card or committing anything. Pausing affects only the decoration; it must not change game timing. Pause decorative updates when unfocused and resume without a time jump or queued burst. W3C explains why continuing movement can obstruct reading and recommends a stop mechanism for prolonged automatic movement, plus disabling nonessential interaction animation. These are useful design constraints for this native app, not a claim of formal web conformance. [Pause, Stop, Hide](https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide.html), [Animation from Interactions](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html).

## Existing implementation seam and risks

The inspected source already separates draw from resolution. [`encounter_search::draw_encounter`](../../src/gathering/encounter_search.rs) creates a `PendingRevelation` with the physical card, `drawn_by`, and `started: false`, and captures a `DrawSource::Encounter` cue. [`RevelationView`](../../src/gathering/runtime.rs) exposes that state. [`controls::automatic_choice`](../../src/gathering_tabletop/controls.rs) can subsequently select `revelation:resolve` when no alternative remains. The new stop belongs after the draw's visual arrival and before that dispatch.

Use a local encounter-reading owner alongside the existing story/result reading state in [`Presentation`](../../src/gathering_tabletop/presentation.rs), reusing [`reading::panel`](../../src/gathering_tabletop/reading.rs), the existing inspection image/effective text, and [`Table`](../../src/gathering_tabletop.rs)'s modal/automatic-dispatch blocking. Continue should use the existing epoch/stale-input discipline. Card/tray/Test identity stays authoritative; the overlay is an enlarged reading view, not a second playable copy.

Key the reading occurrence by committed draw occurrence/revision and physical card identity, not only printed code or card identity. Capture that identity at the draw; subsequent response revisions must not create another reading occurrence. The same card can legitimately be shuffled and drawn again. Scene rebuilding, camera movement, a nested response and repeated input must not reopen an already acknowledged occurrence in the same session. A fresh process can reconstruct an unresolved, unstarted encounter from the saved pending revelation and show it again for reading; that is harmless re-reading, not a second draw. Exact persistence of a cosmetic acknowledgment is not required by this proposal. Verify that opening a save after resolution has begun does not move the card back or invent a fresh draw. Do not infer an occurrence by parsing history prose.

`Cargo.toml` pins **Bevy 0.20.0**. The installed matching primary source exposes UI translation, scale and rotation through `UiTransform`; the current [`tabletop_ui::card_image`](../../src/tabletop_ui.rs) already uses it. A bounded first visual probe can form tapered silhouettes from a small fixed set of overlapping rounded UI segments, moving their transforms in Rust behind the card. This needs no new dependency, custom shader or general animation framework. Whether those contours look organic enough remains untested; reject visible joints or a cartoon-worm appearance. Keep the static surround usable if the animation treatment fails. API/source feasibility does not establish rendering performance or visual quality.

[ADR 0002](../adr/0002-persistent-table-areas.md) selected investigator trays over interactive effect-resolution modals. The proposal is a narrow new **reading** exception; gameplay choices remain in the accepted surfaces. The [presentation contract](../tabletop-presentation.md#investigator-resolution-trays-and-premonition) would need an explicit corresponding update if the design is accepted. No accepted ADR or contract was changed during this investigation.

## Representative checks for the proposed change

| Case | Distinguishing expected result |
| --- | --- |
| Rotting Remains, Jacqueline can play Ward (0) | Reading stop names Jacqueline. Continue preserves Ward; declining/unsuccessful cancellation leads to her willpower-3 test. Reading cannot start the test early. |
| Ancient Evils | Reading occurs before its doom consequence. After legal responses, one doom is added and its explicit advancement check runs. A reopen or double Continue never adds another doom. |
| Ghoul Minion | The card is labelled Enemy. After the reading stop it follows ordinary spawning/engagement. The overlay does not itself move the investigator. |
| Icy Ghoul or Flesh-Eater | Existing Cellar/Attic Spawn instructions govern, including an absent destination; do not force these into the drawer's threat area. |
| On the Hunt and no-match fallback | Preserve the pre-draw replacement choice and search presentation. Only the fallback's actual draw gets the ordinary encounter reveal. |
| Same printed title twice; later re-draw of one physical card | One reading stop per real draw occurrence, neither collapsed by title nor repeated on a scene rebuild. |
| Reduced motion, pause/resume, small viewport, stale input | Still and animated modes expose identical game choices; the card remains readable and Continue usable; no click-through, duplicated effect or resumed animation burst. |

The first acceptance-bearing slice should combine one cancellable treachery, one immediate consequence and one enemy through the production UI and save/reopen path. A frame or shader demonstration alone cannot establish the requested feeling. Judge whether the moment gives Joel time to recognize and dread the specific card, without making ordinary repeated draws tiresome. This investigation ran no native demonstration or new tests and consumes no new owner playtest allowance.

## Related proposal: center the doom readout on the mat

Joel's reported rightward placement is present in [`doom::render`](../../src/gathering_tabletop/doom.rs): the two-line caption is anchored 0.8 units right of the centered altar. The source records that offset as protecting the populated encounter-discard caption's projected space. Merely removing it must not reintroduce that overlap.

I recommend two centered lines that look printed on the mat immediately in front of the altar, with a fixed orientation matching the Scenario area. Give **Total doom: 1/3** the stronger hierarchy in warm ivory and **Agenda doom: 1** a smaller muted-brass treatment. Use the existing readable font, generous spacing and restrained brass rules; avoid an ornate face, flashing danger color or a new font dependency. Keep the altar/token centered and preserve their current geometry and count ownership. Retain a selectable inspection region for the total breakdown without suggesting a gameplay action.

This is a visual proposal, not an implemented correction. Mat lettering trades camera-facing readability for a stable physical association. It must remain legible in normal Scenario focus and a representative oblique view; inspection can supply detail but cannot excuse illegible ordinary counts. Compare against centered camera-facing text as the simpler alternative. Verify the rendered text footprint against populated piles, upright Agenda/Act cards and map controls, without shrinking existing cards or widening the camera just to pass. The user's preference motivates the recommendation; native visual fitness remains unproven. The existing owner evaluation is already used, so this note does not schedule another one.

Following implementation, Joel's 2026-11-08 screenshot showed the Agenda doom line overlapping the Map caption. He selected removal of that redundant line because the altar's counted token already supplies its value. This supersedes the two-line proposal above; the [presentation contract](../tabletop-presentation.md#scenario-locations-and-enemy-sideboard) retains the total-doom threshold and its selectable breakdown at the existing position.

## Source custody and evidence limits

- The governing RR and Legacy FAQ were read from the retained copies under `local/verification/ebb3766/round-flow-research/`; their verified SHA-256 values remain `14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a` and `35c82dc070332acd7863eb6d464641ea79e4657b68ff4f1873538e0bcac6b2e7`. The browser could not open the current RR CDN URL; the publisher's [original AHC01 RR](https://images-cdn.fantasyflightgames.com/filer_public/c4/b0/c4b0d66c-d79e-411b-bdb5-b5d8c457d4bc/ahc01_rules_reference_web.pdf) was also accessible as corroboration, not a replacement baseline.
- Original prepared faces for `01065`, `03263`, `01163`, `01166` and `01160` were visually inspected; [the preparation recipe](../../resources/gathering-asset-selection.json) binds their source variants/hashes. No images were edited, copied into Git or sent to an image-generation service. The selected Icy Ghoul/Flesh-Eater identities and Spawn instructions are already reconciled in [scenario/Tony source research](issue-89-rules-scenario-tony.md).
- Bevy 0.20.0 `bevy_ui/src/ui_transform.rs` was inspected in the installed registry (file SHA-256 `50b2b4fd41a5ea777bfa0f080ac86fc723fcbb1d543f4f4cf6ffd2ec87e0825c`); the versioned docs.rs URL was unavailable to the browser. Existing application source provides the additional consumer witness. No external API assumption requires adopting a new library.
- Rules/source facts, the existing source seam, this proposed design, and experienced/native acceptance are distinct. The main remaining uncertainty is whether the peripheral motion improves tension while preserving comfortable repeated reading. The simpler static version remains a valid comparator.
