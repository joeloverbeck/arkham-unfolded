# Artwork distribution and installation investigation

Investigated on 2026-11-08 against repository revision `cfb50659f2be5f95ab736d9b1bcc5b5d3a8af0be` and the primary sources linked below. This is investigation evidence and a completed design-interview record, not implementation, redistribution authorization or a published download. The six decisions explicitly selected by Joel are recorded below; technical recommendations do not claim completed implementation or verification. The subsequent `to-spec` invocation published [issue #140](https://github.com/joeloverbeck/arkham-unfolded/issues/140), which owns the bounded work; its [current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/140#issuecomment-6727178541) owns active acceptance status.

The owner's clarified public-download scope is **only the prepared runtime artwork collection**: the selected card images, tokens, icons, symbol font and exported pouch model needed by the supported game. The source resource archive and editable Blender authoring files remain personal development inputs and are excluded. Nothing in this investigation uploads, moves or deletes artwork.

The root interview read the complete constitution at revision 2026-09-30.1. The materially applied rules were `AUTH-2`–`AUTH-3`, `PURP-2`–`PURP-5`, `WORK-1`–`WORK-2`, `WORK-6`, `EVID-1`–`EVID-4`, `EVID-8`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `BOUND-1`–`BOUND-5`, `INT-1`–`INT-4`, `AGENT-3`, `AGENT-5`, `OPS-1`–`OPS-2`, `OPS-6`–`OPS-7`, `PUB-1` and `GOV-1`–`GOV-3`. Carry the relevant consultation into the eventual owning spec; this note does not activate implementation or publication.

## Present demand and boundaries

The [current recipe](../../resources/gathering-asset-selection.json) declares 148 payload files totaling **171,131,797 bytes**, approximately **163.20 MiB**, before the collection manifest and archive overhead. The live inventory below independently verified the installed payload. The pouch contributes 21,374,036 bytes. A future multi-gigabyte catalog is not the present installation requirement.

The [source contract](../foundation/sources-and-verification.md#presentation-asset-preparation) already requires a portable, complete collection outside Git and preserves its source archive. At the inspected implementation revision, Choose asset folder selects an already prepared directory; downloading, extracting and repairing are absent. The [play contract](../foundation/play-contract.md#presentation-assets-and-missing-artwork) now records the selected replacement import flow separately from that implemented behavior. Successful hashing does not replace the current image/font/model loading and rendering admission requirements.

## Verified local installation and cleanup inventory

The current ordinary launch resolves to `/home/joeloverbeck/.local/share/arkham-unfolded/gathering-assets-chaos-pouch-staging` through `~/.config/arkham-unfolded/gathering.toml`. Neither XDG location variable was overridden at inspection. This collection is `arkham-unfolded-gathering-v7`: 148 payload files plus `manifest.json`, with no additional regular files or symlinks. The built `arkham-assets check-gathering-launch` and explicit `check-gathering` both exited 0 with 148/148 identities verified. These checks did not open the native application or establish rendering/owner acceptance.

| Measured artifact | Bytes / identity |
| --- | --- |
| Complete current collection, including manifest | 171,160,715 apparent bytes (163.23 MiB); 171,909,120 allocated bytes |
| Current manifest SHA-256 | `8ac085b6b04afa57e8f333baddaf160ff341e1f16eb829050f7ccd20aa7b37bc` |
| Pouch SHA-256 | `514b48674232d76a2875a6d0b43f02afca889f3777972ecd2d3c25a9791c4483`; unchanged from the authoring delivery |
| Measured runtime-only ZIP | 167,669,563 bytes (159.90 MiB), saving 2.04% |
| Measured ZIP SHA-256 | `1af899966527d619d4bd077c71d9cb565ad394ef2c827e19384707f4927c5fef` |

The ZIP is `/tmp/arkham-gathering-v7-inventory-20261108.zip`, with `manifest.json` at its root. `unzip -tq` passed. It is a temporary research artifact, **not** the requested Documents backup or a distributable release: no complete restore comparison was performed and temporary storage is not durable custody. Its exact digest identifies this measurement, not a future release archive. Most payload bytes are already compressed image/model data.

[PR #139](https://github.com/joeloverbeck/arkham-unfolded/pull/139) merged at the inspected commit on 2026-11-08. Its handoff preserved the prior preference and supplied a staging override; the saved preference observed here subsequently points to v7. The stable `gathering-assets` directory still holds v5. The [existing installation procedure](../experiments/gathering-setup.md#artwork-preparation-and-selection), lines 798–819 at this revision, already requires verified staging to replace the stable installation and forbids staging as the normal target. Therefore the observed problem includes an unfinished installation handoff, beyond documentation discoverability. The [pouch investigation](chaos-pouch-integration.md) records earlier state and must not be treated as the live preference.

These eleven older installed runtime collections were found under `~/.local/share/arkham-unfolded/`, totaling approximately 996 MB allocated:

| Exact relative directory | Manifest family/revision | Payload files |
| --- | --- | ---: |
| `assets` | Jacqueline v1 | 13 |
| `assets-v2` | Jacqueline v2 | 14 |
| `tony-assets` | Tony v1 | 24 |
| `tony-assets-v2` | Tony v2 | 25 |
| `tony-assets-v3` | Tony v3 | 28 |
| `tony-assets-v4` | Tony v4 | 29 |
| `gathering-assets-before-keys-20261106` | Gathering v2 | 143 |
| `gathering-assets-before-spark-20261107` | Gathering v3 | 144 |
| `gathering-assets-before-offerings-20261108` | Gathering v4 | 145 |
| `gathering-assets` | Gathering v5 | 146 |
| `gathering-assets-issue-137-staging` | Gathering v6 | 147 |

Their payload hashes matched their own manifests, and the file censuses contained exactly declared files plus each manifest. One historical payload is absent from v7: Jacqueline's `markers/reaction.png`, 6,488 bytes, SHA-256 `c30319ea053f82dfde7d150800a4d37552c1e8a73931635b336c8de404034994`. A backup of v7 alone would lose this file and the older manifests when retiring those collections.

Documentation-guided inspection also found these exact runtime subdirectories under `~/.cache/agent-tmp/`:

- `arkham-89-art-menu/gathering-assets-relocated-v1`
- `arkham-55-assets/party-assets-v1`
- `arkham-124-jacqueline-assets`
- `arkham-124-native/gathering-relocated`
- `arkham-73-assets/jacqueline-assets-v2`
- `arkham-73-assets/party-assets-v2`
- `arkham-73-assets/party-relocated-v2`
- `arkham-73-assets/party-partial-v2`
- `arkham-73-assets/tony-assets-v5`
- `arkham-77-assets/jacqueline-assets-v3`

Nine complete cached collections matched their manifests without extra files; the partial fixture lacks `tokens/clue.png`. All declared payload hashes occur in v7 except the same historical reaction marker. **Do not remove enclosing cache directories**: they also contain saves, logs and retained verification evidence. The ignored repository-local `local/asset-verification/{prepared,changed,missing}` fixtures are evidence, excluded from the selected runtime cleanup. Other verification manifests may describe source snapshots rather than runtime collections. This bounded inventory is not an exhaustive whole-home census.

Joel selected one active runtime installation plus one dated Documents backup containing the current collection and all specifically identified retired runtime copies. The cleanup implementation must refresh this inventory, preserve original relative-path mappings, make the backup, restore it into a fresh temporary destination, compare every backed-up filename/length/hash, and verify the restored current collection with the existing application checker **before deletion**. With the app closed, establish the stable installation and actual saved/default selection, check ordinary launch, then remove only accounted runtime copies. Failed backup, unexpected files or identities, concurrent changes, failed activation or failed verification stop deletion. Preserve the previous usable state on failure; a previously complete collection may still be incompatible with a newer app.

The source resource archive, Blender authoring files, saves, settings and retained research/log evidence are excluded. Documents and the runtime folders were on the same filesystem with approximately 223 GB free: sufficient for these copies, but a Documents backup protects against cleanup mistakes rather than disk failure. No Documents backup, runtime move, setting change or deletion has yet been performed.

## Public distribution has a separate source constraint

The newest official Asmodee/FFG community-use policy found was the [2024-08-20 PDF](https://images-cdn.fantasyflightgames.com/filer_public/51/c8/51c8035b-240e-4a12-8ab6-a7b34b97c819/ip_policy_2024_aug20.pdf). It covers FFG, is not a license, and says individual permission requests are not answered. Guidelines 5–6 expressly object to downloadable card packs/decks, online game versions and apps using its IP.

The strongest competing reading is guideline 1's welcome for freely shared fan creations. The later exclusions still apply. Worked distinction: an original alternate-art card can fit the permitted examples, subject to the other conditions; a free ZIP of official cards for this application matches the disfavored examples. Splitting the ZIP does not change that interpretation.

This states publisher policy, not a legal adjudication. No project-specific permission was established. The live legal-information page exposed no policy body: this is the latest policy **found**, not proven latest. Recommendation: do not describe a public runtime pack as cleared by this policy. Local installation design can proceed independently.

The following file-specific evidence also matters:

| Runtime material | Evidence and remaining limitation |
| --- | --- |
| Card and game-token images | The recipe records exact sources and hashes. Those identities establish selected bytes, not a redistribution license. The project MIT license covers original code rather than these materials; see the [mission](../foundation/mission.md). |
| `fonts/arkham-symbols.ttf` | The recipe extracts it from an external Unity bundle and records the transformation. This investigation found no redistribution license attached to that recipe entry. Extraction is not evidence of permission. |
| `models/chaos-pouch.glb` | The recipe records an owner-supplied Blender-authored model with embedded textures. A public package still needs an explicit grant covering the exported model and embedded inputs; personal ownership/provenance and a public license are distinct facts. Editable source files need not accompany a runtime package unless an applicable license requires them. |
| Lorc icons | Both [Chalice drops](https://game-icons.net/1x1/lorc/chalice-drops.html) and [Swap bag](https://game-icons.net/1x1/lorc/swap-bag.html) are marked CC BY 3.0 by their source. Preserve the existing title, author, source, license and modification statements in a human-readable package notice. [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/) permits redistribution subject to its terms; this license does not clear neighboring game assets. |

Further storage research cannot settle the missing distribution authority. The bounded provisional design is a local importer whose package can come from any separately accepted source, with publication deferred for materials lacking an established basis. This is not a claim that user-supplied files resolve every legal question about the application. No outreach was made.

## Hosting comparison

These are technical capabilities, conditional on having a basis to distribute the actual contents. Prices and terms were checked on 2026-11-08 and can change.

| Option | Official limits and costs | Fit for this project |
| --- | --- | --- |
| GitHub Release assets | A release may have up to 1,000 attachments, each **under 2 GiB**; the release documentation states no total release-size or bandwidth quota. [Release documentation](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases). | Best simple candidate for a project-related, distributable pack of the current size. Attach a versioned ZIP and manifest to a release. Assets are separate from Git history and ordinary clones, but remain hosted under the GitHub repository's Releases surface. |
| itch.io project downloads | Creating pages and uploading content is free. The creator FAQ states individual-file limits and a usual maximum of ten files per page, with increases by request; it does not specify a numerical browser-upload ceiling. [Creator FAQ](https://itch.io/docs/creators/faq). Butler's documented backend ceiling is **30 GB uncompressed per build**, and it supports incremental uploads/patches. [Butler documentation](https://itch.io/docs/butler/pushing.html). | A credible alternative for a public game/asset project, especially if a storefront and the itch app are useful. It adds a second publication surface. The documented ceiling is not a guarantee of unlimited hosting or acceptance. The publisher must have distribution rights. [Terms](https://itch.io/docs/legal/terms#4-publisher-content). |
| Cloudflare R2 Standard | Monthly free allowance: **10 GB-month storage, 1 million Class A operations, 10 million Class B operations**; direct egress has no transfer charge. Above that, Standard storage is $0.015/GB-month, writes $4.50/million and reads $0.36/million, with billing-unit rounding. [Pricing](https://developers.cloudflare.com/r2/pricing/). | Suitable object storage if larger scale eventually warrants operating it. It requires subscription checkout and usage billing. [Getting started](https://developers.cloudflare.com/r2/get-started/). The free `r2.dev` endpoint is rate-limited and designated for development; production public access uses a custom-domain path. [Public buckets](https://developers.cloudflare.com/r2/buckets/public-buckets/). It is a free allowance on a metered service, not a guaranteed zero-cost host. |

GitHub explicitly recommends Releases for distributing large binaries instead of tracking them in Git. This is different from both normal Git's 100 MiB file ceiling and Git LFS storage/bandwidth accounting. [Large-file guidance](https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-large-files-on-github). Its [acceptable-use policy](https://docs.github.com/en/site-policy/acceptable-use-policies/github-acceptable-use-policies#9-excessive-bandwidth-use) still allows throttling or suspension for significantly excessive bandwidth and prohibits infringement. “No published release bandwidth quota” should not be presented as an unconditional CDN guarantee.

No broader file-host survey is needed for the present decision: the strongest simple candidate fits the declared current payload well below its per-file ceiling; R2 supplies a known scaling alternative. Generic personal-drive sharing would add a different account/download workflow without resolving distribution rights or the app's installation confusion.

## One archive or several

**Selected first package: one complete versioned ZIP for the currently supported Gathering content.** Package only the selected runtime files, manifest and attribution notices. Keep the original resource archive, Blender project, previews, saves, settings and personal paths outside the package. The measured research ZIP is 159.90 MiB; a final package including notices must be measured and identified separately before publication.

This gives a player one artifact to obtain and one completeness check, and avoids assembling incompatible investigator/encounter/object revisions. The cost is downloading unchanged files when a small asset changes. At the current size, that cost should be compared with the user burden and implementation complexity of a multi-pack installer rather than assumed unacceptable.

If materially larger supported content or measured update costs justify multiple packages later, split by independently needed playable content and/or stable shared material, not automatically by file type. A small release manifest can bind the exact required package identities, hashes and dependencies while the app assembles one compatible installation. A scenario needing three categories still needs all three category ZIPs. Multipart fragments created solely to bypass a host's file-size ceiling are a different design from independently usable content packs.

Example: changing the 20.38 MiB pouch alone would require reacquiring the full first package under the simple proposal. A separate object pack would reduce that transfer but introduces pack-version compatibility, additional missing-file cases and atomic multi-pack activation. Keep this as an explicit tradeoff; no package manager or delta-update system is proposed merely because the collection may grow.

## Archive import and installed storage

An archive is a transport artifact; the app can manage a normal extracted collection for play. Players need not browse its internal layout. The competing minimum change is a clearly named directory plus improved README and selection UI, with manual extraction. That is cheaper to implement but leaves the exact folder-selection burden reported by the owner. Joel selected app-managed import after this comparison. Loading from ZIP on every launch would instead require changing the existing file-based asset loader; the approximately 2% storage saving supplies little reason for that additional mechanism on the current collection.

The selected first path is **Import artwork ZIP → validate and stage → activate complete collection → play offline**. Browser download plus local import keeps installation independent of a particular hosting service. In-app fetching adds transfer progress, cancellation, interrupted-download handling, HTTP failures and host availability to the scope; it is not required for the selected local-import flow.

Use the per-user application-data area for the installed collection, not a cache required for offline play. XDG defines `$XDG_DATA_HOME` with fallback `~/.local/share`, `$XDG_CONFIG_HOME` with fallback `~/.config`, and `$XDG_CACHE_HOME` for non-essential cached files. An original selected ZIP belongs to the user; deleting it needs a visible retention policy. Installer-owned temporary downloads can be treated as disposable only after successful installation under an accepted policy. [XDG specification 0.8](https://specifications.freedesktop.org/basedir/latest/).

Proposed acceptance properties for a bounded importer, derived from the existing preservation/admission contract:

- Bind the selected package to the app's expected recipe or trusted release manifest: supported identity/version, exact expected paths, lengths and SHA-256 values. A checksum stored solely inside the same untrusted archive cannot establish publisher authenticity. A hash verifies bytes; it does not establish a license, semantic completeness or successful rendering.
- Extract into a fresh staging directory. Reject absolute/traversing paths, duplicate normalized destinations, symlinks and special files; enforce actual streamed file-count and byte limits. Never execute archive contents. The Rust ZIP API documents unsafe member names, `enclosed_name`, symlink detection, and compressed versus uncompressed sizes; those primitives still require a project-specific policy. [Rust ZIP documentation](https://docs.rs/zip/latest/zip/read/struct.ZipFile.html).
- Allow for the old installation, candidate extraction and any retained compressed download when checking free space. Use actual bounded streamed output, not only the ZIP's declared sizes, to stop expansion beyond the accepted limits.
- Preserve the active complete collection and saves on corruption, cancellation, out-of-space, incompatible content, extraction failure or interrupted activation. Change the active selection only after the candidate satisfies the relevant checks; retain the previous installation through successful rendering admission, then remove only that app-managed previous copy under the agreed retention policy.
- Keep metadata/integrity validation separate from the existing image/font/GLB loading and renderer-admission checks. Reuse those checks through the actual play path. An install success message must state what was validated.
- Prove a relocated install works offline with the development resource archive unavailable. Exercise first installation, a changed collection, interrupted update and return through the documented launch path. Such evidence is still separate from owner acceptance of the experience.

These technical acceptance properties are proposals, not a claim that a particular Rust ZIP dependency has been selected, integrated or validated.

## Existing implementation seams and evidence limits

The current application compiles the expected recipe into its asset checks. The manifest must match the expected collection identity, roles, relative paths, sizes, hashes and applicable attribution; v1–v6 collections can be diagnosed but cannot admit current v7 play. Merely adding the pouch file to an older folder does not update its manifest. See [asset requirements and preparation](../../src/assets.rs). The preparation path already refuses differing or partial destinations and verifies a fresh stage before publishing it. It currently holds selected output payloads in memory; an importer intended to handle larger packages should stream bounded extraction rather than inherit that memory behavior.

The [runtime loader](../../src/gathering_tabletop/art.rs) consumes ordinary prepared files. [Configuration resolution](../../src/asset_config.rs) gives temporary command-line overrides precedence over saved preferences and then the XDG default, preserving concurrent edits during setting changes. [Startup admission](../../src/tabletop_startup.rs) follows mechanical checks with actual image, font, scene, material and texture loading and renderer readiness. The [settings UI](../../src/tabletop_support/asset_settings.rs) can select a complete folder but has no ZIP importer or downloader.

The highest proposed test boundary is selected runtime package → installed complete collection → normal application admission. Reuse isolated HOME/XDG launch tests in [asset_launch.rs](../../tests/asset_launch.rs), saved/new-attempt refusal tests in [native_artwork.rs](../../tests/native_artwork.rs), and existing production Retry/startup checks. Test extraction/activation failures through that boundary rather than through a separately maintained asset-manager product. The actual native path and one owner walkthrough must still exercise the human consequence; existing unit tests or historic pouch acceptance do not establish the new setup experience.

## Dated interview decisions and spec handoff

Joel selected these independent decisions on 2026-11-08:

1. Public upload of third-party artwork remains conditional on a supported redistribution basis. Continue runtime packaging and local installation design. The owner's source archive originated in a Tabletop Simulator mod; the mod's availability and recorded play are context, not evidence of this project's publication rights.
2. Normal setup uses app-managed import of a downloaded runtime ZIP into the application's own data area, retaining offline play.
3. Start with one complete runtime package for supported Gathering content; whole-package artwork updates are accepted initially. Revisit multiple packs only when measured burden justifies them.
4. Cleanup leaves one active runtime installation plus a dated, restore-verified Documents backup of the current pouch collection and specifically identified retired runtime copies. Protect the excluded development sources and personal/evidence files.
5. Imports occur outside active play. Preserve the previous installation through integrity verification and successful normal loading/rendering admission; then remove the app-managed previous copy. Failed or interrupted imports leave it intact. User download ZIPs and the one-time Documents backup are not automatically deleted.
6. Acceptance exercises the real pouch ZIP in an isolated user-data setup, ordinary README launch, offline play/save/reopen with development sources unavailable, and return use without selecting an artwork folder again. Automated checks include damaged/incompatible archives and interrupted updates preserving existing artwork and saves. One brief owner walkthrough judges import/recovery clarity; hashes alone do not establish completion.

The interview frontier is empty for this bounded design. The intended effect is one identifiable runtime package and a normal return-to-play path that requires no knowledge of staging folders. The initial human setup burden is obtaining one ZIP and selecting it once; successful later launches require no artwork selection. The selected mechanism is local ZIP import, not an in-app downloader, general package manager, direct compressed-asset renderer or a whole-card-catalog release. Host choice remains a conditional publication recommendation, not a prerequisite to local implementation.

The interview handoff consisted of this evidence and the selected contract additions. Joel then invoked `to-spec`, publishing [issue #140](https://github.com/joeloverbeck/arkham-unfolded/issues/140) with the six selected decisions and the single package-to-installed-collection-to-normal-admission boundary. The [dated issue evidence](https://github.com/joeloverbeck/arkham-unfolded/issues/140#issuecomment-6727192215) retains the exact inventory and measurement identities independently of these uncommitted notes. The issue owns implementation scope, allowances, stop conditions, verification and publication; readiness does not activate implementation. Cleanup is a bounded owner-machine operation, not permission for the app to search for or delete arbitrary user folders. No application code, Documents backup, runtime move/deletion, setting change or public artwork upload occurred during the investigation or spec publication. The only runtime ZIP created is the temporary size-measurement artifact.
