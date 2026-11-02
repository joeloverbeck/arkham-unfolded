# Gathering candidate

This is the local implementation candidate for original Gathering
under [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89). It uses the
actual three selected starting lists, rather than the prepared Study experiment's
scripted hands. **Completing mulligans now begins ordinary scenario play.** The
first full round and all following mythos draws have driver-level evidence.
The earlier native pointer route through setup, one action and replay passed with
readable substitutes. The artwork and contextual-menu continuation is a new
candidate awaiting a bounded native check. Physical keyboard checks, a complete
native playthrough and owner acceptance remain outstanding.

The [research return](../research/issue-89-rules-investigation.md) supplies the
source-derived behavior. This implementation pins that research baseline and
taboo 10. On 2026-11-02 the accepted ArkhamDB taboo source was refreshed and matched
the retained bytes exactly (`d63d719bd025040fd74281c123cc692ce521f305e78d5588e7c83eed31801673`):
latest ID 10, effective 2026-02-19. The publisher resource page still lists Legacy
FAQ 2.5 and Grimoire 1.1; the existing original-content precedence remains applicable.
This dated check is not a guarantee about a future campaign's latest version.
The original deck revisions and explicit weakness selections are
visible in the observation. Original ArkhamDB snapshots remain unchanged.

## Run and resume

From the repository root:

```sh
cargo run --locked --bin arkham-driver -- \
  --experiment gathering --save /tmp/arkham-gathering-setup.session.json
```

Use a new file path to begin a separate setup. Repeating the command reopens that
file. The JSON driver works offline without artwork or the source archive. This
is a developer-accessible player interface.

Records from previous setup-v1, runtime-v2 and runtime-v3 checkpoints use the preserved readers described
under [Record compatibility](#record-compatibility), rather than this build.

The native table uses the same session and save file:

```sh
cargo run --locked --features tabletop --bin arkham-tabletop -- \
  --experiment gathering --save /tmp/arkham-gathering-setup.session.json \
  --art-root /home/joeloverbeck/.cache/agent-tmp/arkham-89-art-menu/gathering-assets-v1
```

Click an investigator card to open its contextual menu and choose the lead.
Click each opening-hand card to select or undo its mulligan; confirm the hand
using the table-wide control. During play, cards, piles, locations and revealed
tokens offer their legal actions through the same shared menu renderer as Party.
Clicking a component with no available action opens inspection directly; otherwise
choose **Inspect** for enlarged artwork and effective rules. The encounter deck
inspection lists its starting quantities without revealing shuffled order.
Act and Agenda inspection also includes their previously revealed reverse sides.
Replay limits those images and texts to the moment being viewed.

Camera controls use the existing Overview, Scenario and investigator views.
Save, Reopen, History and Replay operate on the full session; replay permits
inspection but no live choices. Menus use Tab/Shift+Tab and Enter/Space; Esc
closes them. Camera keys are suspended while a contextual menu captures input.
Headless tests exercise the actual menu controls and save/replay protections.
The older native retest below does not verify this new artwork/menu candidate.

### Artwork preparation and selection

The README uses the verified collection already prepared on this machine. To
prepare another portable collection from the original local archive, first use
the [existing reveal-icon download](../../README.md#prepare-the-tony-artwork), then:

```sh
cargo run --locked --bin arkham-assets -- prepare-gathering \
  --source "/home/joeloverbeck/Pictures/Arkham Horror LCG Resources" \
  --reveal-icon /tmp/arkham-reveal-icon.png \
  --output "$HOME/.local/share/arkham-unfolded/gathering-assets-v1"
cargo run --locked --bin arkham-assets -- check-gathering \
  "$HOME/.local/share/arkham-unfolded/gathering-assets-v1"
```

The finite recipe contains 140 roles: 94 card identities, 11 reverse/unrevealed
sides, card backs, investigator standees, token/class artwork, reveal icon and
symbol font. Location roles explicitly distinguish unrevealed and revealed
faces. Preparation preserves source bytes and the archive; an identical complete
destination is reusable, while conflicting or incomplete output is refused.
Runtime loads only verified files from the prepared collection. Copy its whole
folder, including `manifest.json`, to relocate it; ordinary play needs no archive.

`--art-root PATH` selects a folder for that launch. **Choose asset folder** saves
the next-launch preference in `$XDG_CONFIG_HOME/arkham-unfolded/gathering.toml`
(normally `~/.config/arkham-unfolded/gathering.toml`). Without an override or saved
preference, Gathering uses `$XDG_DATA_HOME/arkham-unfolded/gathering-assets-v1`
(normally `~/.local/share/arkham-unfolded/gathering-assets-v1`). Other experiments'
settings remain separate. `--no-art` is available for diagnostic substitute mode.
Startup reports the chosen folder and verified coverage; absent or corrupt roles
use readable substitutes rather than an unchecked image.

The driver prints the current observation and legal choices. Send one JSON
request per line. At the initial decision, for example:

```json
{"op":"choose","revision":0,"id":"lead:tony"}
```

Each response supplies the current `revision` and choice identifiers. Use those
values for subsequent decisions; stale revisions and unavailable choices are
refused. Opening hands use the full shuffled decks. Select or unselect individual
cards using their offered mulligan choices, then choose `confirm-mulligan` with
the current revision. Confirming with no selected cards keeps the hand.

Other requests:

```json
{"op":"observe"}
{"op":"save"}
{"op":"reopen"}
{"op":"replay"}
{"op":"next"}
{"op":"back"}
{"op":"live"}
{"op":"quit"}
```

Choices save before they change the live session. A refused choice or failed
save leaves the live state and randomness unchanged. Reopening retains pending
mulligan selections; replay shows recorded observations without drawing again.
Replay does not accept live game choices. Records have their own Gathering kind
and source revision, so existing experimental records are not reinterpreted.

## Implemented boundary

Setup binds Tony's Internal Injury, Patrice's The 13th Vision and Jacqueline's
Chronophobia. It applies the selected permanents, Tony's six initial bounties,
Jacqueline's two mental trauma and starting horror, and five resources each.
Opening draws and mulligans replace weaknesses without resolving them, holding
them aside until the opening-hand step finishes.

After mulligans, the scenario places Study, the act and agenda decks and the six
set-aside cards, and shuffles the exact 26-card encounter multiset. The source
inventory is 40 physical cards: the other 14 have explicit destinations. Counts
and public inventory summaries do not reveal hidden draw order.

Ordinary setup initializes the runtime from the actual prepared hands, decks,
locations and randomness. Its scenario introduction and advancement moments are
recorded. The issue's
[current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060)
owns the exact implementation status and evidence.

Named hostile fixtures remain explicitly marked and rejected by the ordinary
driver and native launcher. They exercise difficult selected-card cases alongside
the ordinary preparation and round-flow checks. Their passing tests do not
establish native gameplay acceptance or prove every possible card composition.
The [Premonition follow-up](../research/issue-89-premonition-followup.md) and
[nested-group follow-up](../research/issue-89-nested-tokens-followup.md) replace the
earlier refusal boundaries with finite source-derived interpretations and witnesses.
They do not claim newly obtained official rulings for the exact combinations.

The current runtime increment adds Backpack's searched attachments, Ethereal Form,
Peter Sylvestre's end-of-turn healing and the documented Eyes of the Dreamer / Olive
composition. Safe Undo is offered until a new hidden-information or randomness
boundary; Undo remains in chronological history and replay. The original
encounter assembly is unchanged: independent inventory checks account for every
physical copy and its setup destination, rather than trusting set names alone.

Recorded presentation moments retain automatic token reveals, test results,
draws, harm and scenario transitions before cleanup. Replay can inspect these
moments without recomputing outcomes. The native adapter consumes the same public
projections for locations, enemies, assets, threats, commitments, pending costs
and token custody. Its live playback offers Next event, Pause/Resume and Show
pending choice; rules choices wait until that playback finishes. Inspection pauses
playback and card movement. Camera follow is optional in live play and resumes
on replay navigation after a manual override.

Each rules decision is saved before its presentation begins. Reopening that
session in the same window preserves an unfinished presentation; a fresh process
opens at the saved pending choice, with the intervening moments retained in
Replay. The visual cursor is not a separately persisted game decision. Compact
JSON keeps the measured 250-choice defeat recording below the existing 8 MiB
limit. This is a measured path, not an upper bound for every possible game.
Headless production-system checks cover these adapter behaviors; physical input,
readability, artwork and owner acceptance still require the actual window.

On 2026-11-02, a five-minute authorized native check of `fbf82d3` opened the
ordinary setup window at 3200 × 2000 with readable substitutes and healthy
foreground rendering. The helper initially refused Gathering's exact window
title; its title allowlist was corrected while preserving PID, process-start,
focus and geometry guards. The subsequent calibrated portal click landed inside
the visible lead-choice border, but the save remained at revision 0 and the
capture did not change. The pointer grant closed at 21:02:59 UTC and the isolated
launch was closed by 21:03:04, before the 21:03:42 deadline. The planned first
action and replay route was therefore **not verified**. The owner independently
reproduced the inert physical click and reported `No tab groups found` on Tab.
The original headless pointer tests missed continuous graphics-health updates:
the native monitor changes its sampling resource every frame, causing Gathering
to despawn the hovered controls before activation. A regression test reproduced
revision 0 instead of 1 with those updates; a separate test reproduced the exact
Tab navigation error after graphics admission.

The renderer now rebuilds for changes to graphics admission or its displayed
status, while preserving controls across ordinary sampling updates. The toolbar
and decision panel now register ordered Tab groups. The previously failing
tests pass, including raw mouse events at scales 1 and 2 with same-frame and
separate-frame press/release. These checks establish the reproduced regression
repair. The subsequent owner-authorized native retest of `0d5310b` passed the
bounded route: lead choice, all three mulligans, ordinary scenario entry,
Patrice gaining one resource (5 to 6 resources; 3 to 2 actions), replay from
moment 1 of 8 to 2 of 8, and return to the live table. The isolated save ended at
revision 6, stage `playing`. The pointer grant closed at 21:34:04 UTC and the
isolated process was confirmed absent by 21:34:17, before the 21:36:14 deadline.
The supervisor was explicitly interrupted for cleanup, so its exit 130 is not
normal-exit evidence. Retest receipts and captures are under
`~/.cache/agent-tmp/arkham-89-native-0d5310b/`.

This establishes the observed native pointer route, not a full scenario
playthrough, physical keyboard verification or owner acceptance. Captures also
show overlapping card labels, missing separator glyphs and right-edge clipping;
visual readability remains unfinished.
Captures and event receipts from the failed check remain local under
`~/.cache/agent-tmp/arkham-89-native-fbf82d3/`.

## Record compatibility

New Gathering sessions and recordings identify
`c35be2c:issue-89-gathering-runtime-v4`. This identifies the revised token-composition
semantics; it does not claim a new rules or taboo update.
The envelope format and other experiments' readers are unchanged.

Historical records remain readable and resumable with their preserved source:

| Record source identity | Preserved reader commit |
| --- | --- |
| `c35be2c:issue-89-gathering-setup-v1` | `7b0113275a0ae1bdb57e7f24467e6d3ed6af68d3` |
| `c35be2c:issue-89-gathering-runtime-v2` | `f431de9af84bedf20e0024370b9e6aaa8819a1ef` |
| `c35be2c:issue-89-gathering-runtime-v3` | `46f4c54a018802d22889a238629bcea27871d259` |

This applies to **ordinary setup records as well as verification fixtures**. The
current ordinary path continues into gameplay; historical readers retain their
original setup boundary. The current reader refuses their source identity before
decoding the Session or Recording payload, names the required preserved reader,
and leaves the original bytes unchanged. No personal-save migration is performed.
Do not edit a record's version fields to bypass the check: runtime fields and
the exact replayed decision history differ.

From a repository clone containing the required commit, prepare its separate reader.
The example selects runtime-v3; use the matching commit from the table for older sources:

```sh
gathering_reader_commit=46f4c54a018802d22889a238629bcea27871d259
gathering_reader_dir=$(mktemp -d /tmp/arkham-gathering-reader.XXXXXX)
git archive "$gathering_reader_commit" | tar -x -C "$gathering_reader_dir"
cargo build --locked --manifest-path "$gathering_reader_dir/Cargo.toml" \
  --target-dir "$gathering_reader_dir/target" --bin arkham-driver
```

Build dependencies must already be cached for an offline build; add `--offline`
in that case. The separate target directory keeps the preserved reader's build
artifacts apart from the current application. The source commit, rather than the
temporary checkout, is the durable recovery point.

Copy an ordinary session before resuming it, replacing the example original path:

```sh
gathering_resume_dir=$(mktemp -d /tmp/arkham-gathering-resume.XXXXXX)
cp -- /absolute/path/original.session.json "$gathering_resume_dir/session.json"
"$gathering_reader_dir/target/debug/arkham-driver" --experiment gathering \
  --load "$gathering_resume_dir/session.json" --save "$gathering_resume_dir/session.json"
```

The driver prints the exact pending observation. Use its offered revision and
choice ID to continue. For recorded replay, send `{"op":"replay"}`, then
`{"op":"next"}` or `{"op":"back"}`; `{"op":"live"}` returns to the saved
live state and `{"op":"quit"}` exits. A completed historical setup record stays at its
original unavailable-gameplay boundary. It does not become a new playable run.

For native ordinary setup resume, build and run the same preserved source with
`--features tabletop --bin arkham-tabletop`, passing the copied save:

```sh
cargo run --locked --manifest-path "$gathering_reader_dir/Cargo.toml" \
  --target-dir "$gathering_reader_dir/target" --features tabletop --bin arkham-tabletop -- \
  --experiment gathering --save "$gathering_resume_dir/session.json" --no-art
```

The native Replay controls replay that session. Verification fixtures remain
excluded from both ordinary launchers. For a standalone exported recording, or
a developer's preserved verification session, the following temporary Rust
example uses the same preserved continuity APIs. It does not create fixtures or
make them ordinary player sessions. Create it only in the archived reader:

```sh
mkdir -p "$gathering_reader_dir/examples"
cat > "$gathering_reader_dir/examples/preserved_record.rs" <<'RS'
use arkham_unfolded_experiment::gathering::{Recording, continuity::{
    Replay, load_recording, load_session, commit_choice,
}};
use std::{error::Error, io};
fn replay(recording: Recording) -> Result<(), Box<dyn Error>> {
    let mut replay = Replay::new(recording)?;
    loop {
        println!("{}", serde_json::to_string_pretty(replay.current())?);
        eprintln!("next | back | quit");
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 { break; }
        match line.trim() {
            "next" => { replay.next(); }, "back" => { replay.back(); },
            "quit" => break, _ => eprintln!("Use next, back or quit."),
        }
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    let (kind, path) = (args.get(1).ok_or("session|recording required")?,
                        args.get(2).ok_or("copied record path required")?);
    if kind == "recording" { return replay(load_recording(path)?); }
    if kind != "session" { return Err("Use session or recording".into()); }
    let mut session = load_session(path)?;
    loop {
        println!("{}", serde_json::to_string_pretty(&session.observe_for_play())?);
        eprintln!("choose REVISION ID | replay | quit");
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 { break; }
        let parts: Vec<_> = line.split_whitespace().collect();
        match parts.as_slice() {
            ["choose", revision, id] => commit_choice(path, &mut session, revision.parse()?, id)?,
            ["replay"] => replay(session.recording())?, ["quit"] => break,
            _ => eprintln!("Use an offered revision/ID, replay or quit."),
        }
    }
    Ok(())
}
RS
cargo run --locked --manifest-path "$gathering_reader_dir/Cargo.toml" \
  --target-dir "$gathering_reader_dir/target" --example preserved_record -- \
  recording /absolute/path/original.recording.json
```

Use `session "$gathering_resume_dir/session.json"` instead of the final two
arguments for a copied verification session. Original record files and the
current application remain unchanged by this recovery procedure. Historical
producer evidence covers pending and completed ordinary setup, pending runtime
revelation and pending skill test: the preserved reader loaded/replayed all four
and resumed each nonterminal case. The new reader rejects all eight original
session/recording files with the explicit compatibility message. This establishes
the preserved-reader route, not a migration or native visual acceptance.

The runtime-v3 change repeats that compatibility probe against actual runtime-v2
source at `f431de9`: eight historical session/recording files cover the same four
positions. Its reader loaded and replayed all cases and resumed nonterminal
sessions; the new reader refused both loading and replacement without changing
any original bytes. These probes use temporary developer records, not personal
saves.

The runtime-v4 change repeats the same producer, load, replay and resume probe
against actual runtime-v3 source at `46f4c54`. All eight historical files remain
unchanged; the current reader refuses both loading and replacement and names that
preserved reader. Ordinary historical setup still stops at its original gate.
