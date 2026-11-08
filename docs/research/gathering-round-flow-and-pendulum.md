# Gathering round flow and Crystal Pendulum investigation

Investigated on 2026-11-08 at `ebb37663c3d98357e4ae3cb0d043d20ba97de7e3`
for the owner's `grill-with-docs` interview. [Issue #89](https://github.com/joeloverbeck/arkham-unfolded/issues/89)
and its [current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060)
own work, decisions and acceptance. This note records investigation and the design
proposals considered for a separate follow-up spec; it does not activate implementation.
The [maintained presentation contract](../tabletop-presentation.md) records the
subsequently accepted behavior. This note's recommendations and geometric
estimates are research evidence, not an additional source of current work status.
No native application session was controlled or observed by the agent.

## What the current save establishes

Read-only inspection of the remembered ordinary attempt found round 1,
Investigation, no active investigator, Tony and Jacqueline finished, and Patrice
with three ordinary actions remaining. The available player choice was to begin
Patrice's turn. The owner subsequently relaunched and confirmed the breathing
highlight on Patrice's card. This is not evidence of a phase-engine softlock.
The owner's separate report of having seen Enemy pass without witnessing Mythos
remains a usability observation; this save does not reconstruct that earlier run.

An isolated copy was advanced through the production player driver: begin/end
Patrice's turn, empty Enemy, Upkeep, round 2 Mythos, one agenda doom, and encounter
draws. It stopped at Patrice's Grasping Hands skill-test commitment decision.
The agent did not write the original; the owner advanced it during investigation,
so final verification used a hash-verified retained copy of its initial revision.
The diagnostic deliberately ended Patrice's turn early on that copy. This
demonstrates that both phases exist and
are reachable through that path. It does not establish native automatic progress,
all phase interactions, or full-scenario correctness.

At the inspected revision:

- `src/gathering/phases.rs` implements Upkeep and Mythos, including readying,
  draw/refill, resources, hand limits, round end, doom and encounter draws.
- `src/gathering_tabletop/controls.rs:294` routes turn selection to the physical
  investigator card. Portrait navigation only changes camera focus.
- `src/gathering_tabletop/play.rs:363` renders the phase in the Live Decision
  status. There is no animated phase announcement. With no active turn, the
  status can say there is no pending player decision despite an available turn
  selection.
- `src/gathering_tabletop/controls.rs:733` automatically commits selected Enemy
  bookkeeping and empty player windows, but has no Upkeep/Mythos bookkeeping
  entries. Ordinary draws are routed to investigator decks and encounter draws
  to the encounter deck. Required clicks can therefore be mistaken for a stall.
- The agenda has a doom caption but no physical doom counter. Enemy doom asks
  for `token/doom`, absent from the prepared recipe, leaving a plain-color visual.

## Rules sources and independent expectations

The freshly retrieved PDFs matched the hashes of the repository's accepted
source set. They are retained with the diagnostic evidence; publisher-hosted
URLs alone are not durable custody.

| Source | Identity and relevant locations |
| --- | --- |
| [Original Rules Reference](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf) | SHA-256 `14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a`; pp. 23–26 phase/test timing, p. 15 modified values, pp. 5 and 26 automatic failure. |
| [Legacy FAQ 2.5](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc_faq_v25_february_2026-web-1.pdf) | SHA-256 `35c82dc070332acd7863eb6d464641ea79e4657b68ff4f1873538e0bcac6b2e7`; p. 17 Patrice's simultaneous refill. |
| Crystal Pendulum, original card `60411` | Inspected actual face in the external archive; SHA-256 `7814423a51a1b42bbb4b78c4c6cf71d3d7783eda1ae664bf0244bb4240babc20`. |

The round order is Mythos, Investigation, Enemy, Upkeep; the first **round** skips
Mythos. Empty Enemy still has its timing windows. Upkeep has an opening player
window, action reset, simultaneous readying, draws in player order, resource
gain, hand checks, then phase/round end. Mythos places agenda doom, checks all
doom in play, resolves any advancement, then resolves encounter draws in player
order before its final player window. Ordinary extra doom does not independently
check advancement; explicit exceptions such as Ancient Evils do.

Existing source-derived obligations and worked cases remain in
[scenario/Tony research](issue-89-rules-scenario-tony.md),
[Patrice research](issue-89-rules-patrice.md), and
[Jacqueline research](issue-89-rules-jacqueline.md). Preserve their qualified
interpretations and selected original-campaign/deck scope.

Useful discriminating checks for the follow-up:

- Agenda 1 with one doom plus one doom on Tony's Quarry reaches three when
  Mythos places its next doom. Remove both cards' doom and preserve the agenda
  reverse's real choice before encounter draws. Quarry entering while an agenda
  already holds two doom does not itself advance that agenda.
- Patrice discards eligible non-weakness cards, preserves weaknesses and Hidden
  restrictions, draws her refill simultaneously, then resolves revelations in
  the chosen order. Do not refill again after a revelation removes a drawn card.
- Preserve engagement after readying, player/trigger ordering, hand-limit
  selections, encounter cancellation, surge, harm allocation, and scenario
  endings. Do not automatically pass a real optional ability because only one
  investigator remains or one apparently routine button is visible.
- A final modified skill below zero is floored at zero for the margin. Skill 2
  plus a -4 modifier against difficulty 3 fails by 3, as does automatic failure
  at difficulty 3. Automatic failure at difficulty zero and a tied failure under
  The 13th Vision have margin zero.

## Phase presentation proposal

Keep the existing phase engine and make the full supported round intelligible.
Automatically perform compulsory administration, preserving visible component
movement and stopping for genuine decisions. Announce every entered phase,
including an empty Enemy phase and the first Investigation after setup; never
announce Mythos in round 1 or invent a fifth Round End phase. Round-end effects
belong to the departing Upkeep presentation until the next round begins.

A candidate banner uses the existing dark navy/teal and brass palette, large
phase name and smaller round number, with a restrained entrance, stationary
reading interval and exit. Start around two seconds total (roughly 0.25 / 1.5 /
0.25), subject to native pacing judgment. Present it before that phase's visible
consequences and automatic continuation. Preserve a compact current phase/round
indicator afterward. Do not require clicking a banner or accidentally activate
an underlying control when it disappears. Reduced motion uses stationary text.

This is a design recommendation, not a research-proven optimal duration.
Microsoft's [XAG 116](https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/116)
explains the burden of time-limited information, while
[XAG 117](https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/117)
supports readable stationary text and controllable motion. Applying those
principles here suggests persistent phase information alongside the transient
banner and the existing Reduced motion setting; it is not a claim of full XAG
conformance or a reason to build an accessibility-settings subsystem.

Banner timers have no rules authority. Save/reopen, Undo, pause/inspection,
scene rebuild and focus changes must neither repeat effects nor hide a genuine
choice. Preserve the committed game boundary through interrupted presentation.
When waiting between investigator turns, say which investigator(s) remain rather
than reporting no pending player decision. Keep camera focus independent of turn
selection.

## Doom artwork candidates

Paths are relative to the external archive
`/home/joeloverbeck/Pictures/Arkham Horror LCG Resources`. Source bytes remain
unchanged and outside Git. The owner reserved the choice and selected A during
the interview; the table preserves the comparison.

| Candidate | Source and observed distinction |
| --- | --- |
| A, recommended | `images/other/unclassified/en/sce-base-save/Doom Counter--ImageURL--21ffe1f9a4939f78d008.png`; clean 512 × 512 red/black face; SHA-256 `13855822101b0a63ebbafab6f3c6a12500032c286806c70ab3cd8b93488c7301`. |
| B | `images/other/community/en/the_matter_of_britain/Doom Counter--ImageURL--1431219c55502dd2e23f.png`; visibly pixelated 384 × 384 rendition of the same design; SHA-256 `45a5fa12ddf18ae42d79b7614ad4b8edfeaf126a8e1186a8be7d81b81f6c14ee`. |

The `Doom tokens--DiffuseURL--99e21937b84343029a3a.png` atlas repeats the clean
face; it is not a third design. The misleadingly named
`Doom tokens--ImageURL--8cd8d32074a084c7f580.png` depicts a green clue, and the
inspected blue/red generic tokens depict chaos symbols. Neither is a doom option.
The current clue recipe uses a crop of
`Clue Counter--DiffuseURL--61cb6ee4c64be6c1fc9f.png`; candidate A fits that visual
family. The current recipe has no doom role.

The initial proposal used the selected face on counted solid
tokens on the agenda and other cards carrying doom. The owner's subsequent
plinth suggestion is investigated below. Local counts and the total
threshold calculation remain distinct. Addition/removal should be visible,
including clearing doom from all cards when advancing the agenda. Preparation
must produce a complete replacement collection without overwriting source art
or silently weakening required-artwork admission.

### Altar placement follow-up

The owner accepted the banner, Pendulum picker and clean Doom face recommendations,
then proposed a separate raised platform for agenda doom because the agenda is
upright, then accepted the recommended low altar and separate doom readouts.
Accepted presentation requirements belong in the maintained contract; #89 owns
the decision record. The following source/layout analysis informed that direction.
The exact geometric estimates remain provisional and are not inspected native output.

`src/tabletop_layout.rs:74` renders the active Agenda/Act faces as an open book,
75 degrees from horizontal, with their centers at approximately x = -0.945 and
+0.945, z = -20.2. They occupy about 3.78 units of combined width. Neighboring
piles occupy outer columns (`:28`–`:31`); the map begins at z = -16.5 (`:225`).
The central band in front of the book is therefore available for a low plinth.

Recommended baseline: center x = 0, z about -18.4; width 2.0, depth 1.2, height
about 0.2. Two shallow tiers, clipped corners, dark slate material and a thin
brass rim give it an altar-like form without additional artwork. These dimensions
leave approximately one unit behind it before the cards and 1.3 before the map.
Dimensions and labels remain native-tuning decisions, not gameplay coordinates
or additional persistent game state.

Put one counted clean-red Doom token on the top with a readable **Agenda doom**
label. New agenda doom arrives visibly and updates that token's count; it does
not build a tall pile. At zero, keep the empty plinth and **Agenda doom: 0**, with
no physical Doom token. This follows the existing one-counter-per-total convention
and keeps the place discoverable before the first Mythos.

The altar represents agenda doom only. Keep enemy doom beside each enemy; never
move it to the altar or add it to the altar's local count. A distinct **Total doom:
3/7** readout near the agenda can show progress against the current threshold,
with inspection explaining **2 on agenda + 1 on enemies**. That summary is not
another physical token or an immediate-advancement rule. The official Rules
Reference's Mythos 1.3 includes all doom; `src/gathering/scenario.rs:1911` implements
the selected content's agenda-plus-enemy total and clears both on advancement.
Show each actual removal at its own display location.

A token directly on the tabletop is simpler but does less to establish the
deliberate altar requested by the owner. A circular pedestal is compact but
resembles a stack of tokens. A taller altar or individually stacked counters
would increase obstruction and make quantities harder to judge. The low rectangle
is the recommended compromise. This aesthetic conclusion is design judgment,
not an empirically established optimum.

The important remaining visual risk is projected overlap at oblique camera angles,
not the available floor area alone. Check the local count, label, threshold summary
and Agenda/Act captions together in Scenario focus and a representative shallow
view. Keep both cards selectable and fully inspectable. Use the existing backed
count label to maintain contrast; [Microsoft XAG 102](https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/102)
supports checking labels against their actual backgrounds. No new texture,
simulation, shared infrastructure or altar gameplay action is required. Existing
reduced-motion and nonduplicating save/reopen presentation boundaries still apply.

## Crystal Pendulum and reaction progression

The chooser in `src/gathering/test_cards.rs:281` starts at zero, offers confirm,
cancel, and one-step increments/decrements, and exposes those internal editing
steps as separate actions. `git blame` traces this unchanged design to `f431de9a`;
no documented UX rationale was found. Its numeric storage is `u16`, not a
rules-derived 0–10 limit.

The printed card triggers after a test at its controller's location begins and
names a number for an exact success or failure margin. It has no printed upper
bound. Zero is useful. Even twelve is reachable within the selected decks:
Jacqueline investigates Study with Read the Signs, Pendulum's passive bonus and
Patrice's Promise of Power commitment, then resolves +1: `3 + 5 + 1 + 4 + 1 - 2 = 12`.
Further commitments and boosts mean ten cannot be a hard cap.

Candidate interaction: a compact picker headed **Exhaust Crystal Pendulum**,
direct **Predict 0** through **Predict 10** choices, plus **Other margin…** for
direct numeric entry. Selecting a number exhausts once and records it; dismissal
before selection spends nothing. Remove separate cancel-prediction and
increment/decrement game steps. The existing context menu is a vertical list, so
a compact grid and direct numeric entry would require local UI work. The spec
must retain supported pre-change pending-prediction saves, including their chosen
number, without silently resetting or rejecting them.

The exact phrase **Continue after test-begin reactions** occurs once, at
`src/gathering/test_cards.rs:270`. Confirmation unconditionally restores an
Initiation frame even if no reactions remain; its sole pass choice is absent
from the tabletop's automatic list. That accounts for the second screenshot.
Automatically close the empty window. If another Pendulum, Gregory Gry or another
legal reaction remains, expose it and use **Skip remaining reactions** for the
deliberate decline.

Advance to the next genuine decision, which is not necessarily token reveal:
precommit abilities, commitments and prereveal abilities may intervene. Existing
settlement already skips genuinely empty windows. Preserve those opportunities
and meaningful pre-hidden-information Undo.

## Mask label and separators

Use **Mask: 1/1** on the mat. Inspection still explains that this is an asset
control limit, not a slot that allows replacement. The glossary already defines
that distinction; no new term or ADR is needed.

Use **Hand: 1/2 | Body: 1/1 | Mask: 1/1**, with spacing around each vertical
separator. `src/gathering_tabletop/slots.rs:32` currently joins entries with a
middle dot, but `src/tabletop_support.rs:8` changes dots into slashes on the
rendered text path. A vertical separator is a narrow fix without changing
punctuation across the application. Check actual rendered text/spans: the
accessibility label keeps the dot and would miss the current defect.

## Verification implications

Use the existing player-observation → revision-bound choice → persisted state
boundary already accepted by #89. Add only necessary production UI/presentation
checks around it. Test normal and empty Enemy transitions, Upkeep with both normal
draws and Patrice's refill, all-doom agenda advancement, nested encounter choices,
and save/reopen at pending transitions and decisions. The retained copy trace is
a baseline, not a completed test of the proposed automatic UI.

For Pendulum, cover direct 0/3/12 choices, dismissal, matching/nonmatching margins,
automatic/tied failure, other reactions still available, commitments, and
supported old pending saves. For banners and labels, inspect rendered output and
real input routing; accessibility metadata alone is insufficient. A native run
must establish readable order, timing, motion and actual player control, with
owner acceptance separate from automated evidence.

The four supplied screenshot hashes, driver output, copied attempt, source PDFs
and investigation report are retained under the ignored local verification
directory linked from #89. They remain private evidence rather than repository
assets. No code or prepared-artwork changes were made by this investigation.
