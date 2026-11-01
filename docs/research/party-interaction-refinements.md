# Party interaction refinements

Research dated 2026-11-01 for Joel's `grill-with-docs` interview, against fixed point `d4fd8ca39f3aa4ebbc5329aa5c18775b273c621a`. The related Party experiment work item is [#55](https://github.com/joeloverbeck/arkham-unfolded/issues/55), which is closed. Joel subsequently accepted both recommendations in the first interview round. [ADR 0002](../adr/0002-persistent-table-areas.md#vertical-action-rows-and-commitment-eligibility) records the complete accepted design. This note retains the research and original proposals as decision evidence; it does not reopen #55, activate implementation or establish native usability.

The owner reports four problems or opportunities: overlapping radial action labels, possible shroud-symbol substitution, moving the scenario reference into the Test area, and clearer legal-commitment feedback with per-card contribution previews. The menu evidence below was checked against primary sources. The report of overlap is owner evidence, not an independently reproduced native observation. No native application was launched for this research.

The root research pass consulted `AUTH-1`–`AUTH-3`, `PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `BOUND-1`–`BOUND-5`, `INT-1`–`INT-4`, `AGENT-1`–`AGENT-5`, `OPS-1`–`OPS-7` and `GOV-1`–`GOV-3`. These identify the governing proposal and evidence boundaries; they do not authorize implementation or publication.

## Existing decision and new pressure

[ADR 0002](../adr/0002-persistent-table-areas.md) selected labeled radial controls for short action sets **with a compact nearby list for detailed wording or constrained space**. The [original research](tabletop-contextual-controls.md) already treated the local list as the least-complex credible comparison, and did not claim that a radial arrangement was generally superior. The new report therefore exposes a case the existing fallback was intended to handle, while raising a separate design choice: whether consistent vertical rows should replace the hybrid convention altogether.

The reported example is Study with Patrice active: `Patrice: Investigate Study (one action; [INTELLECT] against shroud 2)` overlaps `Inspect`. The explanation is decision-relevant, so removing it or hiding it until hover would not preserve the requested information. Merely reducing container width also cannot solve the problem unless row height and wrapping adapt. Static inspection of `src/party_tabletop/controls.rs:448–506` finds that Party selects radial layout by entry count, source type and viewport size, without checking label length. Its three absolute slots have fixed 145-pixel widths and fixed offsets. This supports the reported failure mechanism; no native reproduction is claimed.

The existing inspection modal is a separately accepted component. A proposal to change routine action selection does not redesign full card inspection. The ADR also delegates exact dimensions, spacing and responsive breakpoints to implementation; these are not additional owner policy choices.

## Menu recommendation

Use a compact, card-adjacent vertical action popover: one full-width selectable row per option, with a clear action title and visible supporting details. Allow a row to contain several text lines. Keep the source card visibly identified and the panel within the usable viewport. Prefer readable wrapping and a sensible maximum width over an arbitrarily narrow panel.

This is an application-specific inference from the owner-reported failure, the existing decision, and the evidence below. It is not a measured claim that a vertical list is faster for Arkham Unfolded. A stable universal row layout also avoids asking the player to reinterpret the same interaction as radial or linear according to label length.

Proposed text structure, with symbol placeholders pending verification:

```text
Study · Patrice

┌──────────────────────────────────────┐
│ Investigate Study                    │
│ 1 action · [INTELLECT] vs shroud 2    │
├──────────────────────────────────────┤
│ Inspect                              │
└──────────────────────────────────────┘
```

The first item is one clickable row with two lines, not two separate controls. The header may carry actor identity only when every action in the panel has that actor; otherwise name the actor within the affected row. Costs, warnings and other decision-relevant conditions remain visible before activation. The displayed shroud word is provisional until the asset investigation establishes a supported semantic symbol.

## Primary evidence and limits

**Original pie-versus-linear experiment.** Callahan, Hopkins, Weiser and Shneiderman studied 33 undergraduates with little or no mouse experience, using menus of exactly eight items. Pie menus reduced target-selection time; the error difference was only marginal (`p = .087`) and preferences were divided. Participants selected a named target rather than evaluating consequences. The authors warn against broad adoption claims, identify screen coverage as a problem, and explicitly limit the study to the tested menu size. This does not establish superiority for two changing actions with explanatory labels. [CHI 1988 paper, pp. 97–100](https://www.cs.umd.edu/~ben/papers/Callahan1988empirical.pdf)

**Long labels are a known radial-layout difficulty.** Original implementer Don Hopkins explains that long labels enlarge pie menus and that layout must prevent overlap while clearly associating labels with directions. Short labels and icons are easier to fit. This is first-hand design analysis, not a new comparative experiment. It supports taking the reported overlap seriously without concluding that every radial interface is unsuitable. [The Design and Implementation of Pie Menus, disadvantages and design considerations](https://www.donhopkins.com/home/catalog/piemenus/ddj/piemenus.html)

**Preserving visual context matters.** Tapia and Kurtenbach describe how horizontal text in vertical wedges produced oversized menus, obscured the underlying task, and complicated placement near screen edges. Their refinements retained radial interaction while reducing graphics and permitting labels outside wedges. The paper reports iterative design informed by about 15 users and explicitly says those refinements were not formally tested. It provides a credible repair alternative, not evidence that another radial revision will solve the present task more economically than a list. [UIST 1995 paper, pp. 190–194](https://www.cs.columbia.edu/graphics/courses/csw4170/resources/p189-tapia.pdf)

**Small attached overlays are a desktop convention.** GNOME describes popovers as overlays attached to their invoking controls, permits varied content, and recommends low complexity, small size, and Escape dismissal. Its approximate one-third-window size guidance is a ceiling heuristic rather than a target. This supports the proposed container but supplies no tested pixel width for this game. [GNOME Human Interface Guidelines: Popovers](https://developer.gnome.org/hig/patterns/containers/popovers.html)

**Routine actions need not become central dialogs.** GNOME describes action dialogs as obscuring the parent window and requiring a context switch, and says inline controls or actions are often preferable. That supports preserving local context during ordinary card action selection. It does not rule out an existing inspection modal or a genuine multistep decision dialog. [GNOME Human Interface Guidelines: Dialogs](https://developer.gnome.org/hig/patterns/feedback/dialogs.html)

**Keyboard operation has an established interaction model.** W3C's menu pattern documents arrow-key traversal, explicit activation, Escape dismissal, and restoring focus to the invoking context. It is web guidance modeled on native menus, not a Bevy API prescription or proof of native accessibility. The project's ADR already records Tab/Shift+Tab navigation with Enter/Space and Escape. Preserve those accepted bindings unless a separate change is selected; the external reference is useful for focus management, not grounds for silently replacing existing controls. [W3C APG: Menu and Menubar Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/), [existing keyboard decision in ADR 0002](../adr/0002-persistent-table-areas.md)

## Alternatives compared

| Alternative | Benefit | Cost or unresolved pressure | Recommendation |
| --- | --- | --- | --- |
| Repair the existing radial/list hybrid | Keeps the existing convention; list fallback directly addresses long labels. | Two layouts remain; radial spacing and fallback thresholds still need maintenance and native evaluation. | Credible smaller change if retaining radial interaction is valuable to the owner. |
| Use vertical rows for all contextual actions | Accommodates variable explanatory text, predictable scanning and full-width targets through one layout. | Longer lists may require bounded scrolling; no demonstrated native-use advantage yet. | Recommended for this reported workload. |
| Add a persistent side action inspector | More room for complex explanations. | Permanent space cost and travel away from the selected card; introduces a different interaction surface. | No present need established. |
| Use a central modal for each card action list | Large predictable presentation area. | Occludes the tabletop and interrupts routine interaction. | Reserve modal treatment for already accepted inspection or a decision that actually requires it. |

The recommended list preserves the useful part of the contextual-control decision: choose an exact component, inspect its current legal options nearby, then deliberately activate one. No new menu framework, gesture system, or rules representation is needed to evaluate this direction.

## Proposed behavior to preserve

- Opening a menu and moving over its rows do not execute a game choice. The opening click cannot also activate the row under the pointer.
- Use the exact current legal choices and preserve distinct copies, compound selections, completion controls, costs and warnings. Layout cannot invent, merge or discard choices.
- Keep Inspect as its own row when there are gameplay choices. Preserve the accepted direct-inspection behavior when the only available operation is a single Inspect target.
- Retain existing keyboard navigation, visible focus, camera-key capture, revision invalidation and replay inspection-only behavior.
- Keep the source card visible where practical. Place the panel in screen space and reposition the whole panel within the viewport without moving the pointer.
- Escape and an outside dismissal click preserve game state. A click consumed to dismiss must not also execute an underlying tabletop action.
- Preserve pending commitments and other staged game choices when dismissing controls or opening inspection. Source selection, hover, keyboard focus and pending game selection retain their separate meanings.

These are proposed constraints and existing invariants to verify, not a claim that the present implementation already satisfies every item.

## Acceptance cases and stopping boundary

Joel selected universal vertical rows in the first interview round. Exact geometry remains an implementation choice. Any later implementation needs its own bounded handoff and acceptance; this research does not supply it.

The consumer checks for an accepted candidate should include:

1. Patrice's Study menu shows the complete action cost, intellect test and shroud value alongside a separate Inspect row, without overlap or hidden explanatory text.
2. The longest real label and largest real choice set remain readable, reachable and distinguishable; long rows wrap, and bounded scrolling is used only when actual content needs it.
3. Menus opened near each viewport edge remain usable at supported window sizes and UI scale, with no pointer relocation or accidental selection.
4. Mouse and keyboard can reach and activate the same choices; the camera does not also move while the menu captures input, and dismissal restores usable focus.
5. Opening controls does not dispatch a game choice. Dismissal and inspection preserve staged selections; changing game revision or switching replay/live mode invalidates staged selections and prevents stale dispatch.
6. Prepared symbols and readable English substitutes both preserve the information needed to choose.
7. Existing Tony and Jacqueline consumers receive the same accepted convention, alongside Party, through shared production behavior where practical.

Automated layout and interaction checks can establish their exercised geometry and state transitions. Native observation is still needed for readability, source association and pointer comfort; Joel's acceptance determines whether the experience meets his needs. A further research pass is warranted only for a specific unresolved decision, not a general menu benchmark or repeated exploratory redesign.

## Shroud-symbol finding

The selected Party font is `arkhamicons Regular`, 144,020 bytes, SHA-256 `186176d7243c72d272a50c24e833fb77d983e7c4c6c99bc64f89c8322dbd8187`, verified read-only in the current prepared collection. [Current symbol mappings](../../src/tabletop/symbols.rs) include skills, wild, chaos and harm symbols but no shroud. The [font author's configuration](https://raw.githubusercontent.com/zzorba/ArkhamCards/master/assets/arkhamicons-config.json), inspected on 2026-11-01, contains 109 named entries and no shroud entry. Its skill/harm mappings agree with the application's mappings. The current upstream font differs in size from the selected local font, so this is corroboration rather than an exact version match.

No verified shroud glyph has been identified in the selected font. This is a bounded negative finding, not a claim about every Arkham symbol asset. Retain the word unless a genuine glyph and its meaning are verified. The [earlier font examination](attack-opportunity-and-harm-presentation.md) reports unnamed glyph metadata and a full visual sheet inspected for a different symbol; that historical inspection does not independently settle shroud.

If a glyph is later identified, render semantic shroud-stat references with it, retain readable English fallback/accessibility descriptions, and preserve printed artwork and source text. Do not globally replace the character sequence in unrelated names or prose. Generic test difficulty remains generic: shroud is not every test's difficulty. [ADR 0002](../adr/0002-persistent-table-areas.md), under Zoom, test explanation and physical token custody, currently explicitly introduces no shroud glyph dependency.

## Scenario reference in Test

Joel explicitly requests a permanent home in the upper-left of Test across experiments and the eventual game. This desired placement does not need to be re-asked. Interpret upper-left in the normal Test focus orientation, with the appropriate scenario/difficulty reference present between tests as well as during them. Manual orbit can change its apparent screen position.

Party's `card/01104` Standard reference and Tony's reference both use `SCENARIO_REFERENCE` in [shared layout](../../src/tabletop_layout.rs), currently on Scenario. See [Party scene](../../src/party_tabletop/scene.rs) and [Tony scene](../../src/tony_tabletop/scene.rs). Test's upper-left currently holds `test_source()`, used for Jacqueline's Rotting Remains source card. Reserve distinct space for both; do not obscure either. This is ordinary layout judgment, not another owner decision.

Jacqueline's [current card list](../../src/tabletop.rs) and asset recipe do not include this reference card, so coverage of all three experiments requires adding its presentation, inspection and selected asset through the existing preparation contract. Party already offers reference inspection from chaos bag/token controls; retain that access. The future game inherits the placement convention as implemented; this research does not establish its implementation or broader scenario support.

## Legal commitment feedback

The [glossary](../../GLOSSARY.md) already distinguishes the printed identity of wild icons from their effective contribution to the tested skill. Unexpected Courage can show `+2 [INTELLECT]` during an intellect test without changing its printed icons. The [Rules Reference, p. 26, ST.2](https://images-cdn.fantasyflightgames.com/filer_public/c4/b0/c4b0d66c-d79e-411b-bdb5-b5d8c457d4bc/ahc01_rules_reference_web.pdf#page=26) establishes ordinary matching/wild contributions, cards from hand, and one contribution card per same-location supporter. Card-specific restrictions also matter. Icon matching alone is not full commitment legality.

At the fixed point, [Party rules](../../src/party.rs) expose only intellect tests and store effective contributions: On the Hunt 1, Promise of Power 4, Clairvoyance 1, Unexpected Courage 2, Eyes of the Dreamer 1, Guts 0, Lucky! 0. Legal commitment sets reject more than one card per supporting investigator. This limited representation does not establish support for arbitrary conditional or negative icons.

There is a staging inconsistency: [Party controls](../../src/party_tabletop/controls.rs), lines 119–138 and 226–239, allow toggling any card appearing in any legal set, without checking the already staged subset. Two matching cards from one supporter can therefore both be selected, after which confirmation disappears because their combination is illegal. A cue claiming that a card can be added now must account for the staged subset, and its action must agree with the cue.

Propose a steady teal edge treatment plus a readable top-edge contribution badge for an available addition. Selected cards retain the existing gold corners and show explicit selected status with their contribution. Hover and keyboard focus remain separate; color alone does not carry the distinction. Use a static treatment by default, readable without animation or pointer hover. Cards that are not eligible remain inspectable. Keep decision-relevant consequences such as Promise of Power's curse visible in its action row; the numeric badge does not communicate the whole card effect or recommend a tactic.

Propose that selecting Tony's first card while Patrice tests removes the readiness cue and addition control from his second card. Unselect the first before choosing the other; do not silently replace it. This user-facing switching behavior is part of the proposal, not a settled rule requirement. Selected cards remain removable. Clear current-opportunity cues after the commitment window and in replay, and recompute on relevant state changes. Inspection and menu dismissal preserve staged choices as already required.

Reuse the [shared feedback implementation](../../src/tabletop_feedback.rs) where practical, with each existing consumer providing its own legal additions and effective bonuses. The current shared feedback distinguishes hover, menu source and pending selection; it has no eligibility state. Future content must supply its actual rules-derived contribution and restrictions rather than borrowing Party's intellect-only assumption.

## First interview round

The two independent design choices below were presented together and both accepted by Joel with "as you recommend." The reference-card relocation was already explicit user intent; glyph existence is an investigated fact, not a preference question. These are the historical questions at the fixed point above; no design question remains pending. GitHub remains the active work-control plane and no new implementation item is activated here.

1. Replace radial card-action controls across the existing experiments with the compact adjacent list shown above, using one clickable row per action, wrapped supporting details, and Inspect last? Recommendation: yes. Retaining the hybrid with a repaired fallback is the strongest smaller alternative, but keeps two layouts after the owner has reported discomfort with the radial form.
2. Use the proposed steady eligibility edge and contribution badge, distinct selected treatment, and explicit unselect-before-switch behavior when a supporter has reached its one-card limit? Recommendation: yes. Automatic replacement could save a click, but would alter an existing selection implicitly; retaining icon-only eligibility would advertise currently illegal additions.

The answers are recorded in the existing ADR, and the glossary now distinguishes committable cards from the pending commitment. No gameplay code, prepared assets, saves or GitHub state changed during this research pass. Joel subsequently explicitly activated implementation; [#77](https://github.com/joeloverbeck/arkham-unfolded/issues/77) owns that work. The research remains design evidence, not native proof.
