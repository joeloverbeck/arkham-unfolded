# Agent seat

An agent can sit at the real Gathering table with no window. It was built as a first increment under [#153](https://github.com/joeloverbeck/arkham-unfolded/issues/153) and merged on the owner's authorization on 2026-11-11. It is not yet the routine verification path: that still needs a materially different second use and a return use from these documents alone.

```sh
scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop &&
target/debug/arkham-tabletop --experiment gathering --agent-seat --save /absolute/path/to/attempt.json
```

It is the same table, rules session, save path and artwork verification as native play: an existing compatible file resumes, an unused path starts setup, and each committed decision autosaves there. `--experiment gathering` and `--save` are required. `--art-root` works as in [native play](../playing.md#launch-and-resume-the-gathering), and `--initial-view` starts in one of the views listed under **Camera views** below; `--attempt` and `--screenshot` are refused. The seat opens no window and no dialog: it has no window event loop, and artwork recovery, the artwork-folder chooser and the return to scenario selection are not part of it. The seat refuses to open external links: a Credits link, which native play hands to the desktop's opener, only sets `notice` to its address. A startup problem, including a refused option, is one `refused` response, the same text on standard error and an unsuccessful exit.

In batch form the seat reads one JSON request per line on standard input and writes one JSON response per line on standard output. Diagnostics go to standard error. A [held session](#held-session) exchanges the same lines over a local socket instead. The first response has status `ready` and names the graphics `adapter` that rendered the table. Every response carries `ok`, `mode: "seat"`, `status`, an `error` whenever `ok` is false, the player-visible `observation` exactly as the headless driver prints it, and `table`, what the table shows:

- `settled` and `pending`: whether the table will accept input, and what is still in progress when it will not. A dialog or menu waiting for the player is settled.
- `shows_latest`: false while the table is still presenting its way to the observation, for example behind a reading dialog.
- `modal`: the surface that captures the table, such as `inspection` or `story reading`, or `null`.
- `notice`: the decision panel's notice, such as a decision that was not committed, or `null`.
- `components`: each shown piece that selects something, with its `name`, `label`, logical screen `position`, `on_screen`, and `available` when it shows the gameplay availability cue. Names are the table's source identities, such as `investigator:tony`, `card:23`, `location:01111`, `deck:tony`, `chaos-bag` and `token:0`. A card keeps the bare name; another piece showing the same source adds what it is, such as `investigator:tony/standee` or `location:01111/clues`.
- `menu`: the open contextual menu and its `rows`, or `null`.
- `controls`: every other shown button. Rows and controls carry `choice` (the choice identity, or `null` for controls such as Continue, Inspect and Menu), `label`, `position`, `on_screen` and `focused`.

| Request | Effect |
| --- | --- |
| `{"op":"observe"}` | Return the current observation without advancing the table. |
| `{"op":"activate","revision":N,"component":"NAME"}` | Move the pointer onto the named component, then press and release. |
| `{"op":"activate","revision":N,"choice":"ID"}` | The same for the one shown control offering that choice. |
| `{"op":"scroll","revision":N,"choice":"ID","y":-4}` | Wheel down over that visible choice; `component` can name a visible component instead. |
| `{"op":"key","revision":N,"key":"Escape"}` | Press and release one key: `Escape`, `Enter`, `Space`, `Tab`, `Home`, `End`, `PageUp`, `PageDown`, the four `Arrow` keys, `KeyW`, `KeyA`, `KeyS`, `KeyD`, `Minus` or `Equal`. |
| `{"op":"key","revision":N,"key":"Enter","focused":"NAME"}` | The same for `Enter` or `Space`, which can commit a choice or dismiss a reading: it presses only when the focused control is the one named. |
| `{"op":"settle","steps":N}` | Step the table until it settles or `steps` is reached (default 1,800, at most 7,200). An `unsettled` response names what remains; ask again to continue. |
| `{"op":"capture","path":"/absolute/path.png"}` | Write a picture of the table. The response follows the completed file. |
| `{"op":"quit"}` | End the session. The attempt stays resumable. |

Selecting a component and choosing from its menu are two activations, as for a player. `revision`, required on every activation and key, is the observation revision the request was decided on; a different current revision is refused as outdated and nothing is pressed. Names match exactly: an absent, ambiguous or off-screen name is refused, never matched to something near. One choice shown both by an open contextual menu's row and by another control, such as the decision bar, names the menu row, because the open menu captures the pointer. Activations and keys are refused while the table is busy, because the table drops input while presenting. While the camera still moves, `pending` names `camera motion` (see **Camera views** below), so settle before the next request. The seat rests the pointer on a part of the named component that nothing covers, as a player would beside a counter standing on a card; when it presses at a later point, `covered` reports the first point it tried and what the pointer found there. When every part is covered it answers `missed`, reports what the pointer is over in `hit` and presses nothing. A component's `on_screen` does not predict this: it means only that the component's reported position lies inside the viewport, so a component wholly under a panel can still be `on_screen: true` (see **Covered cards** below). If something moves under the pointer after it rests, the press lands there instead: the seat cancels it before releasing, so it is not a click, and answers `missed` with `pressed: true`. Like a refusal, a miss stops the connection's later activations and keys (see **Stop after a refusal**). Table time advances only in the seat's own fixed steps of one sixtieth of a second, never with the wall clock.

**Guarded keys.** `Enter` and `Space` are the keys that can commit a choice or dismiss a reading: a focused button activates on either, and in the group clue payment `Enter` on a focused investigator row spends the clues, so that row, which has no choice identity, is named by its label. A request for either must name, in `focused`, the control it expects to press: the focused control's `choice` when it has one, otherwise its `label`, matched exactly. When nothing has focus, or something else does, the key is refused, nothing is pressed and the revision is unchanged; the error says what has focus. Every other key moves focus, the camera or a scroll position, steps the group clue payment's amounts, or closes a menu or dialog without deciding, so it needs only `revision`; a `focused` given with it is checked the same way. With a test result's **Continue** focused at revision 12, and with the `turn:end` control focused at revision 13:

```json
{"op":"key","revision":12,"key":"Enter","focused":"Continue"}
{"op":"key","revision":13,"key":"Space","focused":"turn:end"}
```

`{"op":"key","revision":12,"key":"Enter"}` is refused, as is the first request above when no reading is up and focus is on another control.

**Wheel input.** `scroll` requires the current revision and exactly one visible named target. It checks actual picking before sending ordinary wheel input, retaining the table's modal and camera-capture policy. `y` and optional `x` (default zero) are wheel lines: each must be finite and between -32 and 32, with at least one nonzero axis. Negative `y` scrolls down. A `scrolled` response reports input delivery, position and delta; inspect the resulting table or capture to verify movement. Offscreen or covered targets are refused or missed, as with activation.

**Stop after a refusal.** Standard input is one connection. After the first `refused` response on it, including one for a malformed line, or the first `missed` target, the seat stops acting: every later `activate`, `key` and `scroll` is answered with status `skipped`, `ok: false`, no input sent, and an `error` naming the refused or missed request and its reason. A miss presses nothing, or cancels its press before release, but it shows the table was not as the agent assumed, so it stops the connection as a refusal does. `observe`, `settle` and `capture` are still served, and `quit` still ends the session. In this batch, sent when no reading was up, the Enter is refused, so the activation planned after it is skipped instead of acting on a table the agent did not expect:

```text
{"op":"key","revision":12,"key":"Enter","focused":"Continue"}   refused: focus is on another control
{"op":"activate","revision":12,"choice":"turn:end"}             skipped
{"op":"observe"}                                                 observed
```

Controls without a choice identity are reached with `Tab` and `Enter` where the table's tab order includes them, naming their `label` in `focused`, or closed with `Escape`. The **Menu** button comes last in that order, after the portrait rail, the decision panel and the sideboard headings (see **Tab order and focus** below), so the utilities menu (action history, save, restart, motion, Credits) is within reach; **Save and return to scenario selection** saves and ends the session with `exited`, and no selection window opens. Known limits of this increment: the Crystal Pendulum and Violin pickers compose their choice from several presses and are keyboard-only. It is not established whether the table could drop a choice pressed on a dialog while consequences still resolve behind it, after the seat has answered `activated`; an unchanged `revision` in the response would show it.

**Where choices appear.** `observation.choices` lists every legal choice. Most choices have one entry point; prevention and the round-end group clue payment also have direct controls as described below:

- A choice that belongs to no piece, such as ending a turn, Undo or a confirmation, is a decision-bar control in `controls`, with its `choice`. While a dialog is open, the decision bar offers no choices; the dialog's own buttons carry them.
- A choice that belongs to a card or another piece, such as playing a card, moving to a location or choosing the lead investigator, is only a row of that piece's contextual menu. Activate the component first; the response then shows `menu`, with the choice among `menu.rows`, and a second activation chooses it. A component marked `available` has at least one choice in its menu. A piece with no choice and one thing to inspect opens the inspection instead of a menu.
- A choice that belongs to no shown piece is a row of the menu titled `Current decision`, opened by the decision bar's **Pending decision** control. That control has no `choice`, so reach it with `Tab` and `Enter`.

So a choice whose identity is a `choice` in `controls` is one activation away; any other needs its piece, marked `available`, or **Pending decision** first.

Legal Deny Existence and Idol of Xanatos prevention appears directly beside **Apply** after damage/horror assignment, as well as in the card or asset menu. Deny's Agenda 1 hand-discard response also appears directly beside the choice to accept the discard, before any random discard. The direct controls use the same legal choices and investigator ownership as their contextual actions. Pending amounts, assignments and Apply labels omit damage or horror when that type is absent.

A step with exactly one legal outcome is not offered as a choice: the rules session performs it and adds `Automatic: ` and the step's label to `observation.history`. For example, when only one investigator has yet to take a turn in the investigation phase, that turn begins without a `turn:` choice, and the history records an entry such as `Automatic: Begin Patrice Hathaway's turn`. Only these steps wait for the player even with one choice: the chaos-token reveal and its token selections, committing cards to a test, and the group clue payment. Test result and story reading stops are presentation pauses with no rules choice (see **Readings**).

The group clue payment is listed as one template choice, `act:pay:<contributions>`. It is a row of the act's menu and, at round end, also a decision-bar control beside `round-end:decline-act`, so at round end it is shown in two places. Activating it opens the chooser and commits nothing, so the revision is unchanged. Once the allocation is complete and legal, the chooser's **Spend N clues and advance** control carries the concrete choice, such as `act:pay:4:1:1`, and activating that control pays.

**Readings.** A test result, a story reading (an act or agenda reverse) and an encounter card each open a reading over the table, which `modal` names `test result`, `story reading` or `encounter reading`. **Continue** has focus when a reading opens, and `Enter` naming it presses it. On a test result or story reading, Continue only dismisses, and `focused` names it by its label, `Continue`. On an encounter reading whose revelation is still to resolve, Continue carries the choice `revelation:resolve` and commits the revelation: it is a decision, not a dismissal, and `focused` names it `revelation:resolve`. A response such as Ward of Protection is another control on that reading, chosen before Continue. When a test offers effects after its result, `modal` is `result opportunities` and focus starts on the first effect, such as Lucky!, or on the decline when no effect is listed; `Enter` there commits that choice. `Escape` closes none of these. It closes the contextual menu, the utilities menu, an inspection (including Credits), the action history, the restart confirmation, and the Crystal Pendulum, Violin and group clue payment choosers.

Opening a save shows again the test results and story readings its most recent committed decision produced, and the encounter card of a revelation still to resolve. They return even when they were dismissed before the previous launch ended, and when the headless driver made that decision. In batch form, the first `Enter` of a launch can meet such a reading instead of the table.

**Card text.** Responses carry no card text: the observation names each card by `code` and `title`, and an open inspection appears only as `modal: "inspection"`. To read a card, open its inspection, from an **Inspect** row of its menu, which has no `choice` (reach it with `Tab`, then press `Enter` naming its label), or by activating a piece with no choice and one thing to inspect, then `capture` the table and read the image. `Escape` closes the inspection.

**Tab order and focus.** With no dialog or menu open, `Tab` moves through four groups, then wraps to the start:

1. the portrait rail: **Focus Tony Morgan**, **Focus Patrice Hathaway** and **Focus Jacqueline Fine** (the current investigator's label ends in `· current turn`), then **Scenario**, **Test**, **Overview**, and **Resolve** while a resolution drawer is open;
2. the decision panel's controls;
3. the sideboard headings, one for each location group on the enemy sideboard, then on the **Cards at locations** board;
4. the **Menu** button.

While a dialog or menu has focus, `Tab` cycles through its own buttons. `controls` lists buttons in screen reading order, not Tab order. The seat reports `focused` only for controls and menu rows: a sideboard heading is a component and carries no `focused`, so a `Tab` response with nothing focused means a heading has focus. A player's `Enter` there selects its location, but the seat refuses `Enter` and `Space` on a heading, because `focused` cannot name it; activate the heading's component instead. With no dialog or menu open, focus normally starts on the first decision-bar control that carries a `choice`, or on **Overview** when there is none, and returns there whenever the table changes, such as after a commit, a dismissed reading or a new camera view. Chaos-token decisions, such as a test's token reveal, and Tony's Watch this! wager are the exceptions: the decision panel shows that decision's own controls in place of the decision bar, such as token abilities, `undo` and `test:reveal`, and gives none of them focus. When `undo`, `tokens:confirm` or another choice the decision bar would show is offered, nothing has focus; otherwise **Overview** has it. Check `focused`, and activate by `choice` there or `Tab` before a guarded key. An `Enter` meant for a reading that is not shown would press whatever control has focus; naming the expected control in `focused` (see **Guarded keys**) has it refused instead. Read `focused` after each `Tab` instead of counting presses; the count depends on which controls and headings are shown.

**Camera views.** Each launch starts at **Overview** unless `--initial-view` names another view. To change view, `Tab` to its rail button and press `Enter`: **Scenario** frames the map and both sideboards, **Test** the test area, **Overview** the whole table, a **Focus** portrait that investigator's area and **Resolve** the resolution drawer. A portrait only moves the camera; it does not choose who acts. The view changes at once. `pending` names `camera motion` only while the camera still moves, for example while camera following is on (**Menu → Resume camera following**); then settle before the next request. `Home`, the arrow keys, `Minus` and `Equal` move the camera only while no button has focus, and a button normally has focus, so use the rail buttons. To start in a view, launch with `--initial-view overview`, `scenario`, `test`, `tony`, `patrice`, `jacqueline` or `resolve`. `resolve` needs an existing save with an open resolution drawer; otherwise the seat refuses to start. `Home` still means **Overview**.

**Covered cards.** A view can leave a piece inside the viewport but wholly under a panel, where activating it answers `missed`. In Overview the decision panel can cover the investigators' hand cards: activate a hand card, including one to commit to a test, from its investigator's **Focus** view. Some decisions put their cards in an investigator's resolution drawer: cards searched from a deck, the cards of a discard being chosen or ordered, such as Patrice's upkeep discard, and cards being resolved outside the test area, including a revelation still to resolve. Chaos tokens revealed outside a skill test go there too. While the drawer holds them, `controls` includes **Resolve**, and `observation.play.pending.resolution_owner` names whose drawer it is. Drawer cards can stay covered even in that investigator's **Focus** view, so choose them from **Resolve**.

Every committed decision is saved, so in batch form a shell that runs one command at a time can send a batch of requests in one launch and resume the attempt in the next. Camera position and an open menu do not survive a launch; a held session keeps them.

Each response repeats the whole observation and table, about 15 KB, so keep the responses in a file and read only what the next decision needs. This batch dismisses a test result read at revision 12, settles and quits; `focused` in the view is what the next guarded key must name:

```sh
printf '%s\n' '{"op":"key","revision":12,"key":"Enter","focused":"Continue"}' \
    '{"op":"settle"}' '{"op":"quit"}' |
  target/debug/arkham-tabletop --experiment gathering --agent-seat \
    --save /absolute/path/to/attempt.json > responses.jsonl
jq -c '{status, error, revision: .observation.revision, settled: .table.settled,
        pending: .table.pending, modal: .table.modal, menu: .table.menu,
        focused: [(.table.controls[]?, .table.menu.rows[]?) | select(.focused) | {choice, label}]}' \
  responses.jsonl
```

**Evidence limits.** A seat run can establish presentation logic, layout, controls and rendered output on the seat's image, with real artwork and a real graphics adapter. It cannot establish window behaviour, display pacing, focus or physical input, and it is not acceptance of native play. The seat's own admission requires decoded artwork, the registered symbol font, ready render pipelines and completed GPU work for its image; it makes no claim about display rate, and native rendering health is unchanged. Protocol behaviour is checked without a graphics card in the ordinary tests. The real table, including admission, mesh picking, capture, resuming, the end of a session when artwork becomes unavailable, and a held session between tool commands with a refused second start and its idle end, is checked only where the prepared collection and a graphics adapter exist:

```sh
ARKHAM_GATHERING_TEST_ASSETS=/absolute/path/to/prepared-collection \
  scripts/dev cargo test --locked --features tabletop --lib \
  gathering_tabletop::seat::tests::the_real_table_is_admitted_and_played_without_a_window \
  -- --ignored --exact
ARKHAM_GATHERING_TEST_ASSETS=/absolute/path/to/prepared-collection \
  scripts/dev cargo test --locked --features tabletop --test agent_seat \
  the_real_table_is_held_between_tool_commands_until_quit -- --ignored --exact
```

`ARKHAM_GATHERING_TEST_ASSETS` names the folder that holds the collection's `manifest.json`. **Import artwork** installs it as the `collection` folder of an installation, `${XDG_DATA_HOME:-$HOME/.local/share}/arkham-unfolded/artwork/installation-*/collection`, and records that path as `asset_root` in `${XDG_CONFIG_HOME:-$HOME/.config}/arkham-unfolded/gathering.toml`, as **Choose asset folder** records its folder. Name `collection` itself: the `installation-*` folder above it is refused with a `manifest.json` error. Take the path `asset_root` records, since more than one installation can be present, such as the previous one, kept until a new import is admitted.

## Held session

With `--socket` the seat holds one session on a local socket across separate shell commands. Use it for play and for long checks, where the table's state must last between commands; batch form suits a short confirmation that fits in one command. Start it once, keeping its response in a file and reading it with the same bounded view as batch form:

```sh
target/debug/arkham-tabletop --experiment gathering --agent-seat \
  --save /absolute/path/to/attempt.json --socket /absolute/path/to/seat.sock > ready.jsonl
jq -c '{status, error, revision: .observation.revision, settled: .table.settled,
        pending: .table.pending, modal: .table.modal, menu: .table.menu,
        focused: [(.table.controls[]?, .table.menu.rows[]?) | select(.focused) | {choice, label}]}' \
  ready.jsonl
```

The start command returns as soon as the table is admitted. It prints the same `ready` response as batch form, about 15 KB, and exits successfully, while the seat keeps running as a separate process, detached from the shell and its process group; `jq -c .adapter ready.jsonl` shows the graphics adapter. Until then it blocks: the seat verifies the artwork and builds the table, admission then waits up to 120 seconds for loading and rendering, and the admitted table settles its opening presentation for up to 1,800 steps. The start command sets no time limit of its own, so allow it a few minutes. The seat keeps none of the shell's standard input, output or error, so the start command finishes like any other command. It needs no background-process feature of the shell or agent harness. A startup problem prints one `refused` response and exits unsuccessfully, with no socket left and no seat running. `--socket` needs an absolute path, at most 107 bytes long, and `--agent-seat`. The socket file is accessible only to its owner. When a seat already answers on the path, the start is refused and that seat is left untouched: use it, or quit it first. A socket file where nothing answers, as a crashed or killed seat leaves it, is replaced. Any other existing path is refused and left unchanged.

Then send requests from any later command. Each command sends its requests over one connection, as arguments or as standard-input lines, and prints each response line unchanged. Here one command opens Tony's menu and the next, after reading its rows, chooses from it:

```sh
scripts/tools seat /absolute/path/to/seat.sock \
  '{"op":"activate","revision":0,"component":"investigator:tony"}' > responses.jsonl
printf '%s\n' '{"op":"activate","revision":0,"choice":"lead:tony"}' '{"op":"settle"}' |
  scripts/tools seat /absolute/path/to/seat.sock > responses.jsonl
```

The command exits unsuccessfully when a response is `refused`, `missed` or `skipped`, or when the session ends before every request is answered. When no seat is listening on the path, it says so, names the path and exits unsuccessfully: start a seat first. The `jq` view above reads its output the same way. Each command is one connection for **Stop after a refusal**: a refusal or miss skips only the rest of that command's activations and keys.

Between commands the table keeps its camera, open contextual menu, dialogs, readings and group clue payment chooser, so reading a menu's rows and choosing from it can be two commands. A reading dismissed in a held session stays dismissed. Every committed decision still autosaves, so the attempt stays resumable. Connections are answered one at a time, in arrival order; closing one does not end the session.

The seat's diagnostics, standard error in batch form, go to the socket path with `.log` appended, such as `/absolute/path/to/seat.sock.log`. A start that claims the socket begins that file afresh, even when admission then fails, so the log holds the seat's diagnostics from then on. The launcher's note that it replaced a leftover socket goes to the start command's standard error instead. A start refused before it claims the socket, for a refused option, a seat already answering there, a path that is not a socket or a path too long, leaves an existing log unchanged. Read terminal checks there, such as the modal-dialog warning.

`{"op":"quit"}` answers `bye`, removes the socket and ends the seat; the attempt stays resumable and a later start resumes it. When the table ends the session itself, for example through **Save and return to scenario selection**, the response is `exited` and the socket is removed too.

A seat that receives no request for 30 minutes ends itself as `quit` does: the attempt stays resumable, the socket is removed and the log says that no request arrived. Time the seat spends answering does not count, and an open connection that sends nothing does not keep it. To think longer between requests, start the seat with `--idle-minutes MINUTES`, from 1 to 1440. After the idle end a command finds no seat listening; start again to resume the attempt.

## Hosting an agent partner

A held seat hosts [agent partners](../playing.md#play-with-agent-partners) as the window does. Enable them through the menu: `Tab` to **Menu**, then **Agent partners**, an investigator's `you play` row and **Enable agent partners**, pressing `Enter` naming each in `focused`, then close the menu with `Escape`: while it is open, the command below returns at once. The line to paste is in the seat log, after `Agent partners are enabled.`; give it to the partner's session. Because the seat runs frames only inside its own requests, run this between the seat's own decisions:

```sh
scripts/tools seat-wait /absolute/path/to/seat.sock
```

It answers nothing. It settles the table at least every half second, prints a line whenever the revision, the stage or the awaited investigators change, and returns once the table waits on the seat alone: no **Take over** is shown, or a reading, dialog or menu is open over the table (the [partner instructions](partner-instructions.md#decide) say when the table applies a partner's choice). In a shared decision, such as committing cards to a test, set the seat's part before running it. `--revision N` and `--choices N` also return at a revision or after that many applied choices, and `--timeout SECONDS` (default 240, at most 590) bounds the wait. It exits 0 on one of these conditions, 3 at the timeout and 4 when no seat listens on the socket, from the start or once it has gone; `scripts/tools seat-wait --help` has the rest. A seat run checks the partner protocol and the table's logic; it is not native acceptance.
