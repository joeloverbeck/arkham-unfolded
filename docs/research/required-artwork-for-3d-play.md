# Required artwork for 3D play: removal investigation

Investigated on 2026-11-07 against source revision `0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a`. Source links below pin that revision. This is research and a proposed implementation handoff; it does not report an implemented or tested change. [ADR 0004](../adr/0004-require-artwork-for-3d-play.md) records the owner's selected product direction and supersedes the earlier playable-substitute requirement.

## Conclusion and selected boundary

Remove native play without a complete, successfully loaded prepared artwork collection across Jacqueline, Tony, Study escape and Gathering. Keep the headless player driver and asset-independent rules/CI verification as engineering interfaces. An incomplete installation must reach a blocking startup error window offering **Choose asset folder**, **Retry** and **Quit**, with the same useful diagnosis in the terminal. No player or automatic game decision may occur behind that window.

One bounded implementation issue related to [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89) is warranted: this crosses startup, persistence ordering, four presentation consumers, shared component rendering, configuration, launch tooling and verification. Joel subsequently invoked `to-spec`, publishing [#124](https://github.com/joeloverbeck/arkham-unfolded/issues/124) with `ready-for-agent`; its [current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/124#issuecomment-6706875824) owns execution and acceptance status. Implementation remains unactivated at publication. The existing finite asset recipes and admission gates make a bounded implementation credible; source inspection does not establish the eventual patch's correctness, native usability or duration.

The strongest smaller alternative is to reject missing collections at launch while leaving all substitute rendering intact. It would prevent most accidental no-art launches but retain the parallel captions, layouts and maintenance burden motivating this request. Deleting only `--no-art` is weaker still: normal launches already accept incomplete or absent collections. Complete retirement therefore includes the exclusive substitute branches and their obsolete acceptance expectations.

## Finite artwork requirement and preparation evidence

The application already compiles mode-specific selections from four committed recipes ([selection dispatch, `src/assets.rs:125`][selection]). Completeness means the entire current selection for the launched mode, including content not currently visible and the symbol font. It does not mean the external archive's entire catalog or future unsupported content.

| Native mode | Current recipe | Required files |
| --- | --- | ---: |
| Jacqueline | [`asset-selection.json`](../../resources/asset-selection.json), collection v3 | 15 |
| Tony | [`tony-asset-selection.json`](../../resources/tony-asset-selection.json), collection v5 | 43 |
| Study escape | [`party-asset-selection.json`](../../resources/party-asset-selection.json), collection v2 | 57 |
| Gathering | [`gathering-asset-selection.json`](../../resources/gathering-asset-selection.json), collection v3 | 144 |

Gathering's recipe declares 149,472,011 payload bytes: 105 card faces/reverses, two card backs, 20 tokens, 12 class images, three standees, one symbol font and one reveal image. Its existing coverage test independently compares selected deck/scenario identities and required reverses against the recipe ([`src/assets.rs:840`][coverage]). Preserve and extend this inventory check when supported content changes; a rendered starting hand cannot establish full scenario coverage.

A contemporaneous read-only run of the existing `target/debug/arkham-assets check-gathering-launch` exited 0 and reported `/home/joeloverbeck/.local/share/arkham-unfolded/gathering-assets` complete at **144/144 verified**. The binary was not rebuilt, its identity was not bound to the inspected source revision, and no retained output artifact was supplied for this report. This observation supports availability of the local collection to that installed checker; it is not exact-head verification, image/font decoding evidence or a native launch result.

Default folder names lag current recipe versions: `assets-v2`, `tony-assets-v4`, `party-assets-v1` and `gathering-assets` ([`src/asset_config.rs:183`][defaults]). The inspected Jacqueline v2 and Tony v4 installations held 14 and 29 files respectively, against current requirements of 15 and 43. Strict startup will expose these obsolete installations. Preserve prior folders and saved preferences; prepare a fresh complete collection through the existing preparation path when needed, without overwriting or treating an old manifest as complete.

## Startup and loading changes

| Existing seam | Direct source finding | Implementation consequence |
| --- | --- | --- |
| Native dispatch and CLI | [`src/bin/tabletop.rs:14`][entry] selects all four native modes. Each parser supports `--no-art`; Gathering does so at [`src/gathering_tabletop.rs:351`][gathering-cli]. | Remove native bypasses and help text; ordinary launch must always resolve an artwork root. |
| Collection inspection | [`src/assets.rs:311`][inspect] accepts a recognized earlier manifest and verifies individually available files. `complete` requires the current manifest and every expected file at line 354. | Retain read-only diagnostic inspection, but require completeness for native startup and replacement-folder acceptance. |
| File checks | [`src/assets.rs:637`][file-checks] restricts relative paths, rejects symbolic links/nonregular files, and checks expected size and SHA-256. | Preserve these existing checks; report their failure without reducing the requirement to existence alone. |
| Gathering preparation | [`src/gathering_tabletop/art.rs:23`][prepared] accepts an optional root and incomplete report; `load` at line 54 requests only verified roles. Its default represents disabled artwork. | Production construction must require a complete collection and register every required image/font. A default empty-art resource cannot authorize play. |
| Actual load success | [`src/tabletop_health.rs:303`][load-predicate] excludes only Loading/NotLoaded states. Failed/unknown states and an empty handle list satisfy its readiness predicate. | Require successful loading/decoding for all required assets; retain identity/path information for actionable errors. Rendering performance readiness remains a separate necessary check. |
| Initial save ordering | [`src/gathering_tabletop.rs:434`][startup] opens the table before inspecting artwork. `Table::open` creates and saves a new session at [`:104`][open-table]. | Move validation before creation and defer the first persistent save through successful asset admission. Failed startup must neither create a save nor change an existing save. |
| Action admission | Gathering's gate at [`src/gathering_tabletop.rs:611`][admission] rejects game-changing actions while checking; automatic framework advancement also checks readiness at line 1480. | Connect artwork readiness to the production admission boundary and retain rejection of queued inputs from checking/error frames. |
| Later loss/failure | [`src/tabletop_health.rs:109`][health] resets sampling on an ineligible observation without revoking an already-ready phase. | A required runtime asset becoming unavailable must block further play and preserve the last committed save. Retained decoded assets need no continuous disk rescanning merely because their source file moved. |
| Folder settings | [`src/tabletop/asset_settings.rs:171`][settings] rejects unrecognized folders but explicitly permits saving recognized incomplete collections at lines 181–207. | Accept only a complete replacement, preserve the prior setting on cancel/failure, and use the startup recovery path before gameplay can exist. |
| Configuration and exit | [`src/asset_config.rs:88`][config] gives CLI override precedence over saved/default roots; CLI does not persist. [`src/bin/tabletop.rs:23`][entry] prints returned errors and exits nonzero. | Preserve precedence and expose the effective root. Integrate terminal diagnostics with the selected blocking window and explicit Retry/Quit behavior. |

Jacqueline synchronously reads its symbol font and silently returns `None` on read failure ([`src/tabletop/symbols.rs:83`][font]); the other modes request it through the asset server. Both paths belong to the new requirement. Inspection currently reads and hashes files before the asset server reopens them, so validation success is not proof that subsequent decoding succeeded. The implementation must exercise this actual handoff, including a failure after preflight.

## Retire substitutes without deleting ordinary presentation

The removal is broader than a launch flag, but an absent optional image does not always mean missing artwork.

| Surface | Exclusive substitute behavior to remove or simplify | Behavior to retain |
| --- | --- | --- |
| Counters and harm | Named fallback counters and alternate offsets in [`tabletop_counters.rs:89`][counters], [`tabletop_pieces.rs:281`][pieces-harm], [`tabletop_layout.rs:212`][harm-layout] and [`gathering_tabletop/card_counters.rs:102`][gathering-counters]. Gathering also retains token captions until decoding at [`gathering_tabletop/play.rs:657`][token-captions]. | Counts, zero values, effective capacities, meaningful status text and readable semantic names in inspection/history. |
| Standees and cards | Substitute capsules in [`tabletop_standees.rs:60`][standees]; optional card textures in [`tabletop_pieces.rs:64`][pieces] and [`tabletop_discard.rs:20`][discard]. | Physical bases, rims, card geometry, empty areas and legitimately concealed card sides using their required back artwork. |
| Navigation, inspection and symbols | Text-only image substitutes/alternate widths in [`tabletop_ui.rs:166`][navigation], optional-image inspection paths in [`tabletop/inspection.rs:21`][inspection], and missing-font substitutions in [`tabletop_harm_text.rs:107`][harm-text]. | English rules text, actionable labels, history, titles and effective card inspection; artwork never replaces rules authority. |
| Gathering card lookup | [`CardRender::image`, `gathering_tabletop.rs:221`][card-lookup] mixes unavailable artwork with legitimate empty discard/victory piles. | Separate genuine empty/hidden state from failure to supply a required texture; do not replace every `Option` mechanically. |

For example, an empty victory display has no top card to render and remains legal. A nonempty victory display whose required card image failed to load must block native play. Similarly, ordinary Gathering titles/status at [`src/gathering_tabletop.rs:884`][ordinary-labels] remain useful with artwork. Image-less choice/cancel helpers in [`src/tabletop_reveal.rs:566`][reveal] do not by themselves establish an asset fallback.

Launch tooling and tests also preserve no-art paths: [`tools/src/native.rs:175`][native-tool] accepts the flag and forwards it at line 686; [`tools/tests/launch.rs:182`][launch-tests] exercises that mode. Replace these expectations with refusal/recovery checks. Preserve the explicitly retained headless driver ([`src/bin/driver/gathering.rs:73`][driver]) and asset-independent CI ([`.github/workflows/check.yml:41`][ci]); neither establishes native artwork readiness.

## Proposed proof and acceptance surface

The decisive automated seam is **production startup → artwork admission → game actions/persistence**, including recovery through Choose asset folder and Retry. Tests of only the report formatter or a parallel test-only gate would leave the claimed boundary unproved. Valid startup and recovery must wait for all required images and the font to decode, then satisfy the existing rendering gate before permitting player or automatic actions.

| Case | Required observable result |
| --- | --- |
| Root absent, wrong root or expected structure wholly absent | Blocking error identifies the resolved root and expected collection structure; no per-file flood; Choose asset folder, Retry and Quit remain usable. |
| Missing, invalid, incompatible or obsolete manifest | Distinct collection diagnosis; do not misreport the folder as a complete current collection or silently adopt verified subsets. |
| Structure present but required files missing | State total unavailable count and show at most five expected paths, followed by the remaining count. No new/changed save and no game decisions. |
| Unreadable, changed, nonregular or disallowed-link file | Explain the relevant cause with the same bounded path sample; preserve the collection and existing setting. |
| Decode/load failure after successful preflight | Remain blocked with asset identity/path and a useful error; failed/unknown states and an empty required list cannot pass. |
| Complete collection and successful decoding | Admit the same supported native mode only after every required asset and rendering readiness succeed. |
| Choose, cancel, failed choice, repair then Retry, and Quit | Exercise the actual startup window, setting preservation and revalidation. Retry must not reuse stale completeness or admit queued game input. |
| Required runtime asset lost after admission | Stop further play before another committed decision, preserve the last committed save, and expose failure rather than substitute art. |
| Existing save and fresh save path under refusal | Existing bytes remain identical; a fresh path remains absent. Successful recovery resumes/starts through the ordinary persistence path. |

Existing fallback tests need adjudication against the changed contract, including [`gathering_tabletop/tests/token_guidance.rs:605`][fallback-test], [`gathering_tabletop/tests/counters_and_enemies.rs:54`][counter-tests], and incomplete-folder acceptance in [`tabletop/asset_settings.rs:264`][settings-tests]. Keep tests for legitimate empty components and meaningful captions. Asset fixtures at [`src/assets.rs:1190`][fixtures] use synthetic payload bytes; use decodable synthetic image/font fixtures where real decoding is the property under test. Do not make ordinary CI depend on personal artwork.

The native acceptance-bearing increment is a failed launch with a missing/partial collection, recovery through the startup controls into complete-art Gathering, and a representative decision/save/resume without substitute layouts. Exercise representative shared cards, standees, counters, inspection and symbols across the retained native modes. Automated state checks establish refusal and persistence behavior; native inspection and owner use establish readability, recovery usability and the selected experience separately.

## Spec handoff, limits and stopping boundary

Prepare one implementation spec from this report and ADR 0004, with code, launch tooling, tests, current foundation contracts and maintained instructions in scope. Keep archive import, automatic downloading, a general asset manager, new supported gameplay, save-format changes and artwork redistribution outside it. Do not rewrite historical evidence as though the old substitute behavior never existed; mark superseded requirements through the decision record.

No application change, test suite or native launch was performed for this report. The installed-checker observation above is the only runtime asset check described here. Exact-head strict startup, recovery, decoder behavior, post-admission refusal and owner acceptance remain prospective evidence obligations. Source facts support the removal map; implementation decomposition and reliable failure injection remain engineering work within the eventual accepted contract.

The subsequent [published spec](https://github.com/joeloverbeck/arkham-unfolded/issues/124) carries the presented envelope: one coherent local candidate within **six aggregate engineering hours**, including coding, verification and review, with **one owner review totaling at most fifteen minutes, including retries**, and no paid services. The owner review judges failure-message clarity, folder selection/Retry and the complete-art presentation; automated checks cannot supply that usability judgment. These are prospective ceilings, not a delivery estimate, activated work or consumed allowance. The spec owns this envelope, the production startup/action/persistence test seam above, and the publication boundary; the current handoff owns subsequent activation and accounting.

Retain the constitution's default [OPS-4 checkpoint](../principles/principles-for-repos.md#ops-4--non-convergence-and-resource-tripwires-stop-implementation): return after two active hours without an inspectable startup/refusal increment, three failed repair cycles at the same boundary, or an earlier applicable scope/resource tripwire. Return at the ceiling with the candidate and deficits; do not expand into asset-management infrastructure or automatically extend the work. Implementation activation, desktop access and publication retain their separate authority boundaries.

Consulted for this research and handoff: `AUTH-2`, `AUTH-3`, `EVID-3`, `EVID-4`, `AGENT-5`, `OPS-1`, `OPS-2`, `OPS-3` and `OPS-4`. Readiness, implementation, verification, publication and owner acceptance remain distinct.

[selection]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/assets.rs#L125
[coverage]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/assets.rs#L840
[defaults]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/asset_config.rs#L183
[entry]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/bin/tabletop.rs#L14
[gathering-cli]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/gathering_tabletop.rs#L351
[inspect]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/assets.rs#L311
[file-checks]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/assets.rs#L637
[prepared]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/gathering_tabletop/art.rs#L23
[load-predicate]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop_health.rs#L303
[startup]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/gathering_tabletop.rs#L434
[open-table]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/gathering_tabletop.rs#L104
[admission]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/gathering_tabletop.rs#L611
[health]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop_health.rs#L109
[settings]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop/asset_settings.rs#L171
[config]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/asset_config.rs#L88
[font]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop/symbols.rs#L83
[counters]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop_counters.rs#L89
[pieces-harm]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop_pieces.rs#L281
[harm-layout]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop_layout.rs#L212
[gathering-counters]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/gathering_tabletop/card_counters.rs#L102
[token-captions]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/gathering_tabletop/play.rs#L657
[standees]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop_standees.rs#L60
[pieces]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop_pieces.rs#L64
[discard]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop_discard.rs#L20
[navigation]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop_ui.rs#L166
[inspection]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop/inspection.rs#L21
[harm-text]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop_harm_text.rs#L107
[card-lookup]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/gathering_tabletop.rs#L221
[ordinary-labels]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/gathering_tabletop.rs#L884
[reveal]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop_reveal.rs#L566
[native-tool]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/tools/src/native.rs#L175
[launch-tests]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/tools/tests/launch.rs#L182
[driver]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/bin/driver/gathering.rs#L73
[ci]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/.github/workflows/check.yml#L41
[fallback-test]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/gathering_tabletop/tests/token_guidance.rs#L605
[counter-tests]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/gathering_tabletop/tests/counters_and_enemies.rs#L54
[settings-tests]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/tabletop/asset_settings.rs#L264
[fixtures]: https://github.com/joeloverbeck/arkham-unfolded/blob/0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a/src/assets.rs#L1190
