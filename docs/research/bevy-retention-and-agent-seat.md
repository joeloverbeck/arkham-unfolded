# Bevy retention assessment and agent-seat preparation

Investigated on 2026-11-11 against `470bb9b052600f2b2fa0c675d2c856afa682cf05`, with `netrunner-unfolded` at `dc5bef3` and `unfolded-tabletop` at the pinned `8f709f71d34e7193ca096f71d7810d1a494b43a8`. This is investigation evidence and a completed design-interview record. It is not implementation or acceptance of native play. [ADR 0006](../adr/0006-keep-bevy-and-keep-its-slice-narrow.md) records the engine decision. [#150](https://github.com/joeloverbeck/arkham-unfolded/issues/150) and [#151](https://github.com/joeloverbeck/arkham-unfolded/issues/151) own the two small follow-ups. The agent seat has no issue yet; a later `to-spec` invocation is expected to publish its spec from the second half of this note.

The interview read `AUTH-1`–`AUTH-3`, `PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `BOUND-1`–`BOUND-5`, `INT-1`–`INT-4`, `AGENT-1`–`AGENT-5`, `OPS-1`–`OPS-7`, `PUB-1` and `GOV-1`–`GOV-3` of the constitution at revision 2026-09-30.1. `ADOPT` was not triggered. The rules that shaped the result were `PURP-2`–`PURP-4`, `WORK-1`–`WORK-3`, `WORK-6`, `EVID-2`–`EVID-6`, `EVID-8`, `SEM-1`, `REP-2`, `BOUND-1`, `BOUND-4`, `AGENT-5` and `OPS-1`–`OPS-2`.

## The question

Joel asked whether replacing Bevy with code tailored to the two games would now be better, having seen impressive demonstrations built without a general engine. Those demonstrations were not inspected. A greenfield showpiece shows what it shows under its own conditions (`PURP-2`); it does not carry this product's text-heavy UI, driver history, tests or accepted visual conventions.

**Answer: no, on current evidence.** No replacement prototype was built, so the cost figures below are estimates and the comparison remains an argument from measurements, not a measured comparison.

## Footprint

| Repository | Bevy-bound code |
| --- | --- |
| arkham-unfolded | 38,400 presentation lines: 23,330 production and 15,070 tests holding 369 tests. About 25,300 other lines in `src` import no Bevy. |
| netrunner-unfolded | One file, 1,252 of about 2,400 lines. |
| unfolded-tabletop | About 240 of 475 lines, plus the vendored `wgpu-hal` patch. |

The first count given during the interview (about 27,000 lines) was an undercount: many presentation files receive Bevy through `use super::*`.

The slice in use is narrow: mostly unlit textured meshes, one directional light, no shadow or post-processing configuration, one custom material (233 lines) and one glTF model; flexbox UI with buttons, scroll areas, tab focus and text spans; mesh and UI picking; render-to-texture for printed labels; screenshots. Presentation code is written directly in Bevy's entity style and its tests assert against Bevy's world, so it is not a thin adapter over an engine-neutral model.

Dependency and build measurements, all on the owner's machine:

- 529 locked packages. The Linux tabletop build's normal dependency graph holds 369 crates, 62 of them Bevy's. About 175 would remain if the Bevy crates went but `wgpu`, `winit`, `parley`, `taffy` and `gltf` stayed (computed from `cargo metadata`).
- A one-function edit rebuilds `arkham-tabletop` in about 3.5 s. The debug executable is about 250 MB. Recent CI runs take 5–7 minutes.
- The library, `arkham-driver` and `arkham-assets` build without the `tabletop` feature in about 21 s.

## Recorded pain and where it lives

| Pain | Location | Removed by a tailored renderer? |
| --- | --- | --- |
| Slow uploads, about 10 updates per second ([#55](https://github.com/joeloverbeck/arkham-unfolded/issues/55)) | `wgpu-hal` memory placement, beneath Bevy | No, unless the graphics layer also goes |
| Frame pacing and idle power ([#43](https://github.com/joeloverbeck/arkham-unfolded/issues/43)) | Bevy's window runner | Yes, but a 192-line project module already owns it |
| Build weight | Partly Bevy's 62 crates | Partly |

Joel's only complaint was the severe lag, now repaired, and a remaining minor camera stutter.

A tailored replacement would need a window loop, renderer and shaders, text rasterization, UI layout and focus, picking, asset upload and capture. The estimate of 8,000–12,000 new lines is unmeasured. It would also port the presentation layer and its tests, and require Joel to re-inspect every accepted visual convention in the [presentation contract](../tabletop-presentation.md). That human inspection is the binding cost (`EVID-8`). It would also be a shared rendering and UI stack for two games, which the [mission](../foundation/mission.md) excludes without re-ratification (`OPS-2`).

Its real advantages are direct control of the frame loop, roughly half the dependency graph, no engine API churn and all code in the repository. One observation supports the last point: Bevy 0.20 postdates the investigating model's training, and the probe below first failed to compile on a renamed pointer event type.

## Lock-in

Leaving already means rewriting the presentation layer. From here the cost grows in proportion to presentation code; no cliff was found. Most remaining campaign work is rules code. Three things would make leaving disproportionately harder: Bevy entering rules or saves, more Bevy subsystems, and third-party Bevy plugins. The [frame-pacing research](tabletop-frame-pacing.md) already met a plugin one version behind.

The rules boundary holds but is unchecked: every check phase compiles with `--features tabletop`. #150 adds the Bevy-free build.

Not selected: an engine-neutral layer, restructuring existing presentation code, or porting Netrunner's table as an exit rehearsal. Each spends effort on a departure that may never happen (`WORK-6`).

## Camera stutter

The pacing module caps active frames at 60 per second with a sleep. The display runs at 143.99 Hz, the session is Wayland, and the application runs through Xwayland because only Bevy's `x11` feature is enabled. Sixty does not divide the display rate evenly, so frames are held for alternating two and three refreshes. That is the likely cause and is project code, not Bevy. It has not been measured or seen confirmed on the display. #151 owns the bounded test and its two-launch ceiling.

Separately, the installed NVIDIA driver is 620.70.06, while the upload-memory patch was measured on 615.78.08. The patch's own note asks for a repeated comparison after a driver update. That comparison was not run.

## Decisions and their authority

| Decision | Authority |
| --- | --- |
| Owning the stack is not itself a goal; agents changing any behavior on request satisfies "Joel's own program" | Joel, explicit |
| A replacement candidate means the existing window and graphics crates plus project-owned scene, rendering, UI and picking | Joel, explicit |
| Stay pinned on Bevy 0.20.0; upgrade only for a specific need | Joel, explicit |
| Agents should operate the real table without borrowing the desktop | Joel, explicit |
| Order: #150, #151, the agent-seat spec, then [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89); the seat's first increment is about six agent hours and at most one owner look | Joel, explicit |
| If the seat proves better, share it between Arkham and Netrunner with the least duplication | Joel, explicit |
| Containment rules, reopening conditions, the stutter test, the seat's shape and these records | Delegated to the agent's recommendation after a complete proposal (`AGENT-5`) |

The three side items are all off the path of #89, as were the three preceding merges. Joel selected them with a return to #89 afterwards. None resets the real-use streak for complete Gathering play (`WORK-3`).

## Agent seat

**Agent seat** is the working name for an agent playing at the real 3D table, seeing and choosing only what a player can, without the owner's desktop. It is distinct from the headless player driver, which plays the rules with no table. The term enters the glossary only if the seat is adopted.

### Problem and incumbents

Joel: operating the game "as if they were some remote technician connecting to my computer" is very awkward; agents should play it natively.

- The headless player driver already exercises rules and decisions without Bevy.
- The desktop route operates the real window through a timed approval, the GNOME portal and `xdotool`. The files handling it hold about 3,900 lines.
- The application itself offers only `--screenshot`, `--quit-after-screenshot` and `--initial-view`.

The simpler alternative considered was the unmodified application on a private virtual display (`PURP-4`). It was not exercised: no such tool is installed (`Xvfb`, `weston`, `sway`, `cage`, `gamescope` and `Xephyr` are all absent), and it would leave the agent working from screenshots and pixel clicks, which is the indirectness Joel wants removed. It remains the fallback if the seat fails.

### Feasibility probe

Classification: decision-only seam probe on a toy scene (`EVID-4`). It establishes that the boundary can be crossed on this machine, not that the real table works.

Pinned Bevy's own `examples/app/externally_driven_headless_renderer.rs` was built unchanged as a temporary example against this repository's feature set, then extended with a UI node holding text, click observers on a mesh and on the node, and `PointerInput` messages aimed at the image target. It ran from an agent shell with `DISPLAY` and `WAYLAND_DISPLAY` unset. The temporary file was removed afterwards.

Demonstrated, on both the RTX 2080 Ti and the `llvmpipe` software driver:

- Rendering to an image with no window, and saved captures showing the 3D scene and the UI text.
- A synthesized move, press and release produced `PointerClick` on the mesh and on the UI node through Bevy's picking backends. A click on empty space produced none.

Three conditions were needed, each already present in this repository's own windowless render test:

- A logical `Window` with `PrimaryWindow` must exist. Without it Bevy's picking treats every pointer as outside the viewport.
- The camera needs `IsDefaultUiCamera`, or UI is not drawn to the image.
- A capture needs several further updates before its file exists.

Prior art: Arkham's ignored render test already runs the production `install` with real prepared artwork, a logical window and an image target; Netrunner has an equivalent whole-table capture test.

Not demonstrated: the real table admitted and played windowless, software-driver speed on the real table, and GPU access from a more restricted agent sandbox.

### Existing seams

Read at the investigated revision; a sub-agent's map was spot-checked against the source.

- **Click path.** A target carries `ContextTarget` with a `controls::Source`. `PointerClick` queues `Action::Select`; `handle_actions` opens a contextual menu; a menu row is a button whose `Activate` queues the choice; the choice commits through `continuity::commit_choice_with_draws`, which validates the revision and saves.
- **Identity.** `controls::Source` and card identities are stable. Entities are rebuilt on every dirty render. No component ties a button to its choice id; that mapping lives in observer closures.
- **Busy table.** Choices queued during an announcement or active consequences are dropped silently, and menus hide their rows while presenting.
- **Settling.** `Redraw` is not a settled signal: the breathing cue on available targets requests redraws indefinitely. The real conditions are pending presentation, consequences, trays, sealed tokens, card motion and camera motion; a test helper approximates them.
- **Clock.** Animations read `PresentationTime`, advanced from wall-clock time by the pacing module. Tests already advance it directly.
- **Admission.** Rendering health counts completed GPU work only for a window with a swapchain texture and requires a focused, visible window. With no real window it would never admit play, and its watchdog exits the process after 15 s without an update while armed.
- **Startup.** Incomplete artwork opens `zenity` dialogs. Capture uses the primary window and waits 90 admitted frames, not for the table to settle.

### Design selected for the spec

1. **One product, one path.** The seat is a mode of `arkham-tabletop`, entered by an explicit flag. It uses the same `install`, rules session, save path and artwork admission as native play. It is agent access inside the one product, not a second pipeline (`BOUND-1`), and should be removable by deleting its module and a short list of integration points (`BOUND-4`).
2. **Enumerated differences from native startup, and no others:** no window event loop; a logical primary window; the main camera renders to an image and is the default UI camera; pointer input names that image; the seat advances the presentation clock in fixed steps; seat-specific rendering admission; startup failures are reported as text, never dialogs.
3. **Admission.** The seat requires decoded artwork, the registered font, ready pipelines and completed GPU work for its image. It makes no claim about display rate. Native rendering health is unchanged and is not weakened to fit the seat.
4. **Input boundary.** The seat writes the same `PointerInput` messages Bevy's own window-event conversion writes, plus keyboard messages and the logical window's cursor position. Everything after that is production code. The seat never triggers observers, pushes actions or calls handlers.
5. **Naming.** Table targets are named by their `Source`; choice-bearing controls by their choice id, which needs one new component on those controls. Lookup is exact. An absent or ambiguous name is an explicit failure, never a nearest match (`SEM-1`).
6. **Acting.** To activate a target the seat projects it to the screen, moves the pointer, lets one update establish hover, presses and releases. It then confirms that the target was what the pointer hit, and otherwise reports what was hit instead. A covered target therefore fails honestly. Selecting a card and choosing from its menu are two activations, as for a player.
7. **Settling.** One production predicate states that nothing is pending. The seat steps until it holds or a step limit is reached, and reports which condition remained. Because the table drops input while busy, activation is refused with a reason unless the table is settled.
8. **Observation.** The same player-visible `Observation` the headless player driver prints, plus what the table shows: targets with their names, labels, screen positions and whether they currently offer choices; open menu rows and panel buttons; modal state; and whether the table is settled. Nothing hidden is added, and privileged fixtures stay a separate test facility.
9. **Capture.** A picture of the seat's image on request. The response is sent only after the file exists.
10. **Protocol.** JSON lines in the headless player driver's envelope, read from standard input or a script file, one response per request: `observe`, `activate`, `key`, `pointer`, `settle`, `capture`, `quit`. It needs only a shell, so Codex and Claude sessions use it alike.

### Test seam

The seat's protocol is the single seam. Two levels use it:

- The existing no-GPU table host runs protocol behavior in CI: naming, refusals, busy-table handling, settle limits and stale targets. Prior art: the headless player driver's tests and the pointer-driven button test.
- A GPU-backed run with the real prepared asset collection runs on the owner's machine, gated like the existing render test. CI has no artwork.

### First increment and what it can establish

The first increment is the smallest loop that bears the claim (`EVID-3`): through the seat alone, an agent starts or resumes a real scenario attempt, observes, activates a card and a menu choice by pointer, waits for the table to settle, sees the consequence in the next observation and a capture, and makes a further decision that depends on it. It must also show one planted presentation defect failing honestly, such as a covered target, so that a clean run means something (`EVID-4`).

Bound: about six agent hours and at most one owner look, ending in adopt, narrow or drop. Stop early if Bevy obstructs the seat (ADR 0006's third condition), if it would require weakening native rendering health or artwork admission, or if the surface doubles.

A seat run can establish presentation logic, layout, controls and rendered output on its image. It cannot establish window behavior, display pacing, focus or physical input, which remain with the desktop route and the owner. Joel's inspection of actual play remains the acceptance.

Adoption as a recurring path would additionally need a materially different second use and a return use from repository documents alone (`EVID-3`). Those are outside the first increment.

### Sharing with Netrunner

Build the seat in Arkham first (`EVID-5`, `WORK-6`). Keep the game-neutral mechanics (windowless host, stepping, pointer synthesis, capture and the protocol loop) in one module behind a narrow game-facing interface: observe, resolve a name to a screen position, and report settled. When Netrunner adopts the seat as a second real consumer under its own accepted issue, that module moves to `unfolded-tabletop` at a pinned revision, as the first extraction did. Building it in the shared library first is not selected.

### Open risks

- Seat admission is new decision-bearing code and must fail as honestly as the native guard.
- Automatic administrative choices commit without agent action; observations must make that visible.
- The software driver may be too slow for the real table.
- Shrinking the desktop-route tooling is a later decision and depends on adoption.
