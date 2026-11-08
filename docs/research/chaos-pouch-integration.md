# Chaos pouch integration investigation

Read-only investigation on 2026-11-08 for the owner's `grill-with-docs`
interview. Application baseline: `7978dbefd911fabeb4df4eacd777f9b3e2b15dce`.
These are source observations and candidate seams, not evidence of in-game
appearance. The [current Gathering handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060)
links the accepted interview decisions and owns subsequent work activation,
allowances and acceptance.

## Delivered artwork

The full external `~/Documents/Arkham Unfolded Assets/Chaos Bag/README.md`
was read, together with `verification/roundtrip.json`, and the intended
`previews/01-game-view-sheen.png` was inspected.

- Runtime delivery: `chaos-pouch.glb`, 21,374,036 bytes; SHA-256 independently
  checked as `514b48674232d76a2875a6d0b43f02afca889f3777972ecd2d3c25a9791c4483`.
- The delivery record reports five meshes, six primitives, 208,708 triangles,
  five materials and nine embedded images, with no external file dependencies.
  Those structural counts were not independently re-parsed in this interview.
- The GLB uses Y-up metres and the `ChaosPouch` root. Its origin rests on the
  floor beneath the body; the cords make bounding-box centering inappropriate.
- This is one static open empty pouch. Tokens, labels, interaction, animation,
  lights and cameras are not supplied by the exported object.
- The authored velvet uses `KHR_materials_sheen`, with color factor approximately
  `[0.08, 0.14, 0.15]` and roughness `0.75`. The delivery explicitly requires
  reading these factors and rendering sheen. Its no-sheen comparison does not
  establish how Bevy will look.

The delivery and existing [source contract](../foundation/sources-and-verification.md#presentation-asset-preparation)
agree on custody: preserve the authoring folder; select the GLB into a fresh,
self-contained prepared collection. The embedded textures do not require loose
runtime copies. The Blender source, original textures, previews, reference and
authoring/verification files remain in the external authoring folder. Runtime
artwork stays outside Git; recipe identities, hashes and application shader or
loader code belong with the project, subject to its language contract.

## Current application seams

| Concern | Observed source at the baseline |
| --- | --- |
| Placeholder | `src/tabletop_pieces.rs:106-151` builds a brown cylinder; the label and curse pool are separate. `src/tabletop_layout.rs:175-176` supplies its position. |
| Selection | `src/gathering_tabletop/play.rs:492-520` attaches `Source::Bag`; `scene.rs:17-38` binds root click handling. Imported mesh clicks must reach that logical owner. |
| Availability | `src/tabletop_feedback.rs:184-215` creates a separate rim. An unscaled logical parent with a transformed visual child is a candidate for preserving cues and labels. |
| Reveals | `src/gathering_tabletop/reveal.rs:40` and `controls.rs:314,421-445` use the bag's existing choices. The artwork does not own their semantics. |
| Preparation | `src/assets.rs:343-469` verifies identities and prepares fresh collections while refusing differing destinations. `resources/gathering-asset-selection.json` is currently v6. |
| Asset types | `src/assets.rs:136-142`, `src/gathering_tabletop/art.rs:39-63` and `src/tabletop_startup.rs:69-83` classify non-font roles as images. A GLB needs an explicit model path and retained handles. |
| Admission | `src/tabletop_health.rs:335-359` recognizes decoded images/fonts only. Model dependencies and renderer readiness need their own coverage. |

The pinned shared dependency is `unfolded-tabletop` revision
`8f709f71d34e7193ca096f71d7810d1a494b43a8`. Its `src/loading.rs:23-60`
already checks generic recursive loading state; the image/font restriction is
in Arkham's callback. This investigation found no demonstrated need to change
the shared producer.

The [presentation contract](../tabletop-presentation.md#test-area-planned-reveals-and-results)
keeps the physical bag at the Test area's upper right. Its separate screen-fixed
reveal button and decorative pouch are distinct existing presentation elements.

## Sheen feasibility and language constraint

`Cargo.toml:26` pins Bevy 0.20.0. Both its installed glTF source and
[versioned documentation](https://docs.rs/bevy_gltf/0.20.0/bevy_gltf/#supported-khr-extensions)
list `KHR_materials_sheen` as unsupported. Loading this GLB alone cannot meet
the delivered material requirement.

Bevy's glTF extension hooks and
[`MaterialExtension`](https://docs.rs/bevy_pbr/0.20.0/bevy_pbr/trait.MaterialExtension.html)
provide a candidate narrow route: read the factors in Rust and attach a custom
material shader to the velvet. Actual light-dependent appearance, material
interaction and performance remain untested.

At investigation time, the [implementation-language contract](../foundation/mission.md#implementation-language)
said all project-owned executable logic required Rust; no project-owned WGSL
files were found. The interview therefore initially proposed a narrow WGSL
exception. Joel clarified on 2026-11-08 that the constraint was intended for
general-purpose programming languages, not GPU languages. The mission now owns
that clarified boundary; this material does not need a special language waiver.
The owner selected the recommended Rust loading plus material-shader route,
whose rendering feasibility remains untested. Rust-only GPU programming was
not shown impossible:
[Rust-GPU releases](https://github.com/rust-gpu/rust-gpu/releases) document
SPIR-V and a WGSL target, but interoperability with this Bevy material path was
not tested. This investigation did not adopt a Rust-GPU toolchain.

At the interview's close, no application code, external artwork, configuration
or personal saves had changed. No app launch, GPU measurement or owner visual
acceptance had occurred.

## Implementation on 2026-11-08

The owner activated implementation after the interview. The v7 recipe selects
148 external files, adding the unchanged GLB at `models/chaos-pouch.glb`.
`arkham-assets prepare-gathering` accepts its source through `--chaos-pouch`;
v1–v6 collections remain recognizable but cannot admit play. A fresh complete
collection was prepared at
`~/.local/share/arkham-unfolded/gathering-assets-chaos-pouch-staging`.
The previous installation, saved folder preference, authoring folder and personal
saves are preserved. Review launches require the explicit staging override;
the existing saved choice still points to the v6 issue-137 collection.

`tabletop_chaos_pouch` handles this GLB's velvet material, preserving Bevy's
conversion of its other PBR properties. The WESL shader uses the imported linear
sheen color and roughness, the Charlie distribution/visibility and view-only
albedo scaling described by the
[Khronos sheen specification](https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_sheen).
Its lighting support is bounded to the table's directional and ambient lighting;
this is not a general implementation of textured sheen or environment lighting.
The factor parser refuses unsupported sheen fields. The required scene, custom
material and nine embedded textures participate in startup admission.

The delivered PNG sampler requests trilinear minification. Bevy decodes these
PNGs without mip levels, which produced visible normal-map aliasing in the first
render. Rust now builds filtered levels in memory before the first GPU upload,
averaging color in linear space. It preserves the original GLB and base-level
bytes. A render regression compares the first material upload with a later
rebind, preventing the initial unfiltered-view ordering defect from returning.

The scene is a child of the existing logical bag, scaled from metres by five
and anchored by the authored floor origin at the mat surface. Labels and
selection remain on their existing owners. Mesh-descendant clicks reach the
bag; Inspect-only interaction still opens inspection directly. The teal
availability halo follows the pouch footprint and the same gameplay choices as
its menu. Joel explicitly reconfirmed the breathing cue for choices beyond
Inspect during implementation. Inspection alone keeps it off; Reduced motion
uses the existing steady cue.

Automated evidence is retained under ignored
`local/verification/7978dbefd911fabeb4df4eacd777f9b3e2b15dce/chaos-pouch-*`.
The offscreen GPU check uses the production Gathering scene/UI and actual GLB,
with a clearly marked rules fixture. It compares sheen enabled/disabled,
checks material admission and missing-material refusal, all nine filtered
textures, six primitives/208,708 triangles, mat contact and the available cue's
low/high opacity. These captures establish rendered output, not physical input,
foreground frame rate or owner appearance acceptance.

The same real-asset check passed from a relocated collection in an offline,
read-only mount namespace, with the authoring folder, original resource archive
and installed artwork directories hidden from the process. Synthetic collection
checks cover absent/changed model bytes, restored-file recovery and refusal to
overwrite earlier collections. The current GitHub handoff owns final check
results, the owner review route and the remaining acceptance boundary.

The first owner launch exposed a startup timeout on all nine embedded textures.
The earlier check requested their handles only after the scene was loaded;
native startup requests the full admission set before loading. With that order,
concurrent labeled-subasset loads replace decoded images. The preparation system
handled `Added` events but ignored `Modified`, leaving replacements without
filtered levels. A model-only probe found matching handles and loaded dependency
states, but only one texture level on all nine images. The production admission
test reproduced the owner's exact nine-file timeout twice. Preparing both added
and replaced images cleared that regression without relaxing the readiness gate.
The real-asset test now calls `Recovery::install` before loading and checks the
entire production admission set, including missing-material refusal and recovery.
The owner screenshot and unchanged review-save hash are retained with the failed
baseline; successful native retry and visual acceptance remain separate checks.
