# Mission and founding boundary

This document records the founding product scope and subsequent accepted boundary changes. The initial assessment below is historical; [README](../../README.md) records implemented status, and GitHub owns active work. The full intended campaign remains incomplete.

## Decisions established in the interview

Arkham Unfolded is a personal utility and recreation project for Joel: make it possible to enjoy playing Arkham Horror: The Card Game without physically setting up the game, moving its components, or administering its rules. Joel makes the investigators' decisions; the program administers the game. The intended experience retains a readable, animated three-dimensional tabletop with decks, cards, tokens and investigator standees at their locations.

The application is implemented in Rust and runs natively on Ubuntu, in English. The implementation-language contract below also governs its supporting tools. Bevy is a candidate for presentation, not an irrevocably accepted architecture. The application must remain controllable and maintainable as Joel's own program.

The first complete delivered slice is the entire original Night of the Zealot campaign on Standard difficulty: The Gathering, The Midnight Masks and The Devourer Below. Joel controls Tony Morgan, Patrice Hathaway and Jacqueline Fine together, starting from the three named ArkhamDB decks. Completion includes legal campaign branches, resolutions, defeats, failures and their consequences. It does not require a victory, and the program must not force a favorable path.

A smaller demo can investigate a risk or demonstrate progress, but cannot count as delivery of this slice. “Perfectly” expresses the required rules fidelity and absence of manual rules work; feasibility and sufficient evidence for that promise have not yet been established.

The repository is public from creation at `joeloverbeck/arkham-unfolded`. This supersedes the initial request for a private repository. Joel intends to record sessions for YouTube and may attract interest, but this project supports his own use. Public visibility creates no commitment to multiplayer, arbitrary decks, every released card, user support, hosted operation or automatic video publication. Original project code uses the MIT license; it does not relicense game content or other third-party material.

On 2026-11-09, Joel clarified the intended route for interested users: build the application themselves from the public source, download a separately hosted prepared runtime artwork ZIP, and import it through the application. The intended audience can follow the build instructions. Artwork publication remains governed by the [source contract](sources-and-verification.md#presentation-asset-preparation); this clarification selects the distribution experience, not a host or a redistribution basis.

Joel reports working regularly on `castkeep`, `character-relationship` and `narrative-explorer`, which he considers mainly done with occasional fixes. No other unfinished active primary artifact was identified in his response to the founding portfolio question; this does not declare unmentioned repositories parked or complete. This project's founding allocation takes discretionary development time while keeping occasional maintenance independent. The intended trade is to recover access to a favorite tabletop experience that the existing physical and digital options do not provide as desired.

## Implementation language

Rust is required for project-owned general-purpose programming: application behavior, development and agent tools, tests and fixtures, diagnostics, asset preparation, build and verification helpers, prototypes, and maintained executable snippets and templates. Moving that logic into inline commands, generated files, or a different directory does not exempt it. This is an owner-selected maintainability constraint, not a claim that Rust makes behavior correct.

On 2026-11-08, Joel clarified that this constraint concerns general-purpose languages such as Java and TypeScript, not GPU shader languages. GPU shader programs may use languages such as WGSL without a per-use language exception or proof that Rust cannot implement them. Host-side application logic, loading, asset preparation and tooling remain governed by the Rust requirement. Shader source is project code, distinct from external artwork payloads.

Declarative configuration and data, including Cargo TOML and GitHub Actions YAML, are permitted. Thin shell launchers may select a working directory, set invocation parameters, call existing tools and propagate failure; parsing, validation, transformations, interactive workflows and process supervision belong in Rust. External libraries, system tools and services may use their own implementation languages. Calling an external tool does not permit moving project-owned logic into a non-Rust script or command string.

A project-owned implementation in another general-purpose language requires evidence that the required operation cannot be performed through Rust, including available native interfaces or external tools. Convenience, familiarity, shorter code, migration cost or the absence of a ready-made Rust crate do not establish necessity. Before introducing an exception, record the exact operation, evidence, minimum non-Rust boundary and removal condition in the owning GitHub issue or PR for Joel's explicit acceptance. Without that evidence and acceptance, use Rust or report the unsupported operation.

The existing Python tools and tests are migration debt, not qualifying exceptions. Keep their existing consumers operational during replacement; implement new behavior in Rust and retire each Python path only after its replacement preserves the required behavior and safeguards. The GitHub migration issue owns the bounded work and acceptance; this contract does not activate the implementation or declare migration complete. Generic skill examples must be adapted to this language contract before they produce project artifacts.

## One product and its boundary

The product takes prepared, revision-bound campaign and deck inputs plus player decisions; its rules authority produces game consequences, presentation, saved state and action history. Joel consumes and accepts the playing experience. An invalid or unsupported preparation must fail visibly before play. On 2026-11-07, Joel selected [complete artwork as a prerequisite for 3D play](../adr/0004-require-artwork-for-3d-play.md), replacing the earlier readable-substitute requirement. The headless player driver and engineering checks remain independent of artwork. Development, release, parking and abandonment belong to this one product.

Deck import, rules execution, tabletop presentation, save/load and agent access are internal parts of that product. They do not authorize a general game engine, card editor, asset-management platform, autonomous rules interpreter, shared service or separate content-production pipeline. ArkhamDB remains the deck editor. The existing resource archive remains externally owned preparation input; the application consumes its own selected, portable presentation assets under the [source contract](sources-and-verification.md#presentation-asset-preparation).

Human acceptance must inspect actual play, readability, pacing and control burden. Headless tests establish only the behavior they exercise. Agent playtesting must reach the same rules and player decisions as the human application; privileged fixtures are separate test facilities. Save/resume must be inspected through its actual consumer path.

Joel accepted [complete replay retirement](../adr/0003-retire-replay.md) on 2026-11-04 because he will not use it. The replacement may redesign persistence and treats every pre-removal save and recording as unsupported. The [implementation report](../research/replay-removal.md#implementation-result) records the local removal and its evidence separately from owner acceptance.

Joel selected [experiment retirement and scenario selection](../adr/0005-retire-experiments-and-select-scenarios.md) on 2026-11-07. The maintained entry offers only original The Gathering on Standard with the three selected starting decks, marked in development. Browsing selection needs no game session or artwork. Retired experiment evidence remains historical; this choice does not complete the campaign or accept #89's full native playing experience.

Joel selected [agent partners](../adr/0007-agent-partners-join-the-running-table.md) on 2026-11-13 as an occasional way to play. For a session, he may hand chosen investigators to agents, who play them as players at the table while he concentrates on his own. Playing all three himself remains the default, and the delivered slice is unchanged. Partners receive only the decisions and information the [play contract](play-contract.md#agent-partners) gives the player of their investigators, and the program still administers the game. Play stays local to one machine and one window. This creates no commitment to network multiplayer, other human players or hosted agents, and a partner's judgement is never a rules authority.

## Code shared with the sibling games

Arkham consumes `unfolded-tabletop`, the public library that holds mechanisms first written here, at one immutable revision pinned in `Cargo.toml` and `Cargo.lock`. Rules, cards, saves and presentation policy stay in this repository, in adapters over the shared code, and shared code imports no game's domain. A change to a shared mechanism reaches Arkham only through a new pin, with Arkham's own checks and acceptance.

`EVID-6` asks for three materially different real cases and a holdout before cross-product adoption. Sharing with fewer rests on two scoped waivers under `ADOPT-2`. Neither edits the constitution, and neither establishes general library fitness or acceptance of an untested migration.

- **2026-11-05, Arkham and Netrunner.** Recorded in netrunner-unfolded's [shared code and development](https://github.com/joeloverbeck/netrunner-unfolded/blob/main/docs/foundation/05-shared-code-and-development.md#accepted-evid-6-exception). It covers code used by both games and, for any surface, ends when a third consumer appears.
- **2026-11-11, a third consumer.** Joel accepted the waiver below so that `mudsong`, a private repository of his, can share mechanisms with these games. Its authoritative text is that repository's `docs/foundation/04-engineering-and-shared-code.md`. This is its record here, carried by the first extraction made under it: the seeded generator and shuffle that setup and play draw from.

| Field | Accepted boundary |
| --- | --- |
| Exact rule | `EVID-6`: three materially different real cases and an independent holdout before cross-product adoption |
| Scope | Concrete mechanisms, not specific to any game, owned in `unfolded-tabletop` and really used by `mudsong` and by at least one of `arkham-unfolded` and `netrunner-unfolded` |
| Reason | These three are the only real products. Duplicating mechanisms or inventing nominal consumers would defeat the intended reuse |
| Accepted risk | Two consumers can miss an incompatible assumption; a later case may force an interface change or a reversal |
| Required evidence | A real path in each consuming application, each repository's own checks, game policy preserved in adapters, and a coordinated rollback |
| Appetite | Two active hours per extraction attempt, then reassess. No extension into a framework project |
| Ends | For a given mechanism: a consumer outside these three games, publication to crates.io or a promise of general reuse, or loss of its second real consumer. There is no milestone or date expiry. After that, further sharing needs the normal evidence or a new waiver |

## Founding proposal and stopping boundary (historical)

The founding commitment is this repository bootstrap, three foundation documents and one proposed feasibility issue, with no gameplay implementation or paid services. The active bottleneck is understanding and bounding the rules obligations of this exact campaign and party, including chosen upgrades and weaknesses. There is no accepted estimate for completing the full campaign implementation.

The proposed next work item is a source and feasibility assessment, bounded to two active agent hours and one coherent review surface, with no paid services or gameplay infrastructure. It would reconcile source identities and precedence, expose material interaction families, propose a difficult first execution case, and estimate implementation and verification burden. It would return a recommendation to proceed, narrow, investigate further or park; it would not manufacture execution readiness. This appetite is proposed for final draft acceptance, not already activated.

GitHub Issues and pull requests are the sole active work-control plane. Foundation documents preserve accepted reasoning and contracts; they are not a second queue. Joel owns acceptance and portfolio decisions. Implementation, verification, publication and the playing experience retain their applicable acceptance boundaries.

This document specializes `PURP-1`–`PURP-5`, `WORK-1`–`WORK-3`, `EVID-3`–`EVID-4`, `BOUND-1`–`BOUND-2`, `AGENT-5`, `OPS-1` and `GOV-1`–`GOV-2` of [the repository constitution](../principles/principles-for-repos.md). The constitution remains canonical.
