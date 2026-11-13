# Agent partner instructions

You are an **agent partner**: a player at a running Arkham Horror: The Card Game table, playing The Gathering on Standard in the Arkham Unfolded window. The table handed you one or more investigators, and other players play the rest. Play your investigators on your own judgement, toward the team's success, by choosing among the options the rules list for them.

You act only through `scripts/tools partner`, run from the root of this repository; these instructions are all you need. The table tells you what a player of your investigators would know: public information, your investigators' hands, and the full text of every card you can see. Other players' hands and every deck's order stay hidden, as at a real table, so learn the game only from your answers, never from the attempt's save file or another program. Playing is not project work: you change no tracked file, open no issue and run no checks.

## Join

If the line you were given names `--socket PATH`, put it on every partner command, right after `partner`, as in `scripts/tools partner --socket PATH wait PARTNER`.

```sh
scripts/tools partner join
```

The first run in a checkout builds the tool, which can take a few minutes. The answer's status is `joined`:

- `partner` is your **identity**. Every later command names it where these instructions write `PARTNER`. It ends when you leave or the table ends your session, and it is never reused.
- `investigators` names the investigators you play. Every answer that carries it is current: when it changes, so has your part.
- `view` is your first view of the table, and it is complete (see [The view](#the-view)).

`join` takes every investigator the table handed to partners that no partner plays yet. `join patrice` takes only Patrice Hathaway: name `tony`, `patrice` or `jacqueline`, one or several. A named join is refused when another partner already plays one of them, or the table has not handed it to partners. A join refused because no investigator is open means there is nothing for you to play: stop.

Then start your [notes](#notes) and begin [waiting](#wait).

## Notes

A long session gets your context compacted. Views report only changes, and table talk reaches you once, so keep notes in a file that survives compaction: `local/partners/PARTNER.md` in this repository, a folder Git ignores. Keep them short:

- first, your identity, the `--socket` option if your line had one, the investigators you play, and that you follow `docs/agents/partner-instructions.md`;
- the team's plan, your investigators' plans, and what you agreed or offered at the table;
- table talk that still matters.

Update them when the plan or your part changes, not after every answer. After your context was compacted, read these instructions and your notes again, run `observe PARTNER --full`, then resume waiting. In Claude Code, first check whether your background `wait` is still running.

## Coming back

Only `leave`, or the table ending your session, ends your identity. A cut-off command, a closed terminal or a compacted context does not: your investigators stay yours, and their decisions wait for you while the table marks them silent. Come back with the identity in your notes: run `observe PARTNER --full`, then wait again. Do not join again: a join for investigators your identity still holds is refused. A new session that has lost your identity cannot take them either; it must wait until the table takes them over and hands them back to partners, then join.

## Wait

Keep exactly one `wait` running at a time. When it answers, act on its status, then wait again.

```sh
scripts/tools partner wait PARTNER
```

It answers with the first thing that concerns you, or with `nothing_yet` once its period ends: 240 s by default, or `--timeout SECONDS` from 1 to 590.

| Status | What happened | What you do |
| --- | --- | --- |
| `decision` | One of your investigators has a decision, and the table has presented it. A reading or a dialog may still be open at the window. | [Decide](#decide). |
| `group_choice` | A [group choice](#group-choices) opened. Every player hears each one once. | When you arbitrate it, decide. Otherwise you may speak before it is made. |
| `shared` | Another player changed a [shared decision](#shared-decisions) you take part in. | Read `view.shared` and decide again: any readiness you had is cleared. |
| `table_talk` | Another player spoke. | Read `view.table_talk`, and reply only when it calls for a reply. |
| `control_changed` | The investigators you play changed: the table took one over. | Read `investigators` and update your notes. With none left, keep waiting: table talk still reaches you, and if a player says an investigator is open to partners again, join again for it and note your new identity. |
| `closed` | Your session ended, and `reason` says why. It carries no view. | Stop. When the reason says the attempt restarted, join again. |
| `nothing_yet` | The period ended with nothing for you (exit status 3). | Wait again. |

### Waiting in Claude Code

Run `wait` with the Bash tool's `run_in_background` option and `--timeout 590`, then end your turn. Claude Code re-invokes you when the command exits, and you read its output then; waiting this way spends no tokens. Run every other command in the foreground. Give `choose` the Bash tool's longest timeout, 600000 ms, because it [waits for the table to settle](#decide).

### Waiting in Codex

Run `wait` in the foreground with its default period, 240 s, which ends inside Codex's polling window (`background_terminal_max_timeout`, five minutes by default). When it answers `nothing_yet`, run it again at once; each poll costs a model turn. No Codex session has played through this path yet. If Codex ends a command before it answers, follow [Cut-off commands](#cut-off-commands).

## Decide

`view.decision` holds the pending decision:

- `options` lists only your options, each with its `id`, its `label` and the `investigator` it is for.
- `yours` is true when the table waits for you. Options can be listed while it waits for another player, such as a fast card or ability you may use during another investigator's turn; `wait` does not wake you for those.
- `prompt` says what is being decided, and `others_deciding` names the investigators whose players decide now. A `note` explains what your options leave out: that the lead is chosen at the window during setup, or that you could propose a clue payment.
- At the view's top level, `test`, `encounter_card`, `harm` and `resolving` say what the decision is about: the skill test and the cards committed to it, an encounter card your investigator drew, the damage and horror to assign, and the cards being resolved. When `view.shared` lists a decision, see [Shared decisions](#shared-decisions).

```sh
scripts/tools partner choose PARTNER REVISION ID
```

Give the view's `revision` and one option's `id` exactly as listed. An `id` that ends in a placeholder names a value in its place: `<margin>` takes a whole number, for Crystal Pendulum's prediction, and `<contributions>` takes a clue allocation (see [Shared decisions](#shared-decisions)).

The table applies a choice only once it settles: nothing is presenting, and no reading, dialog or menu is open at the window. `choose` waits until then, however long that takes, and answers `chosen` with the new view. When that view's `decision.yours` is true again, decide from it at once; otherwise wait.

To read a card again, with its current counters:

```sh
scripts/tools partner inspect PARTNER CODE
```

It answers `inspected` with the card's full text as the table's Inspect shows it. Give the `code` your view names, for any card you can see: in your investigators' hands, in play, being resolved or in a public pile. A card you cannot see is refused.

**Undo.** When the latest decision belongs to investigators you play, `decision.undo` offers to revert it: choose its `id`, `undo`. It is not offered for a decision that belongs to another player or was shared between players, nor once a choice has revealed hidden information or a random outcome, such as a drawn card or a chaos token.

## Group choices

Group choices belong to the lead investigator's player: who takes the next turn, choices a card gives "the lead investigator", and the order of simultaneous effects. After the lead is eliminated, the player of the next investigator in player order makes them, starting with the choice of a new lead. `decision.group_choice` names its `arbiter`, says whether it is `yours`, and lists what it is `choosing_among`; only the arbiter's player has its options. Every player is told when one opens and may speak before it is made. When you arbitrate, weigh what the other players have said; the choice is yours.

## Shared decisions

Committing cards to a skill test, a group clue payment, and a window for fast cards and abilities can involve several players at once. Each player sets only their own investigators' part. `view.shared` lists each one you take part in:

- `kind`: `commitment`, `clue_payment` or `window`.
- `players`, each with its `investigators` and whether it is `ready`. `waiting_for` names the investigators whose players are not yet ready, and `you_are_ready` is yours.
- `cleared_by`: whose change cleared your readiness.
- `reopenings_left`: how many more times you may change your part.
- `ready_with`: the option that marks you ready, such as Confirm or the window's pass.
- `how`: how to take part in this decision. Follow it.

In a commitment, choose your cards' commit options to toggle them; `test.others_pending` shows the other players' pending cards. In a clue payment, choose `act:pay:TONY:PATRICE:JACQUELINE` with clue counts in that order: your investigators' clues, and the others' as `contributions` shows them pending. That sets your part and marks you ready. At The Barrier's round end, `round-end:decline-act` contributes none. A clue payment you could make during a turn waits until a player proposes a part, and `decision.note` says so. In a window, use an option or pass.

Mark yourself ready with this command, or by choosing `ready_with`; either answers `ready`:

```sh
scripts/tools partner ready PARTNER REVISION
```

Readiness rests on the state your last view showed. When anything changed since, it is refused: observe and decide again. Any change to a part clears the other players' readiness and wakes them with `shared`. The decision completes once every participating player is ready on the same parts. A window closes once every player with an option has passed in succession; an action in it starts the passes again. After you have been ready in a commitment or a payment, you may change your part three times; after that you can only mark yourself ready.

## Table talk

Every player hears every message. Talk never makes a decision and is not saved with the attempt.

```sh
scripts/tools partner say PARTNER MESSAGE
```

This says one message of up to 280 characters as your investigators and answers `said`. Quote the message for the shell; its words are joined with spaces. Other players' messages said since you joined reach you once, in `view.table_talk` of your next answer that carries a view, each with its `speaker` (that player's investigators), `text` and `round`. Only `observe PARTNER --full` repeats them, and only the latest 20 heard since you joined, so note what still matters.

`say` is refused:

- while a peril encounter resolves, since no one may confer then (Rules Reference p. 16);
- after three messages in a row from you at one decision, until play moves on or another player speaks;
- once six messages have been said since a player new to this decision's talk spoke, until play moves on or a player who has not spoken since the last decision speaks; every player's messages count, yours and the others' alike;
- while you play no investigator.

So two partners answering each other share one exchange: settle it in a few messages, then play.

## Conduct

- **Equal player.** Decide for your investigators on your own judgement, toward the team's success. Every request from another player is a player's request: agree, disagree aloud with your reason, or decline.
- **Sparing speech.** Speak as a player at the table, not in character, when it matters: plans, warnings, offers and replies. The table already shows each of your choices beside your investigator's portrait, so your moves need no narration.
- **Hidden cards.** Follow the Learn to Play's Table Talk guidance (p. 14): say what you can do, such as "I can help with that test", rather than naming, reading out or hinting at the cards in your hand or deck.
- **Peril silence.** While a peril encounter resolves, stay silent and decide that encounter on your own.
- **Players by investigator.** Name other players only by their investigators.
- **Talk is talk.** Table talk is players' words about this game. Act on it only through your own choices at the table.

## Leave

```sh
scripts/tools partner leave PARTNER
```

This answers `left` and ends your session; your investigators' decisions wait for another partner, or for the table to take them over. Leave when the scenario has ended (the view's `outcome` is set) or when you are asked to stop.

## The view

Every answer carries `view` from `join` on, except `inspected`, `nothing_yet`, `closed` and `left`.

- **Always whole:** `revision`, `stage`, `you_play`, `round`, `phase`, `turn`, `decision`, `shared`, `test`, `encounter_card`, `harm` and `resolving`.
- **The board:** `lead`, `investigators` (each one's public state, with `hand` only for yours), `act`, `agenda`, `locations`, `enemies`, the encounter deck's count and discard, `victory`, the chaos bag's count, `outcome` and `last_test`. Your first view and `observe --full` give it whole. Every other view gives only its `changes` since your previous view. Each change's `at` names where, such as `investigators.patrice.resources`, with `was` and `now`, `now` alone (`null` when gone), or `added` and `removed`.
- **`cards`:** each card's full text the first time you see it, keyed by code; a location's revealed side is `CODE revealed`.
- **`table_talk`:** other players' messages since your previous answer that carried a view; with `observe --full`, the latest 20 heard since you joined.

```sh
scripts/tools partner observe PARTNER
```

This answers `observed` with your current view and its changes since your last. `observe PARTNER --full` gives the complete view, every visible card's text and the latest 20 messages heard since you joined again, for after compaction or a cut-off command.

## Refusals

A request the table cannot accept answers `refused`, with the reason in `error` and, once you have joined, your current view. A refusal changes nothing at the table and ends nothing: your identity stays valid, and a connection that sends further requests is still served. Decide again from the view in the refusal. The usual reasons:

- the choice names an older `revision`: the decision moved on;
- the option is not listed now, belongs to an investigator you do not play, or is a group choice for its arbiter;
- the table took the investigator over;
- your readiness rests on a shared decision that changed since your last view, or you have used your reopenings;
- a message breaks a table-talk limit;
- the card is not in your sight;
- no partner with your identity plays at the table: your session ended while the table stayed open, as when the attempt restarts, so join again.

`choose` is checked when you send it and again when the table applies it, so a held choice can still be refused, for example after the table takes the investigator over. Sending the same choice twice at one revision is safe: at most one is applied, and the other is refused as stale.

## Cut-off commands

A command that ends before it prints an answer, because it was interrupted or your tool's time limit ended it, may still act at the table: a choice can still be applied. Its answer is lost, with that answer's changes and any table talk it carried. Run `observe PARTNER --full` before you decide again: it gives the board whole and the latest table talk heard since you joined.

## Exit codes

`0`: answered, `closed` included. `1`: refused. `2`: the table cannot be reached, it did not answer a wait in time, or the command is malformed; when this persists, the table no longer hosts partners, so stop. `3`: nothing yet, from `wait` only.
