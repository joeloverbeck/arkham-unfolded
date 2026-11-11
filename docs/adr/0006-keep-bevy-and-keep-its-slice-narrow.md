---
status: accepted
---

# Keep Bevy for presentation and keep its slice narrow

Selected on 2026-11-11 through a design interview: keep Bevy 0.20.0 for the
Gathering table instead of replacing it with project-owned windowing,
rendering, UI and picking. Joel asked whether a tailored replacement would now
serve better. The investigation found nothing that replacement would remove at
acceptable cost. The severe lag came from GPU upload memory beneath Bevy and
would survive any renderer built on the same graphics layer. Frame pacing is
already project code. Replacement would reopen every accepted visual convention
for owner inspection and leave the path to complete Gathering play.

Bevy remains a replaceable candidate, as the [mission](../foundation/mission.md)
states, until native Gathering play is accepted. The candidate it was compared
with is the window and graphics crates Bevy already uses plus project-owned
scene, rendering, UI and picking; a raw graphics API would also own every
driver quirk. A program whose every behavior agents can change on request
satisfies "Joel's own program" without removing the engine.

Keep leaving affordable instead of preparing to leave:

- Rules, saves and the headless player driver stay free of Bevy, and a
  Bevy-free build belongs in the checks.
- Adding another Bevy subsystem or a third-party Bevy plugin is an explicit
  decision in the owning issue, because each one widens what a departure must
  replace and can block an upgrade.
- Stay pinned. Upgrade only as bounded work for a specific needed fix or
  feature.
- Introduce no engine-neutral layer, and do not restructure existing
  presentation code for portability.

Reconsider replacement only when one of these occurs:

1. A defect traced to Bevy itself, not the graphics layer, the driver or project
   code, blocks an accepted requirement and survives one bounded repair attempt.
2. A needed fix exists only in a newer Bevy, and the upgrade breaks accepted
   visuals or exceeds its bounded effort.
3. Bevy obstructs agents operating the real table without the owner's desktop.
4. A third game joins the two that share presentation code.

Joel explicitly selected the pinned version, the comparison candidate and the
reading of "own program". After the complete recommendation he delegated the
containment rules and reopening conditions to the agent's judgement. The
[assessment](../research/bevy-retention-and-agent-seat.md) retains the
measurements, their limits and the interview record.
