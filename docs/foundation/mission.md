# Mission and founding boundary

This document records the founding product scope. The repository is at the foundation stage; the intended application is not implemented or verified. The next assessment is a proposal, separate from the founding commitment.

## Decisions established in the interview

Arkham Unfolded is a personal utility and recreation project for Joel: make it possible to enjoy playing Arkham Horror: The Card Game without physically setting up the game, moving its components, or administering its rules. Joel makes the investigators' decisions; the program administers the game. The intended experience retains a readable, animated three-dimensional tabletop with decks, cards, tokens and investigator standees at their locations.

The application is implemented in Rust and runs natively on Ubuntu, in English. The implementation-language contract below also governs its supporting tools. Bevy is a candidate for presentation, not an irrevocably accepted architecture. The application must remain controllable and maintainable as Joel's own program.

The first complete delivered slice is the entire original Night of the Zealot campaign on Standard difficulty: The Gathering, The Midnight Masks and The Devourer Below. Joel controls Tony Morgan, Patrice Hathaway and Jacqueline Fine together, starting from the three named ArkhamDB decks. Completion includes legal campaign branches, resolutions, defeats, failures and their consequences. It does not require a victory, and the program must not force a favorable path.

A smaller demo can investigate a risk or demonstrate progress, but cannot count as delivery of this slice. “Perfectly” expresses the required rules fidelity and absence of manual rules work; feasibility and sufficient evidence for that promise have not yet been established.

The repository is public from creation at `joeloverbeck/arkham-unfolded`. This supersedes the initial request for a private repository. Joel intends to record sessions for YouTube and may attract interest, but this project supports his own use. Public visibility creates no commitment to multiplayer, arbitrary decks, every released card, user support, hosted operation or automatic video publication. Original project code uses the MIT license; it does not relicense game content or other third-party material.

Joel reports working regularly on `castkeep`, `character-relationship` and `narrative-explorer`, which he considers mainly done with occasional fixes. No other unfinished active primary artifact was identified in his response to the founding portfolio question; this does not declare unmentioned repositories parked or complete. This project's founding allocation takes discretionary development time while keeping occasional maintenance independent. The intended trade is to recover access to a favorite tabletop experience that the existing physical and digital options do not provide as desired.

## Implementation language

Rust is required for all project-owned executable logic: application behavior, development and agent tools, tests and fixtures, diagnostics, asset preparation, build and verification helpers, prototypes, and maintained executable snippets and templates. Moving logic into inline commands, generated files, or a different directory does not exempt it. This is an owner-selected maintainability constraint, not a claim that Rust makes behavior correct.

Declarative configuration and data, including Cargo TOML and GitHub Actions YAML, are permitted. Thin shell launchers may select a working directory, set invocation parameters, call existing tools and propagate failure; parsing, validation, transformations, interactive workflows and process supervision belong in Rust. External libraries, system tools and services may use their own implementation languages. Calling an external tool does not permit moving project-owned logic into a non-Rust script or command string.

A project-owned non-Rust implementation requires evidence that the required operation cannot be performed through Rust, including available native interfaces or external tools. Convenience, familiarity, shorter code, migration cost or the absence of a ready-made Rust crate do not establish necessity. Before introducing an exception, record the exact operation, evidence, minimum non-Rust boundary and removal condition in the owning GitHub issue or PR for Joel's explicit acceptance. Without that evidence and acceptance, use Rust or report the unsupported operation.

The existing Python tools and tests are migration debt, not qualifying exceptions. Keep their existing consumers operational during replacement; implement new behavior in Rust and retire each Python path only after its replacement preserves the required behavior and safeguards. The GitHub migration issue owns the bounded work and acceptance; this contract does not activate the implementation or declare migration complete. Generic skill examples must be adapted to this language contract before they produce project artifacts.

## One product and its boundary

The product takes prepared, revision-bound campaign and deck inputs plus player decisions; its rules authority produces game consequences, presentation, saved state and replay records. Joel consumes and accepts the playing experience. An invalid or unsupported preparation must fail visibly before play; missing artwork must remain playable through readable substitutes. Development, release, parking and abandonment belong to this one product.

Deck import, rules execution, tabletop presentation, save/load, recording/replay and agent access are internal parts of that product. They do not authorize a general game engine, card editor, asset-management platform, autonomous rules interpreter, shared service or separate content-production pipeline. ArkhamDB remains the deck editor. The existing resource archive remains externally owned preparation input; the application consumes its own selected, portable presentation assets under the [source contract](sources-and-verification.md#presentation-asset-preparation).

Human acceptance must inspect actual play, readability, pacing and control burden. Headless tests establish only the behavior they exercise. Agent playtesting must reach the same rules and player decisions as the human application; privileged fixtures are separate test facilities. Save/resume and replay must be inspected through their actual consumer paths.

## Proposed initial work and stopping boundary

The founding commitment is this repository bootstrap, three foundation documents and one proposed feasibility issue, with no gameplay implementation or paid services. The active bottleneck is understanding and bounding the rules obligations of this exact campaign and party, including chosen upgrades and weaknesses. There is no accepted estimate for completing the full campaign implementation.

The proposed next work item is a source and feasibility assessment, bounded to two active agent hours and one coherent review surface, with no paid services or gameplay infrastructure. It would reconcile source identities and precedence, expose material interaction families, propose a difficult first execution case, and estimate implementation and verification burden. It would return a recommendation to proceed, narrow, investigate further or park; it would not manufacture execution readiness. This appetite is proposed for final draft acceptance, not already activated.

GitHub Issues and pull requests are the sole active work-control plane. Foundation documents preserve accepted reasoning and contracts; they are not a second queue. Joel owns acceptance and portfolio decisions. Implementation, verification, publication and the playing experience retain their applicable acceptance boundaries.

This document specializes `PURP-1`–`PURP-5`, `WORK-1`–`WORK-3`, `EVID-3`–`EVID-4`, `BOUND-1`–`BOUND-2`, `AGENT-5`, `OPS-1` and `GOV-1`–`GOV-2` of [the repository constitution](../principles/principles-for-repos.md). The constitution remains canonical.
