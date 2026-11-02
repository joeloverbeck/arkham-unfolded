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
is a developer-accessible player interface; no new native tabletop screen or
human gameplay acceptance is claimed.

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
act/agenda narrative in a native screen. The final observation explains that
scenario play is unavailable and offers no
invented gameplay choices. Full card effects, complete rounds, scenario endings,
native presentation and their corresponding continuity checks remain part of
the complete Gathering target. The issue's
[current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060)
owns the exact implementation status and evidence.
