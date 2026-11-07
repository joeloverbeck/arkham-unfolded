# Gathering tokens and shuffling: investigation and selected design

Research on 2026-11-05 against
`92ad202b052105bc88a69bdafd317c03363f41c3`, supporting the owner's
`grill-with-docs` interview under [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89).
Joel selected the recommendations during the interview, closing its final
sideboard choice with "Proceed as you recommend" on 2026-11-05. This records
the source investigation and accepted design; it does not establish implemented
behavior, native appearance, pacing or complete-scenario acceptance. Joel then
authorized implementation; the local candidate and remaining native checks are
described in the [Gathering guide](../experiments/gathering-setup.md).

The current consumer is ordinary Gathering play. The seven reported problems
concern repeated chaos-token values, invisible shuffling, bounty/ammo/charge
presentation, a possible bounty font symbol, and enemy damage counters. The
investigation closes at a concrete selected presentation and coverage contract;
implementation returns to #89 after the design interview. No new animation
framework, physics simulator, card rules, paid service or publication is
selected here.

Consulted for this investigation: `AUTH-1`–`AUTH-3`, `PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`,
`EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `AGENT-5` and
`OPS-1`–`OPS-7`, plus `GOV-1`–`GOV-3`. The foundation and existing presentation decisions continue
to govern the proposal.

## What the current shuffle implementation does

The owner's mulligan diagnosis is correct. After drawing the replacements,
the rules append the rejected cards to the investigator deck and shuffle it.
The renderer gives those same cards a face-down return trip and then ends the
sequence. It has no following mix or shuffle beat.
See [rules](../../src/gathering.rs#L711) and
[visual return](../../src/gathering_tabletop/draws.rs#L129).

The existing [play contract](../foundation/play-contract.md#decisions-and-tabletop-interaction)
already requires visible shuffling. The accepted
[draw choreography](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#gathering-card-draws)
requires individual unseen-hand handling, one draw at a time, approximately
3–4 seconds for five cards, preserved hidden information and no animation
authority over the rules. It specifies the mulligan return but not its shuffle
movement. Replay-related wording in that older section is superseded by
[ADR 0003](../adr/0003-retire-replay.md).

The complete shuffle-site inventory in the current Gathering rules is below.
The inventory comes from the actual permutation sites and their callers, not
only from searching card descriptions for the word “shuffle.”

| Trigger | Current operation and presentation consequence |
| --- | --- |
| Initial investigator decks | [Lead selection](../../src/gathering.rs#L682) shuffles each full deck before opening draws, in lead order. Each shuffle needs to precede that deck's first visible draw. |
| Mulligan replacement | [Confirmation](../../src/gathering.rs#L711) keeps rejected cards out while replacements are drawn, returns them, then shuffles only when cards were rejected. Keeping all cards must not invent a shuffle. |
| Setup weaknesses | After the final mulligan, [withheld weaknesses](../../src/gathering.rs#L735) return to their owners and each affected deck is shuffled. This can cause a further shuffle after the last investigator's mulligan return. Render only information the public setup path has disclosed. |
| Initial encounter deck | [Scenario assembly](../../src/gathering.rs#L805) assembles and shuffles the 26-card encounter multiset. This is a real shuffle after setup, not an encounter draw. |
| Player-deck exhaustion | [Draw continuation](../../src/gathering/draws.rs#L88) moves discard into an empty deck and shuffles when another card is needed. It can occur inside one simultaneous draw. The exhaustion horror follows completion of that draw and its nested consequences. If both piles are empty, no shuffle occurs. |
| Encounter-deck exhaustion | [Rebuild](../../src/gathering/scenario.rs#L1945) returns encounter discard and shuffles, called at the next [mythos continuation](../../src/gathering/phases.rs#L561), [encounter draw](../../src/gathering/encounter_search.rs#L545), or [search cleanup](../../src/gathering/encounter_search.rs#L516). It must not rebuild inside an unresolved search. |
| Backpack | [Finishing its search](../../src/gathering/backpack.rs#L162) removes chosen attachments and shuffles the remaining investigator deck. Zero chosen attachments still reaches the shuffle instruction. The publicly inspected remaining cards return face-down first. |
| Kicking the Hornet's Nest | [Search finish](../../src/gathering/encounter_search.rs#L516) shuffles the encounter deck, including the no-match branch. Cards previously disclosed in the tray return before mixing. |
| On the Hunt | [Search finish](../../src/gathering/encounter_search.rs#L516) shuffles; its [no-match branch](../../src/gathering/encounter_search.rs#L648) shuffles before initiating the fallback encounter draw. The fallback must not appear to precede mixing. |
| Patrice's elder sign | After the player selects the retained discard, [all other discarded cards return](../../src/gathering/retry.rs#L83), then her deck is shuffled. Declining does not shuffle; [selection requires more than one discard](../../src/gathering/retry.rs#L46). The retained card visibly stays in discard. |
| Winging It played from discard | [Event completion](../../src/gathering/player_events.rs#L205) returns the event to its owner's deck and shuffles. Playing it from hand follows ordinary discard cleanup and must not show this shuffle. |
| Agenda 2b | [The reverse](../../src/gathering/scenario.rs#L1909) returns encounter discard, shuffles, and then discards until a Ghoul is found. The existing qualified S8 no-match interpretation rebuilds once after the finite search; any later presentation must retain that qualification rather than silently changing it. |

These are current code facts, not a new fidelity certification of every
underlying rule. Relevant rules were also checked independently against the
retained AHC60 Rules Reference at
`local/issue-1-evidence/sources/rules-reference.pdf`, SHA-256
`14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a`.
Its Drawing Cards/Discard Piles entries require completing a simultaneous
draw across a reset, exhaustion horror after the entire draw, and no shuffle
of an empty discard. Mulligan and setup prescribe replacement-before-return
and the later return of opening weaknesses. Encounter Deck requires rebuilding
an empty encounter deck. See [publisher AHC60 source][rr] pp9–10, 15–16 and 27;
the accessible [publisher AHC01 source][old-rr] independently supplies the same
basic obligations. Existing [scenario research](issue-89-rules-scenario-tony.md)
owns the qualified Agenda 2 no-match interpretation.

## Motion research and alternatives

A physical riffle divides the deck into two face-down packets, releases their
raised inner edges into an interleave, and reforms one deck. That identifying
sequence is documented in the [New Jersey Division of Gaming Enforcement's
published definition][riffle] p15. It is used here solely as a physical-motion
reference; casino procedures are not Arkham rules or application requirements.

Mash shuffling is also a real practice for sleeved cards: the [Grand Archive
publisher's tournament guidance][mash] recommends it when using sleeves, and
[Dragon Shield][sleeves] describes the handling feel of its sleeves during
mash shuffling. These sources support physical plausibility, not the claim
that a particular virtual animation will feel pleasant. Their shuffle-count
requirements do not apply to Arkham or to this app's already-randomized deck.

[Google's duration and easing guidance][motion] recommends brief but readable
transitions and smooth changes in speed. It concerns interface transitions;
its desktop duration numbers are not a measured prescription for a multi-phase
card-handling animation. [W3C's animation guidance][reduced-motion] supports
turning off nonessential motion. That is consistent with this app's existing
reduced-motion contract, without claiming web accessibility certification of
the native application.

| Candidate | What the player can read | Cost and drawback |
| --- | --- | --- |
| Current return plus a “shuffled” caption | A textual explanation, but the visible handling still looks like placing known cards on top | Smallest implementation; fails the explicitly requested physical shuffle effect. |
| Overhand packet transfers | Repeatedly lift and drop small packets | Simple rigid geometry, but from the app's overhead view it can resemble a few cuts and preserve the impression that the returned packet stayed together. |
| Split, interleave, square | Two packets visibly become one mixed stack | Best proposed balance of recognizable mixing, compact footprint and existing rigid-card geometry. |
| Full riffle and bridge | A conspicuous casino-style shuffle with flexed cards | Needs convincing bending or deformable geometry; longer, more theatrical, and easier to distract from the next game decision. |
| Face-up fan, airborne scatter or vortex | Dramatic but obviously digital mixing | Risks hidden information, excessive footprint and motion burden; conflicts with the established unseen-person card handling. |

**Recommendation:** a restrained, face-down, mash-style two-packet interleave
followed by squaring the deck. The name is descriptive; the chosen visual
need not simulate hand anatomy, contact forces, sleeve friction or the
permutation algorithm.

## Selected choreography

Target approximately **1.3 seconds per actual shuffle**, with a native tuning
range of 1.1–1.5 seconds. This timing is an authored starting proposal, not a
research result. Card return travel happens first and is additional to it.
Preserve the accepted 0.7-second individual draw cadence.

| Phase | Initial timing | Visible treatment |
| --- | --- | --- |
| Gather and lift | 0–160 ms | The returned cards join the face-down deck; the deck lifts slightly from its own mat. The known-return copies lose inspectable faces before mixing begins. |
| Split | 160–400 ms | Two near-equal packets separate just enough to read as distinct. A slight height difference and restrained opposing yaw suggest an unseen pair of hands. Avoid a large lateral spread into neighboring cards. |
| Interleave | 400–880 ms | The packets approach through their inner corners; short groups of visible card edges alternate into one stack with a slight stagger. Show actual interleaving layers rather than two opaque blocks clipping through one another. All exposed surfaces use the appropriate deck back and neutral card edge. |
| Square | 880–1160 ms | The small uneven edges compress into a clean rectangle. Motion decelerates as the stack becomes aligned; no elastic bounce or extra cut is necessary. |
| Settle | 1160–1300 ms | The squared deck lowers to its exact resting position and the next queued draw or consequence becomes eligible. |

The actual table footprint constrains the split: keep it near the deck's
existing mat, gain visual separation through a shallow lift and yaw, and
verify the closest investigator discard/threat rows and encounter piles.
The scene currently draws each deck as a [body and top face](../../src/gathering_tabletop/scene.rs#L42),
so mixing needs a temporary anonymous packet/edge representation rather than
moving that single top card. The static deck is hidden during the transient
representation and restored exactly once at the endpoint.

Use the real card count for packet sizes and final height. For one card,
perform only a short face-down square/settle; there are not two packets to
mix. An empty deck must not become a stack, and an empty-discard no-op must
not claim a shuffle. For small decks, use their actual number of card layers.
For larger decks, a bounded set of edge groups can suggest the stack, provided
the visible total height and public count stay correct. This is visual
aggregation, not access to hidden identities or their permutation.

One legible interleave communicates the committed shuffle. Repeating seven
physical shuffles would add delay without making the rules randomization more
correct. Cosmetic asymmetry can be a stable function of the cue's public
sequence and deck identity; presentation must never consume game RNG.

Camera movement remains the player's choice unless the existing follow-camera
option is enabled. Shuffle at the actual deck, retain the ordinary status and
history explanation, and avoid teleporting the deck to Test or opening a
shuffle modal. Existing inspection/menu/reading stops pause the choreography.
Reduced motion settles directly to the committed state while preserving the
readable explanation that the deck was shuffled. Enabling reduced motion
during a shuffle must settle it without issuing a new rule operation.

## Required information and lifecycle

The current [DrawBatch](../../src/gathering/moments.rs#L20) exposes the source,
drawn cards and mulligan returns only. Its [cue](../../src/gathering/moments.rs#L35)
is attached to a committed revision and optional presentation step. It
contains no shuffle event, no shuffled-card count at the operation, and no
reset position within a draw. Count comparison deliberately avoids
interpolation across a reset in
[draw playback](../../src/gathering_tabletop/draws.rs#L388).

**Proposed implementation seam:** emit a small, ordered, transient deck
operation cue at each real shuffle/return/reset site. It must identify the
deck, public source/count information required to render it, and its position
relative to draw transfers and existing presentation moments. No hidden deck
order or list of anonymous face identities belongs in that cue. Do not infer
a shuffle from history prose or before/after counts: Backpack can shuffle
without changing the count, and a refill can hide several operations inside
one net count change.

An ordered sequence can remain local to the existing Session-to-presentation
path. The renderer does not need a general animation timeline, second rules
engine, persisted playback tape or new save-version policy. If a proposed
implementation cannot preserve interrupted draw ordering without changing
saved semantics, that consequence must be exposed before proceeding, not
hidden as a presentation detail.

The existing [presentation owner](../../src/gathering_tabletop/presentation.rs#L85)
only accepts already-saved choices. It selects cues by revision and step,
waits for the current view to reach the renderer and for motion to finish,
then advances. Extend that same owner to shuffle activity; a shuffle must
count as ongoing motion for advancement and game-choice gating. The current
[advance system](../../src/gathering_tabletop.rs#L1133) checks `CardMotion`
only, and thus would not automatically wait for a separate shuffle component.

Retain current lifecycle behavior: opening the same saved Session in place
can retain its exact ongoing transient state; a genuinely different or fresh
load restores authoritative endpoints without inventing historical motion.
Scene rebuilds, camera movement, art becoming ready, Inspect, reduced-motion
changes and Undo must not start another shuffle or consume another random
number. Save failure must not start the proposed cue. Restart uses the newly
committed setup and its real shuffles, preserving the prior attempt as the
existing restart contract specifies.

## Worked cases that distinguish the proposal

1. **Replace two mulligan cards.** Two cards leave the hand and wait beside
   the deck; two replacements arrive; both rejected cards turn face-down and
   return; the deck visibly interleaves and squares. Their later positions
   remain unknown. Current handling stops after return; the proposed extra
   movement is the difference being judged.
2. **Backpack with no selected attachments.** The inspected cards return
   face-down and the deck mixes despite unchanged deck count. A count-based
   detector would miss it. The player still explicitly finishes the search.
3. **On the Hunt with no eligible enemy.** Return the disclosed search cards,
   shuffle, then animate the fallback draw. The search's failed selection
   must not fabricate an enemy or draw before the shuffle.
4. **Patrice refills five from an empty hand with two cards left.** Show the
   two-card draw segment, discard-to-deck reset and mixing, then the remaining
   three. The rule operation is still one simultaneous draw; animation does
   not introduce a decision or expose later faces before their permitted
   observation. A separate case with Watcher already retained in hand needs
   four cards, and its forced attack may interrupt after the first two and
   before reset. The current draw record must not be mistaken for proof that
   the reset/interruption position is already available to the renderer.
5. **Patrice keeps one discard after her elder sign.** That selected card
   stays face-up in discard; the others turn face-down, join her remaining
   deck and mix. Decline or an empty movement must not animate a return.
6. **Agenda 2.** Return encounter discard, mix, then start the discard-until
   operation. Preserve the existing finite no-match interpretation and any
   distinct post-search rebuild rather than merging them across the search.
7. **Interruption and missing art.** Pause during the interleave with Inspect;
   resume the same motion, then reopen in a fresh process to the saved
   endpoint. With absent deck artwork, anonymous readable backs still move;
   neither path changes the next random outcome.

## Verification and selected direction

Source inspection establishes the shuffle gap and integration risks. It does
not establish that the proposed motion looks good. A later accepted candidate
needs focused production-path checks for the inventory above, no-op and
one-card cases, public counts, save failure, interruption, reduced motion,
stale choices and preservation of the next draw/random state. These checks
should use the existing rules oracle and actual presentation registration,
not a second model that merely repeats the implementation.

Native evidence must include complete motion for a mulligan return, a search
cleanup and a player-deck refill, plus the crowded personal and encounter
layouts. A still screenshot cannot establish interleaving or pacing. Judge
whether the player can tell that the deck was mixed, whether the next decision
arrives without irritating delay, and whether edges, captions or neighbors
obscure the action. That is the acceptance surface for appearance and pace;
the broader complete-scenario playthrough remains with #89.

Joel accepted the visual treatment and initial pacing in interview answer Q1:
approximately 1.3 seconds of compact face-down mixing at each actual shuffle,
with the existing reduced-motion setting removing the movement. This selects
the design; it is not native visual acceptance. The strongest
alternative is a simpler overhand packet shuffle; it is cheaper but less
explicit about interleaving from the normal camera. Exact curves, offsets,
edge-group count and private Rust types are reversible implementation choices.

## Chaos-token rows

The repeated numeric value is a display defect, not double application of the
modifier. [The result renderer](../../src/gathering_tabletop/results.rs#L261)
passes the signed contribution to `token_row`, which separately adds the image
and unconditionally prefixes `face.label()` at lines 302–313. The same details
renderer serves the live result and [History](../../src/gathering_tabletop/ui.rs#L527).

Keep one row per physical revelation, including identical faces. Suppress the
second textual number only when a numeric face and its contribution agree.
Keep actual contributions for symbols, which do not have a printed numeric
value. Use the existing frozen report, never later board state:
[report data](../../src/gathering/test_report.rs#L27),
[resolution capture](../../src/gathering/runtime.rs#L1859), and
[symbol values](../../src/gathering/runtime.rs#L2049).

| Case | With artwork | Without artwork |
| --- | --- | --- |
| Ordinary numeric −2 | `[−2 token] −2` | `−2` |
| Skull resolving to −2 | `[skull token] Skull · −2` | `Skull · −2` |
| Curse | `[curse token] Curse · −2` | `Curse · −2` |
| Canceled numeric token | `[−2 token] −2 · Canceled — no contribution` | `−2 · Canceled — no contribution` |
| Ignored symbol | `[skull token] Skull · Ignored — no contribution` | `Skull · Ignored — no contribution` |
| Automatic failure | `[auto-fail token] Automatic failure` | `Automatic failure` |

Automatic failure currently gets a misleading `+0` in its row, although the
report's separate outcome explanation is correct. Omit that numeric amount.
If a supported effect produces a different contribution from a numeric face,
show both meanings explicitly, for example `−2 → −4`; this display rule does
not introduce such a card effect. The aggregate chaos modifier remains in
the final arithmetic. Current `TokenFace` has no Bless variant; no new token
rules are implied by this presentation correction.

## Token artwork and the font investigation

The following inputs were found and inspected. Paths are relative to the
unchanged source archive named in the
[source contract](../foundation/sources-and-verification.md#presentation-asset-preparation).

| Purpose | Archive-relative PNG | Size | SHA-256 |
| --- | --- | --- | --- |
| Bounty | `images/other/unclassified/en/sce-base-save/Bounty--ImageURL--5e10a5f17dda3cb8d795.png` | 384×384 | `1128d732cab07a656e0607a95bbc28ca6830f7d945a05c205a4779d7df03bc12` |
| Ammo | `images/other/unclassified/en/sce-base-save/Ammo--ImageURL--1d0027bdf77a9f2c7229.png` | 384×384 | `4eefba18f84d9afb06815853c93ea4931080b42cd103c3d63d0df0c656a38f6c` |
| Spell motif selected by Joel | `images/bundles/7af1e2752099652f0a9dbc894013fd3c9803ec75e44885106e4c613b59763445/Sprite/Spell--2d26e3476eb8f7e7.png` | 157×156 | `ecea8bdecb56f0ff9e1908ce64e1d457de90aa2bd00afe16f8fa85c2dc8a651c` |

Bounty is a cream/gold circular wanted-card/target illustration; ammo is a
red circular target and white bullet. Match the current clue token's visual
size and preserve each emblem, with a separate readable number including 1.
One physical counter represents the total, following the existing clue
convention; six bounties do not require six overlapping token models.

The exact Spell sprite appears nearly white in isolation, but its alpha
channel ranges from 0 to 255 and its luminance varies. It must not be rejected
as blank from that preview. The archive catalog binds it to sprite path ID
`-1014947186258289388` in the named bundle. A 319-square Texture2D export also
exists, but it is not the exact input Joel selected. Proposed treatment:
a solid deep-violet disk, the supplied pale motif, restrained rim and physical
thickness, and a separate backed count that leaves the motif visible. Joel
selected this direction in interview answer Q5. A
contrasting visual candidate must establish the actual composition before
claiming readability. No charge image has yet been generated or prepared.

The installed prepared `assets/fonts/arkham-symbols.ttf` identifies itself as
arkhamicons Regular Version 1.0 and has SHA-256
`186176d7243c72d272a50c24e833fb77d983e7c4c6c99bc64f89c8322dbd8187`.
It is byte-identical to the [ArkhamCards font][font] at commit
`74e2d97e26e9bb46ac3a2b2cf1b160fe099add06`. The same revision's
[named icon configuration][font-map] lists 107 icons, with no bounty, ammo,
charge or target glyph. Inspecting the actual cmap independently confirmed
the installed font's coverage. A generic typography bullet is not a bounty
symbol. Keep “bounty” and “bounties” in card text; do not substitute an
approximate symbol, rewrite the name Bounty Contracts, or upgrade the font
as an incidental part of this work.

## Counter meaning, positions and zero counts

The rules already distinguish Ammo, Charge, Supply, Resource, Offering, Bounty
and Key in [UseKind](../../src/gathering/cards.rs#L41). The public
[AssetView](../../src/gathering/runtime.rs#L99) exposes only its numeric uses.
The renderer needs the actual use type. Infer neither ammo from a weapon name
nor charges from the Spell trait: a spell without charges gets no invented
counter, and offerings/supplies keep their distinct meaning. Verified charge
consumers include Mists of R'lyeh, Eyes of the Dreamer, Azure Flame and
Clairvoyance. Do not relabel all other uses as charges or erase their captions.

The current asset renderer special-cases Contracts as `N bounties` and uses
`Uses N` for other positive totals; see [play.rs](../../src/gathering_tabletop/play.rs#L670).
Recommended placements and retained information:

- Bounty Contracts: one bounty counter centered on its card.
- Ammo and charge assets: one correctly typed counter in a consistent central
  position on the card, with the corresponding count caption removed after
  the replacement is readable. Full inspection keeps original artwork clear.
- Enemies: one damage counter above the card and horizontally centered; bounty
  on the left margin and doom on the right, clear of the printed central
  damage/horror icons. Side groups are vertically centered with a visible gap.
- Keep titles, location, engagement, readiness, attachments and unrelated
  damage/horror information; removing one redundant count does not remove the
  entire card caption or accessible English description.
- Keep counts upright and associated with the correct physical card when it
  exhausts, moves or is inspected. Use the actual rotated footprint for clearance.

Joel accepted stable side assignments in Q2. He explicitly anticipates other
counter types, such as clues, on card margins in later scenarios. The layout
should admit multiple distinct types per side and center each occupied group.
Assign additional supported types to stable sides, balancing the assigned
types where practical; do not move an existing type across the card when
another appears or disappears. This is a presentation convention, not
activation of future scenario rules or a general token-layout framework.

Joel accepted the following zero policy in Q3:

| Counter | Zero behavior |
| --- | --- |
| Ammo and charges on an asset | Keep the counter and show `0` while that asset remains in play. |
| Bounties on Bounty Contracts | Keep the counter and show `0`. |
| Bounty or doom on an enemy | Omit that counter at zero. |
| Damage on an enemy | Keep the counter and show `0/effective maximum health`. |

Enemy damage means **damage accrued / effective maximum health**, not remaining
health. A two-health enemy starts at `0/2`; after one damage it shows `1/2`.
The existing investigator harm renderer already has those amount/capacity
semantics and named missing-art substitutes:
[tabletop_pieces.rs](../../src/tabletop_pieces.rs#L281).
The existing rules-owned [enemy_health](../../src/gathering/scenario.rs#L1655)
supplies the correct denominator, but it is absent from the enemy observation
used by the renderer. Expose it without parsing printed text or duplicating
health formulas. For example, the current three-investigator Ghoul Priest has
15 health; elimination does not turn that denominator into 10 or 5.

## The enemy-pile conflict

Several unengaged enemies at one location currently form a collapsed pile.
[Only its top card gets a visible caption](../../src/gathering_tabletop/play.rs#L897).
The [positioning](../../src/gathering_tabletop/play.rs#L1169) caps the complete
stack offset at one world unit and individual offsets at 0.25. Per-enemy damage
tokens are larger than those offsets, and side counters would overlap too.

There are two materially different promises. Keeping piles allows only the
top enemy's own counters on the collapsed table pile, with each individual
enemy's values available through the existing chooser/inspection. A pile
counter must never imply a shared damage total. This was the initial least
disruptive recommendation, but is an explicit exception to literal
always-visible counters for every enemy. Joel did not select it in Q4.

Displaying every enemy's counters simultaneously requires spreading the
cards into their own space. A small fan beside each room can expose some top
badges, but does not establish room for every side token or a crowded map.
The consumer and verification scope must include multiple
unengaged enemies, adjacent rooms, investigator standees, exhausted cards,
identical enemy copies and all their tokens. Do not silently trade away
individual visibility or claim an untested fan solves the space constraint.

Instead, Joel requested deeper research into solutions used by other players
and selected seeing all unengaged enemies without crowding the map as the
desired outcome. His selected alternative is a tall sideboard to the map's
left, grouping enemy cards beneath their location name, with matching colored
enemy-presence markers above the map location and beside its sideboard header.
Location sides remain reserved for investigator standee slots. He also named
Massive enemies as a relevant location-resident category when supported.
After the follow-up research, Joel answered Q6 with "Proceed as you recommend,"
selecting named groups with three columns and wrapped rows, matching color
plus short identifier and count, cross-highlighting, individual enemy actions,
preserved card size/manual camera and adjusted Scenario framing. Ordinary
engaged enemies remain in investigator threat areas. This does not add new
enemy rules. The follow-up research and decision are recorded in
[Gathering enemy location display](gathering-enemy-location-display.md).

## Loaded artwork and documentation consequences

Hide a caption only when its replacement component is actually readable.
[Art::image](../../src/gathering_tabletop/art.rs#L126) currently returns a
queued handle; that does not prove image load or decode completion. Missing,
loading, invalid or failed artwork needs the component's readable English
substitute, including the zero/count semantics above. Apply this per component
so missing ammo art does not restore unrelated old captions. This follows the
accepted missing-art contract; it introduces no new user preference gate.

Prepare new selected files in a fresh external collection, preserving source
bytes, the active collection and personal settings. Bind exact source/output
identities and any bounded charge-token transformation in the existing
preparation recipe. Ordinary play must consume the relocated prepared assets
without access to the archive. No image or font payload belongs in Git.

The selected typed-counter, zero/fallback, visible-shuffle and enemy-sideboard
requirements are recorded in the existing play contract and ADR 0002, including
the detailed placement and result-row conventions. The glossary defines the
sideboard and its presence markers without implementation details. Exact
geometry, easing, private Rust types and image composition remain reversible
implementation choices. GitHub #89 continues to own activation, evidence and
complete-scenario acceptance; this research note is not a second work queue.

The selected design needs later production-path checks for distinct use types,
zero/one/multiple amounts, per-image loading/failure, rules-owned maximum
health, symbol/numeric/canceled/ignored/auto-fail rows and saved result History.
Native inspection must judge motif/count legibility, exhausted-card placement
and the selected enemy-sideboard behavior. No application tests or native token or
shuffle checks have been run for this design-only investigation.

[font]: https://github.com/zzorba/ArkhamCards/blob/74e2d97e26e9bb46ac3a2b2cf1b160fe099add06/assets/arkhamicons.ttf
[font-map]: https://github.com/zzorba/ArkhamCards/blob/74e2d97e26e9bb46ac3a2b2cf1b160fe099add06/assets/arkhamicons-config.json

[rr]: https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf
[old-rr]: https://images-cdn.fantasyflightgames.com/filer_public/c4/b0/c4b0d66c-d79e-411b-bdb5-b5d8c457d4bc/ahc01_rules_reference_web.pdf
[riffle]: https://www.nj.gov/oag/ge/docs/EmergencyRegs/042511/1369constructionandapplication.pdf
[mash]: https://trg.gatcg.com/tournament-mechanics/shuffling
[sleeves]: https://www.dragonshield.com/collections/classic-sleeves
[motion]: https://m1.material.io/motion/duration-easing.html
[reduced-motion]: https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions
