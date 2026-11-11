# Review guidance

Check project-owned executable logic, including tools, tests, inline commands and generated code, against the [implementation-language contract](docs/foundation/mission.md#implementation-language). Review thin launchers and claimed exceptions against that contract; a passing test suite does not establish a language exception.

For input and presentation regressions, verify that tests use the production registration path for the behavior under test. Check harness-only plugins and systems against native startup; distinguish substituted infrastructure from functionality the test silently supplies.

For tabletop presentation, controls or rules-text changes, apply the [visual and wording review](docs/agents/native-verification.md#visual-and-wording-review) to the affected player journey, including existing cards and panels it exposes. Report observed defects separately from layout preferences and identify any unverified interaction. The guide owns these judgment criteria; automated checks own mechanical requirements.

Choice legality, equivalence and single-option resolution belong to the rules session, so the table, the agent seat and the headless driver meet the same decisions; presentation does not filter, merge or auto-pick choices. Cost wording in option labels comes from the effective cost, never a phrase written into a card's effect text.

A change that enables another Bevy feature or adds a third-party Bevy plugin needs the explicit decision required by [ADR 0006](docs/adr/0006-keep-bevy-and-keep-its-slice-narrow.md) in its owning issue. Bevy must not enter rules, saves or the headless player driver.
