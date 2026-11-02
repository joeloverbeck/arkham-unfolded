# Gathering setup implementation

This is the first implementation increment toward complete original Gathering
under [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89). It uses the
actual three selected starting lists, rather than the prepared Study experiment's
scripted hands. **Scenario play is not yet available.** Completing setup does not
complete the first-round milestone or the scenario.

The [research return](../research/issue-89-rules-investigation.md) supplies the
source-derived behavior. This implementation pins that research baseline and
taboo 10; it does not claim to have checked the latest taboo for a new playable
campaign. The original deck revisions and explicit weakness selections are
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

Records from the previous setup-v1 checkpoint use the preserved reader described
under [Record compatibility](#record-compatibility), rather than this build.

The native setup uses the same session and save file:

```sh
cargo run --locked --features tabletop --bin arkham-tabletop -- \
  --experiment gathering --save /tmp/arkham-gathering-setup.session.json --no-art
```

Choose a lead investigator and use the offered mulligan controls. Click a card
for its printed fields and effective rules. The encounter deck inspection lists
its starting quantities without revealing shuffled order. Camera controls use
the existing Overview, Scenario and investigator views. Save, Reopen, History
and Replay operate on this setup; replay permits inspection but no live choices.
Headless tests exercise the rendered lead button and save/replay protections.
The actual window has not yet received visual, physical-input or owner acceptance.

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

The component assembly does not yet present the scenario introduction or
advanced act/agenda narrative. The final observation explains that
scenario play is unavailable and offers no
invented gameplay choices. Full card effects, complete rounds, scenario endings,
complete native presentation and their corresponding continuity checks remain part of
the complete Gathering target. The issue's
[current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060)
owns the exact implementation status and evidence.

The verification-only runtime now exercises actual post-mulligan hands and ordinary
actions, selected card effects, scenario advancement and ending records, phase
flow, and interrupted draws and tests. Named hostile fixtures are explicitly
marked and rejected by the ordinary driver and native setup launcher. Their
passing tests do not establish complete selected-card coverage or native gameplay.
In particular, [nested token groups and some Premonition compositions](../research/issue-89-token-compositions.md)
still have explicit unsupported boundaries. The runtime is not enabled as ordinary
scenario play while those and other reachable behaviors remain incomplete.

## Record compatibility

New Gathering sessions and recordings identify
`c35be2c:issue-89-gathering-runtime-v2`. This distinguishes the expanded runtime
state from `c35be2c:issue-89-gathering-setup-v1`; it does not claim a new rules or
taboo update. The envelope format and other experiments' readers are unchanged.

Setup-v1 records remain readable and resumable with preserved commit
`7b0113275a0ae1bdb57e7f24467e6d3ed6af68d3`. This applies to **ordinary setup
records as well as verification fixtures**, even though ordinary setup behavior
has not expanded. The current reader refuses their source identity before
decoding the Session or Recording payload, names the required preserved reader,
and leaves the original bytes unchanged. No personal-save migration is performed.
Do not edit a record's version fields to bypass the check: runtime fields and
the exact replayed decision history differ.

From a repository clone containing that commit, prepare its separate reader:

```sh
gathering_reader_dir=$(mktemp -d /tmp/arkham-gathering-setup-v1.XXXXXX)
git archive 7b0113275a0ae1bdb57e7f24467e6d3ed6af68d3 | tar -x -C "$gathering_reader_dir"
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
live state and `{"op":"quit"}` exits. A completed setup-v1 record stays at its
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
