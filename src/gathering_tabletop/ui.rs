//! Gathering content and actions in the established shared tabletop screen.
use super::*;
use crate::tabletop_ui as screen;
use bevy::ui_widgets::RequestClose;

pub(super) fn portraits(
    art: &art::Art,
    images: &Assets<Image>,
) -> [Option<(Handle<Image>, Vec2)>; 3] {
    ["06003", "06005", "60401"].map(|code| {
        art.image(&format!("standee/{code}")).and_then(|handle| {
            images
                .get(&handle)
                .map(|image| (handle.clone(), image.size_f32()))
        })
    })
}

fn token_guidance(p: &mut ChildSpawnerCommands, art: &art::Art, table: &Table, view: &Observation) {
    let tokens = view.play.as_ref().unwrap().tokens.as_ref().unwrap();
    if let Some(summary) = view.play.as_ref().and_then(test_summary) {
        p.spawn((
            art.text(summary, 14., PAPER),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ));
    }
    p.spawn((
        art.text(tokens.instruction(), 17., GOLD),
        Node {
            flex_shrink: 0.,
            ..default()
        },
    ));
    if !tokens.revealed.is_empty() {
        p.spawn((
            art.text(
                "Check: selected · slash: ignored · X: canceled · blue ring: available now",
                12.,
                MUTED,
            ),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ));
    }
    if let Some(progress) = tokens.selection_progress(&view.choices) {
        p.spawn((
            art.text(progress, 14., PAPER),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ));
    }
    if !table.notice.is_empty() {
        p.spawn((
            art.text(&table.notice, 14., GOLD),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ));
    }
    p.spawn((
        Node {
            min_height: px(0),
            padding: UiRect::axes(px(0), px(4)),
            flex_direction: FlexDirection::Column,
            row_gap: px(5),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ScrollPosition::default(),
        ScrollablePanel { inspection: false },
    ))
    .observe(super::scroll_panel)
    .with_children(|body| token_options_body(body, art, table, view));
    p.spawn(Node {
        flex_shrink: 0.,
        ..default()
    })
    .with_children(|footer| {
        screen::controls(footer, |row| {
            for choice in &view.choices {
                if matches!(
                    choice.id.as_str(),
                    "tokens:confirm" | "test:reveal" | "undo"
                ) {
                    control(
                        art,
                        row,
                        choice_label(choice),
                        Action::Choose(table.epoch, view.revision, choice.id.clone()),
                    );
                }
            }
        })
    });
}

/// Shown while Olive McBride and Jacqueline can both still be declared.
pub(super) const OLIVE_FIRST_HINT: &str =
    "Use Olive first if you want Jacqueline to affect Olive's three tokens.";

fn token_options_body(
    p: &mut ChildSpawnerCommands,
    art: &art::Art,
    table: &Table,
    view: &Observation,
) {
    let Some(play) = &view.play else {
        return;
    };
    let Some(tokens) = &play.tokens else {
        return;
    };
    // The rules session offers Jacqueline once when her scopes are equivalent.
    // A sole offered choice is shown directly; several share a disclosure.
    let jacqueline: Vec<_> = view
        .choices
        .iter()
        .filter(|choice| choice.id.starts_with("tokens:jacqueline"))
        .collect();
    let direct = jacqueline.len() == 1;
    let olive: Vec<_> = view
        .choices
        .iter()
        .filter(|choice| choice.id.starts_with("tokens:olive:"))
        .collect();
    // A curse opens another declaration while earlier tokens remain held.
    let declaring = view.choices.iter().any(|choice| choice.id == "test:reveal");
    if declaring {
        if let Some(announcement) = reveal::announcement(view) {
            p.spawn(art.text(announcement, 14., PAPER));
        }
        for line in tokens.reveal_plan() {
            p.spawn(art.text(line, 13., MUTED));
        }
        let count = tokens.fresh_reveal_count().map_or_else(
            || "Reveal until the declared condition is met".into(),
            |count| format!("Planned draw: {count} physical token(s)"),
        );
        p.spawn(art.text(count, 14., PAPER));
        p.spawn(art.text(
            "Reveal tokens executes this plan. Use optional abilities before revealing.",
            13.,
            MUTED,
        ));
        if !tokens.is_skill_test()
            && play.players[tokens.revealer.index()]
                .assets
                .iter()
                .any(|asset| asset.card.code == "11069")
        {
            p.spawn(art.text(
                "Katarina Sojka is unavailable: her reaction requires your own skill test.",
                13.,
                MUTED,
            ));
        }
        screen::controls(p, |row| {
            for (prefix, name) in [
                ("tokens:jacqueline", "Jacqueline"),
                ("tokens:prophecy:", "Dark Prophecy"),
                ("tokens:olive:", "Olive McBride"),
                ("tokens:katarina:", "Katarina Sojka"),
            ] {
                if prefix == "tokens:olive:" && olive.len() == 1 {
                    control(
                        art,
                        row,
                        "Use Olive — reveal 3, resolve 2",
                        Action::Choose(table.epoch, view.revision, olive[0].id.clone()),
                    );
                    continue;
                }
                if prefix == "tokens:jacqueline" && direct {
                    control(
                        art,
                        row,
                        choice_label(jacqueline[0]),
                        Action::Choose(table.epoch, view.revision, jacqueline[0].id.clone()),
                    );
                    continue;
                }
                if view
                    .choices
                    .iter()
                    .any(|choice| choice.id.starts_with(prefix))
                {
                    let expanded = table.token_options.as_deref() == Some(prefix);
                    let entity = control(
                        art,
                        row,
                        &format!(
                            "{} Use {name} · {}",
                            if expanded { "▾" } else { "▸" },
                            if expanded { "hide" } else { "show" }
                        ),
                        Action::TokenOptions(prefix.into()),
                    );
                    row.commands()
                        .entity(entity)
                        .insert(feedback::DisclosureButton);
                }
            }
        });
        if !jacqueline.is_empty() && !olive.is_empty() {
            p.spawn(art.text(OLIVE_FIRST_HINT, 13., MUTED));
        }
        if direct || table.token_options.as_deref() == Some("tokens:jacqueline") {
            let activity = if tokens.is_skill_test() {
                "testing"
            } else {
                "revealing tokens"
            };
            let assistance = if tokens.revealer == Investigator::Jacqueline {
                String::new()
            } else {
                " Jacqueline can assist at the same location.".into()
            };
            p.spawn(art.text(
                format!("{} is {activity}.{assistance}", tokens.revealer.name()),
                14.,
                PAPER,
            ));
            p.spawn(art.text("Add 2 tokens, then cancel 2 tokens or only the auto-fail. Tokens are revealed only when you resolve the declared reveal.", 13., MUTED));
            if jacqueline
                .iter()
                .any(|choice| choice.id.contains(":arbiter:"))
            {
                p.spawn(art.text("To use Arbiter this round, exhaust it now and preserve your ordinary use. Spending the ordinary use first leaves no further use to initiate.", 13., MUTED));
            }
        }
    }
    screen::controls(p, |p| {
        for choice in &view.choices {
            if declaring && olive.len() == 1 && choice.id.starts_with("tokens:olive:") {
                continue;
            }
            if declaring && direct && choice.id.starts_with("tokens:jacqueline") {
                continue;
            }
            let visible = if declaring {
                choice.id.starts_with("tokens:eyes:")
                    || table
                        .token_options
                        .as_ref()
                        .is_some_and(|prefix| choice.id.starts_with(prefix))
            } else {
                choice.id.starts_with("tokens:") && choice.id != "tokens:confirm"
            };
            if visible {
                control(
                    art,
                    p,
                    choice_label(choice),
                    Action::Choose(table.epoch, view.revision, choice.id.clone()),
                );
            }
        }
    });
}

fn short_name(who: Investigator) -> &'static str {
    who.name().split_whitespace().next().unwrap_or(who.name())
}

pub(super) fn choice_label(choice: &crate::Choice) -> &str {
    if choice.id == "undo" {
        "Undo last choice"
    } else {
        &choice.label
    }
}

fn turn_heading(view: &Observation) -> String {
    if let Some(play) = &view.play {
        if play.scenario.outcome.is_some() {
            return "Scenario ended".into();
        }
        if let Some(who) = play.actor {
            return format!("{}'s turn", short_name(who));
        }
        let eligible = Investigator::ALL
            .into_iter()
            .filter(|who| {
                view.choices
                    .iter()
                    .any(|choice| choice.id == format!("turn:{}", who.id()))
            })
            .map(short_name)
            .collect::<Vec<_>>();
        if !eligible.is_empty() {
            return format!("Choose the next investigator · {}", eligible.join(", "));
        }
        return format!("Round {} · {}", play.round, phase_banner::name(play.phase));
    }
    "Setup · no investigator turn in progress".into()
}

fn test_summary(play: &crate::gathering::PlayObservation) -> Option<String> {
    use crate::{gathering::Skill, tabletop_support::symbols::Skill as Symbol};
    let test = play.test.as_ref()?;
    let skill = match test.skill {
        Skill::Willpower => Symbol::Willpower,
        Skill::Intellect => Symbol::Intellect,
        Skill::Combat => Symbol::Combat,
        Skill::Agility => Symbol::Agility,
    };
    let preview = match test.would_succeed {
        Some(true) => "Would succeed",
        Some(false) => "Would fail",
        None => "Pending",
    };
    let summary = crate::tabletop_skill_text::TestSummary {
        skill,
        base: test.base_skill,
        card_bonus: test.modifiers.cards,
        chaos: test.modifiers.chaos,
        chaos_pending: !test.resolved && (play.tokens.is_some() || test.modifiers.chaos.is_none()),
        automatic_failure: test.modifiers.automatic_failure,
        total: test.total,
        difficulty: test.difficulty,
        result: (test.resolved).then_some(test.would_succeed).flatten(),
    }
    .label_with_pending(preview);
    let mut label = format!("{} · {summary}", results::attempt(test));
    if test.modifiers.other != 0 {
        label.push_str(&format!(
            "\nOther bonuses {:+} are included in the value.",
            test.modifiers.other
        ));
    }
    Some(label)
}

fn selected_commitments(view: &Observation) -> Option<String> {
    let play = view.play.as_ref()?;
    let test = play.test.as_ref()?;
    let selected = test
        .commitment
        .iter()
        .filter(|card| card.selected)
        .filter_map(|selected| {
            let card = play
                .players
                .iter()
                .find(|player| player.investigator == selected.owner)?
                .hand
                .iter()
                .find(|card| card.id == selected.card_id)?;
            let wager = if card.code == "03233" {
                selected.wager.map_or_else(
                    || " · Choose a wager".into(),
                    |amount| format!(" · Wager: {amount} resources"),
                )
            } else {
                String::new()
            };
            Some(format!(
                "{}'s {} ({:+}){wager}",
                short_name(selected.owner),
                card.title,
                selected.contribution
            ))
        })
        .collect::<Vec<_>>();
    (!selected.is_empty())
        .then(|| format!("Selected: {}. Select again to remove.", selected.join(", ")))
}

fn wager_guidance(p: &mut ChildSpawnerCommands, art: &art::Art, table: &Table, view: &Observation) {
    p.spawn(art.text(
        "Watch this! — choose a wager for each selected copy.",
        16.,
        GOLD,
    ));
    p.spawn(art.text("0 commits icons only. Pay when confirming; succeed by at least 1 to gain twice your wager.", 13., PAPER));
    let play = view.play.as_ref().expect("Commitment");
    let test = play.test.as_ref().expect("Skill test");
    for selected in test.commitment.iter().filter(|card| card.selected) {
        let prefix = format!("tony:watch:{}:", selected.card_id);
        let choices: Vec<_> = view
            .choices
            .iter()
            .filter(|choice| choice.id.starts_with(&prefix))
            .collect();
        let Some(first) = choices.first() else {
            continue;
        };
        let title = first.label.split(" — ").next().unwrap_or("Watch this!");
        let status = selected.wager.map_or_else(
            || "Choose a wager".into(),
            |amount| format!("Wager: {amount} resources"),
        );
        screen::controls(p, |row| {
            row.spawn((
                art.text(format!("{title}\n{status}"), 13., GOLD),
                Node {
                    width: px(225),
                    ..default()
                },
            ));
            for choice in choices {
                let amount = choice.id.rsplit(':').next().expect("Wager amount");
                let label = if amount == "0" {
                    "0 · icons only".to_owned()
                } else {
                    format!("Wager {amount}")
                };
                row.spawn(Node {
                    width: px(145),
                    ..default()
                })
                .with_children(|slot| {
                    let entity = button(
                        art,
                        slot,
                        &label,
                        Action::Choose(table.epoch, view.revision, choice.id.clone()),
                    );
                    slot.commands()
                        .entity(entity)
                        .insert(AccessibleLabel::new(choice.label.clone()));
                });
            }
        });
    }
    screen::controls(p, |row| {
        for choice in controls::global_choices(view) {
            control(
                art,
                row,
                choice_label(&choice),
                Action::Choose(table.epoch, view.revision, choice.id.clone()),
            );
        }
    });
    if let Some(summary) = selected_commitments(view) {
        p.spawn(art.text(summary, 12., MUTED));
    }
}

pub(super) fn button(
    art: &art::Art,
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: Action,
) -> Entity {
    let entity = screen::button(parent, art.symbol_font(), label);
    if let Some(id) = action.choice() {
        parent
            .commands()
            .entity(entity)
            .insert(ChoiceControl(id.to_owned()));
    }
    parent.commands().entity(entity).observe(
        move |_: On<Activate>, mut actions: ResMut<Actions>| actions.0.push(action.clone()),
    );
    entity
}

pub(super) fn control(
    art: &art::Art,
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: Action,
) -> Entity {
    let mut entity = None;
    screen::control_slot(parent, |slot| {
        entity = Some(button(art, slot, label, action));
    });
    entity.expect("A control slot always contains its button")
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    commands: &mut Commands,
    table: &Table,
    view: &Observation,
    art: &art::Art,
    images: &Assets<Image>,
    orbit: &Orbit,
    health: &RenderHealth,
    size: Vec2,
    context: &CardRender,
    reveal_open: bool,
) {
    screen::header(commands, Panel, |p| {
        p.spawn(art.text(
            if view.play.is_some() {
                "The Gathering · Standard"
            } else {
                "The Gathering · Standard · Setup"
            },
            20.,
            GOLD,
        ));
        let prefix = { "" };
        p.spawn(art.text(format!("{prefix}{}", turn_heading(view)), 18., PAPER));
    });
    // Tab reaches the Menu after navigation (0), decisions (1) and the sideboard headings (2).
    screen::menu_button(commands, (Panel, TabGroup::new(3)), |p| {
        button(art, p, "Menu", Action::Utilities);
    });
    let portraits = portraits(art, images);
    screen::navigation(commands, (Panel, TabGroup::new(0)), |p| {
        for (index, who) in Investigator::ALL.into_iter().enumerate() {
            let current_turn = view
                .play
                .as_ref()
                .is_some_and(|play| play.scenario.outcome.is_none() && play.actor == Some(who));
            let mut marks = Vec::new();
            if table.partners.partner_plays(who) {
                marks.push("PARTNER".to_owned());
            }
            if let Some((deciding, state)) = table.partners.deciding
                && deciding == who
            {
                marks.push(match state {
                    partners::Deciding::Thinking => "THINKING".to_owned(),
                    partners::Deciding::Silent(minutes) => format!("SILENT {minutes} MIN"),
                    partners::Deciding::Unclaimed => "AWAITING PARTNER".to_owned(),
                });
            }
            let entity = screen::portrait(
                p,
                art.symbol_font(),
                who.name(),
                portraits[index].as_ref(),
                current_turn,
                &marks,
            );
            p.commands().entity(entity).observe(
                move |_: On<Activate>, mut queue: ResMut<Actions>| {
                    queue.0.push(Action::Focus(Area::Investigator(who.index())));
                },
            );
        }
        for (name, area) in [
            ("Scenario", Area::Scenario),
            ("Test", Area::Test),
            ("Overview", Area::Overview),
        ] {
            let entity = button(art, p, name, Action::Focus(area));
            if area == Area::Overview
                && !table.modal()
                && !table.menu_open
                && !table.context.is_open()
                && !reveal_open
                && controls::global_choices(view).is_empty()
            {
                p.commands().entity(entity).insert(AutoFocus);
            }
        }
        if view
            .play
            .as_ref()
            .and_then(trays::Layout::for_play)
            .is_some()
        {
            button(art, p, "Resolve", Action::Resolve);
        }
        p.spawn(art.text("WASD: pan\nRight drag: orbit\nWheel: zoom\nHome: overview\n\nFocus does not select who acts.", 11., MUTED));
    });
    let token_decision = !table.is_presenting()
        && !table.modal()
        && view.play.as_ref().is_some_and(|play| play.tokens.is_some())
        && view
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("tokens:") || choice.id == "test:reveal");
    let wager_decision = !table.is_presenting()
        && !table.modal()
        && view
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("tony:watch:"));
    let decision_panel = screen::decision_panel(
        commands,
        (
            Panel,
            TabGroup::new(1),
            ScrollPosition::default(),
            ScrollablePanel { inspection: false },
        ),
        size,
        |p| {
            if !health.allows_play() {
                p.spawn(art.text(health.status(), 20., GOLD));
                button(art, p, "Choose asset folder", Action::AssetFolder);
                return;
            }
            if token_decision {
                token_guidance(p, art, table, view);
                return;
            }
            if wager_decision {
                wager_guidance(p, art, table, view);
                return;
            }
            p.spawn(art.text("LIVE DECISION", 12., GOLD));
            if view.play.is_none()
                && let Some(owner) = view.decision_owner
            {
                p.spawn(art.text(format!("{}'s decision", owner.name()), 16., GOLD));
            }
            // During ordinary play the phase and pending choice below already
            // describe what to do. Keep setup, ending and refusal explanations.
            if view
                .play
                .as_ref()
                .is_none_or(|play| play.scenario.outcome.is_some())
                || view.choices.iter().all(|choice| choice.id == "undo")
                || view.verification_fixture.is_some()
            {
                p.spawn(art.text(play::location_text(&view.prompt), 18., PAPER));
            }
            if let Some(status) = play::status(view) {
                p.spawn(art.text(status, 15., GOLD));
            }
            if let Some(summary) = view.play.as_ref().and_then(test_summary) {
                p.spawn(art.text(summary, 15., GOLD));
            }
            if let Some((who, _)) = table.partners.deciding
                && let Some(cue) = table.partners.cue(who)
            {
                p.spawn(art.text(cue, 16., PARTNER));
                screen::controls(p, |row| {
                    control(
                        art,
                        row,
                        &format!("Take over {}", who.name()),
                        Action::Partners(partners::Input::TakeOver(who)),
                    );
                });
            }
            if !table.is_presenting() {
                for (_, label) in controls::window_opportunities(view) {
                    p.spawn(art.text(
                        format!("Available now: {label}"),
                        15.,
                        Color::srgb(0.78, 0.65, 0.95),
                    ));
                }
            }
            screen::controls(p, |row| {
                if !table.is_presenting() && !table.modal() {
                    for (index, choice) in controls::global_choices(view).into_iter().enumerate() {
                        let entity = control(
                            art,
                            row,
                            choice_label(&choice),
                            Action::Choose(table.epoch, view.revision, choice.id.clone()),
                        );
                        // While a partner decides, Enter must not undo the human's
                        // own latest decision under it.
                        let undo_under_partner =
                            choice.id == "undo" && table.partners.deciding.is_some();
                        if index == 0
                            && !table.menu_open
                            && !table.context.is_open()
                            && !reveal_open
                            && !undo_under_partner
                        {
                            row.commands().entity(entity).insert(AutoFocus);
                        }
                    }
                    if !controls::pending_choices(view).is_empty() {
                        control(art, row, "Pending decision", Action::PendingChoices);
                    }
                }
            });
            if let Some(selected) = selected_commitments(view) {
                p.spawn(art.text(selected, 13., GOLD));
            }
            if view.play.is_none() && !table.is_presenting() && !view.choices.is_empty() {
                p.spawn(art.text("Click a card or table component for its available actions. Portrait buttons only move the camera.", 12., MUTED));
            }
            if !table.notice.is_empty() {
                p.spawn(art.text(&table.notice, 12., MUTED));
            }
        },
    );
    if token_decision || wager_decision {
        commands.entity(decision_panel).insert(Node {
            position_type: PositionType::Absolute,
            left: px(150),
            right: px(12),
            bottom: px(10),
            max_height: px((size.y * 0.4).min(330.)),
            padding: UiRect::all(px(12)),
            flex_direction: FlexDirection::Column,
            row_gap: px(5),
            overflow: Overflow::clip(),
            ..default()
        });
    }
    if table.menu_open {
        screen::utilities(commands, (Panel, TabGroup::modal()), size, |p| {
            let first = button(art, p, "Action history and rules reasons", Action::History);
            p.commands().entity(first).insert(AutoFocus);
            button(
                art,
                p,
                if table.reduced_motion {
                    "Motion: reduced"
                } else {
                    "Motion: animated"
                },
                Action::Motion,
            );
            {
                button(art, p, "Save live attempt", Action::Save);
                button(art, p, "Reopen saved attempt", Action::Reopen);
                button(
                    art,
                    p,
                    "Save and return to scenario selection",
                    Action::ReturnToSelection,
                );
                button(
                    art,
                    p,
                    "Restart The Gathering...",
                    Action::Restart(table.epoch),
                );
            }
            button(
                art,
                p,
                if table.partners.menu_open {
                    "Hide agent partners"
                } else {
                    "Agent partners"
                },
                Action::Partners(partners::Input::Menu),
            );
            if table.partners.menu_open {
                partners_menu(p, art, table);
            }
            button(
                art,
                p,
                if orbit.is_following() {
                    "Camera following: on"
                } else {
                    "Resume camera following"
                },
                Action::Follow,
            );
            p.spawn(art.text(art.status(), 12., MUTED));
            button(art, p, "Choose asset folder", Action::AssetFolder);
            button(
                art,
                p,
                "Credits",
                Action::Inspect("Credits".into(), "credits".into()),
            );
            button(art, p, "Close menu", Action::Dismiss);
        });
    }
    if table.presentation.result().is_some() {
        results::render(commands, table, view, art, size);
    } else if table.prediction.is_some() {
        pendulum::render(commands, table, art);
    } else if table.restart_confirmation {
        restart_confirmation(commands, table, art, size);
    } else if let Some(card) = table.presentation.story() {
        story_reading(commands, art, size, card);
    } else if table.presentation.encounter().current().is_some() {
        encounter::render(commands, table, art, size);
    } else if table.result_opportunities() {
        results::render(commands, table, view, art, size);
    } else if table.inspection.is_some() || table.history_open {
        inspection(commands, table, view, art, size, context);
    }
}

/// Shown under the Agent partners button of the menu (#205).
fn partners_menu(p: &mut ChildSpawnerCommands, art: &art::Art, table: &Table) {
    let shown = &table.partners;
    p.spawn(art.text(
        "Hand investigators to agent partners for this session. The save does not record it.",
        12.,
        MUTED,
    ));
    for who in Investigator::ALL {
        let label = match (shown.selected[who.index()], shown.joined[who.index()]) {
            (false, _) => format!("{}: you play", who.name()),
            (true, false) => format!("{}: a partner plays", who.name()),
            (true, true) => format!("{}: a partner plays · joined", who.name()),
        };
        button(art, p, &label, Action::Partners(partners::Input::Hand(who)));
    }
    button(
        art,
        p,
        if shown.enabled {
            "Turn agent partners off"
        } else {
            "Enable agent partners"
        },
        Action::Partners(partners::Input::Enable(!shown.enabled)),
    );
    if let Some(problem) = &shown.problem {
        p.spawn(art.text(problem, 12., GOLD));
    }
    if let Some(line) = &shown.line {
        p.spawn(art.text(
            "Paste into an agent's terminal (the game's terminal shows it too):",
            12.,
            MUTED,
        ));
        p.spawn(art.text(line, 13., PAPER));
    }
}

/// Agent partners' cue in the decision display.
const PARTNER: Color = Color::srgb(0.55, 0.82, 0.86);

fn restart_confirmation(commands: &mut Commands, table: &Table, art: &art::Art, size: Vec2) {
    let barrier = screen::inspection(
        commands,
        Panel,
        (),
        art.symbol_font(),
        size,
        "Restart The Gathering?",
        |_| {},
        |body, _| {
            body.spawn((
                art.text(
                    "Start again with fresh shuffles, lead selection and mulligans. Your selected decks and rules stay the same. The current attempt will be archived; Reopen saved attempt will resume the new attempt.",
                    20., PAPER,
                ),
                Node { width: percent(100), ..default() },
            ));
            if !table.notice.is_empty() {
                body.spawn(art.text(&table.notice, 18., GOLD));
            }
            screen::controls(body, |row| {
                let cancel = control(art, row, "Cancel", Action::Close);
                row.commands().entity(cancel).insert(AutoFocus);
                control(art, row, "Restart", Action::ConfirmRestart(table.epoch));
            });
        },
    );
    commands
        .entity(barrier)
        .observe(|_: On<RequestClose>, mut actions: ResMut<Actions>| actions.0.push(Action::Close));
}

fn story_reading(commands: &mut Commands, art: &art::Art, size: Vec2, card: story::StoryCard) {
    let kind = match card.side() {
        layout::ScenarioBookSide::Agenda => "Agenda advances",
        layout::ScenarioBookSide::Act => "Act advances",
    };
    let width = (size.x * 0.94).min(1200.);
    let image = art.card(card.code(), true);
    let columns = width >= 900.;
    let content_width = width - 56.;
    reading::panel(
        commands,
        size,
        width,
        card.title(),
        |panel| {
            panel.spawn(art.text(kind, 16., MUTED));
            panel.spawn(art.text(card.title(), 27., GOLD));
        },
        |body| {
            body.spawn(Node {
                flex_direction: if columns {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
                flex_shrink: 0.,
                column_gap: px(24),
                row_gap: px(16),
                ..default()
            })
            .with_children(|body| {
                if let Some(image) = image {
                    let image_width = if columns {
                        (content_width - 24.) * 0.45
                    } else {
                        content_width
                    };
                    inspection_image(body, image, card.code(), image_width);
                }
                body.spawn(Node {
                    width: if columns {
                        px((content_width - 24.) * 0.55)
                    } else {
                        percent(100)
                    },
                    flex_shrink: 0.,
                    ..default()
                })
                .with_children(|text| {
                    if let Some(rules) =
                        crate::gathering::inspection::scenario_card_reverse(card.code())
                    {
                        text.spawn(art.text(rules, 19., PAPER));
                    }
                });
            });
        },
        |panel| {
            panel
                .spawn(Node {
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|row| {
                    let first = control(art, row, "Continue", Action::ContinueStory);
                    row.commands().entity(first).insert(AutoFocus);
                });
        },
    );
}

fn inspection(
    commands: &mut Commands,
    table: &Table,
    view: &Observation,
    art: &art::Art,
    size: Vec2,
    context: &CardRender,
) {
    let title = table
        .inspection
        .as_ref()
        .map_or("Action history", |(title, _)| title.as_str());
    let barrier = screen::inspection(
        commands,
        Panel,
        ScrollablePanel { inspection: true },
        art.symbol_font(),
        size,
        title,
        |p| {
            let close = button(art, p, "Close · Esc", Action::Close);
            p.commands().entity(close).insert(AutoFocus);
        },
        |body, width| {
            if let Some((_, code)) = &table.inspection {
                if code == "credits" {
                    body.spawn(art.text("Icon artwork by Lorc · CC BY 3.0", 22., PAPER));
                    body.spawn(art.text("Chalice drops: artwork unchanged. The slate-blue shield is application geometry.\nSwap bag: PNG unchanged; its white silhouette may be tinted in presentation.", 18., PAPER));
                    for (title, url) in [
                        (
                            "Chalice drops — source",
                            "https://game-icons.net/1x1/lorc/chalice-drops.html",
                        ),
                        (
                            "Swap bag — source",
                            "https://game-icons.net/1x1/lorc/swap-bag.html",
                        ),
                        (
                            "CC BY 3.0 — license",
                            "https://creativecommons.org/licenses/by/3.0/",
                        ),
                    ] {
                        button(art, body, title, Action::CreditLink(url));
                        body.spawn(art.text(url, 14., MUTED));
                    }
                    return;
                }
                if let Some(image) = context.image(code) {
                    inspection_image(body, image, code, width);
                }
                if code.len() == 5 && code.bytes().all(|byte| byte.is_ascii_digit()) {
                    body.spawn((
                        art.text(format!("Card {code}"), 18., GOLD),
                        Node {
                            width: percent(100),
                            ..default()
                        },
                    ));
                }
                if let Some(rules) = context.rules(code) {
                    body.spawn((
                        art.text(rules, 20., PAPER),
                        Node {
                            width: percent(100),
                            ..default()
                        },
                    ));
                }
                if let Some(current) = story::StoryCard::from_code(code) {
                    for previous in context
                        .story
                        .known_reverses(current.side())
                        .filter(|card| *card != current)
                    {
                        body.spawn(art.text(
                            format!("Previously revealed · {}", previous.title()),
                            22.,
                            GOLD,
                        ));
                        if let Some(image) = art.card(previous.code(), true) {
                            inspection_image(body, image, previous.code(), width);
                        }
                        if let Some(rules) =
                            crate::gathering::inspection::scenario_card_reverse(previous.code())
                        {
                            body.spawn((
                                art.text(rules, 20., PAPER),
                                Node {
                                    width: percent(100),
                                    ..default()
                                },
                            ));
                        }
                    }
                }
            } else {
                if let Some(play) = &view.play {
                    for (index, report) in play.completed_tests.iter().enumerate().rev() {
                        body.spawn(art.text(
                            format!(
                                "Test {} · {} · {} · {} vs {} · {}",
                                index + 1,
                                report.tester.name(),
                                report.source,
                                report.total,
                                report.difficulty,
                                results::outcome(report)
                            ),
                            22.,
                            GOLD,
                        ));
                        results::details(body, art, report);
                    }
                }
                for (index, line) in view.history.iter().enumerate() {
                    let line = table
                        .partners
                        .history_line(index, controls::display_history(view, line));
                    body.spawn((
                        art.text(line, 18., PAPER),
                        Node {
                            width: percent(100),
                            ..default()
                        },
                    ));
                }
            }
        },
    );
    commands
        .entity(barrier)
        .observe(|_: On<RequestClose>, mut actions: ResMut<Actions>| actions.0.push(Action::Close));
}

fn inspection_image(
    parent: &mut ChildSpawnerCommands,
    image: Handle<Image>,
    code: &str,
    width: f32,
) {
    let landscape = matches!(
        code,
        "06003" | "06005" | "60401" | "01105" | "01106" | "01107" | "01108" | "01109" | "01110"
    );
    screen::card_image(parent, image, landscape, width);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ui_widgets::{ModalDialog, ModalDialogBarrier};

    fn button_named(app: &mut App, name: &str) -> Entity {
        let world = app.world_mut();
        world
            .query_filtered::<(Entity, &AccessibleLabel), With<Button>>()
            .iter(world)
            .find_map(|(entity, label)| (label.0 == name).then_some(entity))
            .unwrap_or_else(|| panic!("Missing button {name}"))
    }

    fn choose(app: &mut App, id: &str) {
        let table = app.world().resource::<Table>();
        let view = table.view();
        assert!(
            view.choices.iter().any(|choice| choice.id == id),
            "Missing choice {id}"
        );
        let action = Action::Choose(table.epoch, view.revision, id.to_owned());
        app.world_mut().resource_mut::<Actions>().0.push(action);
        app.update();
        super::super::tests::finish_presentation(app);
    }

    #[test]
    fn production_screen_keeps_party_portrait_navigation_and_utility_menu() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = super::super::tests::host(directory.path().join("screen.json"));
        let before = app.world().resource::<Table>().session.clone();
        let portrait = button_named(&mut app, "Focus Tony Morgan");
        let rail = app.world().get::<ChildOf>(portrait).unwrap().parent();
        let rail_node = app.world().get::<Node>(rail).unwrap();
        assert_eq!(
            (
                rail_node.left,
                rail_node.top,
                rail_node.bottom,
                rail_node.width
            ),
            (px(10), px(102), px(220), px(124)),
            "Party's established navigation rail is the layout oracle"
        );
        app.world_mut().trigger(Activate { entity: portrait });
        app.update();
        assert_eq!(app.world().resource::<Table>().session.clone(), before);

        let menu = button_named(&mut app, "Menu");
        let menu_root = app.world().get::<ChildOf>(menu).unwrap().parent();
        let menu_node = app.world().get::<Node>(menu_root).unwrap();
        assert_eq!(
            (menu_node.right, menu_node.top, menu_node.width),
            (px(12), px(12), px(108))
        );
        app.world_mut().trigger(Activate { entity: menu });
        app.update();
        for label in [
            "Action history and rules reasons",
            "Motion: animated",
            "Save live attempt",
            "Reopen saved attempt",
            "Choose asset folder",
            "Close menu",
        ] {
            button_named(&mut app, label);
        }
        let close = button_named(&mut app, "Close menu");
        app.world_mut().trigger(Activate { entity: close });
        app.update();
        assert!(!app.world().resource::<Table>().menu_open);
        assert_eq!(app.world().resource::<Table>().session.clone(), before);
    }

    #[test]
    fn production_inspection_keeps_close_outside_scroll_and_captures_the_screen() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = super::super::tests::host(directory.path().join("inspection.json"));
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::History);
        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<Entity, With<ModalDialogBarrier>>()
                .iter(world)
                .count(),
            1
        );
        let dialog = world
            .query_filtered::<Entity, With<ModalDialog>>()
            .single(world)
            .unwrap();
        let body = world
            .query::<(Entity, &ScrollablePanel)>()
            .iter(world)
            .find_map(|(entity, panel)| panel.inspection.then_some(entity))
            .unwrap();
        assert_eq!(world.get::<ChildOf>(body).unwrap().parent(), dialog);
        let close = button_named(&mut app, "Close · Esc");
        assert_eq!(
            app.world().get::<ChildOf>(close).unwrap().parent(),
            dialog,
            "Close is a sibling of the scrolling body, so long rules cannot scroll it away"
        );
        let revision = app.world().resource::<Table>().session.observe().revision;
        app.world_mut().trigger(Activate { entity: close });
        app.update();
        assert!(!app.world().resource::<Table>().modal());
        assert_eq!(
            app.world().resource::<Table>().session.observe().revision,
            revision
        );
    }

    #[test]
    fn act_and_agenda_readings_fit_content_and_keep_continue_outside_long_body() {
        use bevy::ecs::system::RunSystemOnce;
        for card in [story::StoryCard::ActOne, story::StoryCard::AgendaTwo] {
            for (size, scale, artwork) in [
                (Vec2::new(1600., 1000.), 1., false),
                (Vec2::new(1600., 1000.), 1., true),
                (Vec2::new(960., 540.), 1.25, true),
                // One column: the artwork above the text cannot fit, so the body scrolls.
                (Vec2::new(640., 480.), 1.25, true),
            ] {
                let directory = tempfile::tempdir().unwrap();
                let mut app = super::super::tests::host(directory.path().join("story.json"));
                crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
                    &mut app, size, scale,
                );
                app.update();
                let panels = app
                    .world_mut()
                    .query_filtered::<Entity, With<Panel>>()
                    .iter(app.world())
                    .collect::<Vec<_>>();
                for panel in panels {
                    app.world_mut().despawn(panel);
                }
                if artwork {
                    let image = app
                        .world_mut()
                        .resource_mut::<Assets<Image>>()
                        .add(Image::default());
                    let role = if card == story::StoryCard::ActOne {
                        "card-01108-back"
                    } else {
                        "card-01106-back"
                    };
                    app.insert_resource(art::Art::with_images_for_test([(role, image)]));
                }
                app.world_mut().resource_mut::<Table>().dirty = false;
                app.world_mut()
                    .run_system_once(move |mut commands: Commands, art: Res<art::Art>| {
                        story_reading(&mut commands, &art, size, card)
                    })
                    .unwrap();
                for _ in 0..4 {
                    app.update();
                }
                let button = button_named(&mut app, "Continue");
                let world = app.world_mut();
                let dialog = world
                    .query_filtered::<Entity, With<ModalDialog>>()
                    .single(world)
                    .unwrap();
                let height = world.get::<ComputedNode>(dialog).unwrap().size().y / scale;
                assert!(height <= size.y * 0.9 + 1., "{card:?}: {height}");
                if size.y == 1000. {
                    assert!(height < 800., "Short story must fit its content: {height}");
                }
                let center = world.get::<UiGlobalTransform>(dialog).unwrap().translation;
                let button_center = world.get::<UiGlobalTransform>(button).unwrap().translation;
                assert!(
                    (center.x - button_center.x).abs() <= scale,
                    "{card:?} at {size:?}, {scale}x, artwork {artwork}: dialog {center:?}, Continue {button_center:?}"
                );
                let (body, node) = world
                    .query::<(Entity, &ComputedNode, &ScrollablePanel)>()
                    .iter(world)
                    .find_map(|(entity, node, panel)| panel.inspection.then_some((entity, node)))
                    .unwrap();
                if size.y == 480. {
                    assert!(
                        node.content_size().y > node.size().y,
                        "Narrow story scrolls"
                    );
                }
                let mut ancestor = button;
                while ancestor != dialog {
                    assert_ne!(ancestor, body, "Continue must not scroll with the body");
                    ancestor = world.get::<ChildOf>(ancestor).unwrap().parent();
                }
                super::super::tests::assert_unclipped_button(&mut app, "Continue", size, scale);
            }
        }
    }

    #[test]
    fn production_portrait_uses_prepared_art_and_turn_tracks_actor_not_camera_focus() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = super::super::tests::host(directory.path().join("portrait-turn.json"));
        let image = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        app.insert_resource(art::Art::with_images_for_test([(
            "standee/06003",
            image.clone(),
        )]));
        app.update();
        let world = app.world_mut();
        assert!(
            !world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0 == "TURN"),
            "Setup decision ownership is not an investigator turn"
        );
        choose(&mut app, "lead:tony");
        for _ in 0..3 {
            choose(&mut app, "confirm-mulligan");
        }
        choose(&mut app, "turn:tony");
        let portrait = button_named(&mut app, "Focus Tony Morgan · current turn");
        let children = app.world().get::<Children>(portrait).unwrap();
        assert!(children.iter().any(|child| {
            app.world()
                .get::<ImageNode>(child)
                .is_some_and(|node| node.image == image)
        }));
        let world = app.world_mut();
        assert_eq!(
            world
                .query::<&Text>()
                .iter(world)
                .filter(|text| text.0 == "TURN")
                .count(),
            1
        );
        let before = app.world().resource::<Table>().session.clone();
        let patrice = button_named(&mut app, "Focus Patrice Hathaway");
        app.world_mut().trigger(Activate { entity: patrice });
        app.update();
        button_named(&mut app, "Focus Tony Morgan · current turn");
        assert_eq!(app.world().resource::<Table>().session.clone(), before);
    }

    /// A scrolling body that is not squeezed by its dialog or its own cap shows all of
    /// its content, so it must report no height to scroll through.
    fn assert_fitting_bodies_do_not_scroll(app: &mut App, size: Vec2, label: &str) -> usize {
        let world = app.world_mut();
        let dialog = world
            .query_filtered::<Entity, With<ModalDialog>>()
            .single(world)
            .unwrap();
        let dialog_height = world.get::<ComputedNode>(dialog).unwrap().size().y;
        let images = world
            .query_filtered::<&ChildOf, With<ImageNode>>()
            .iter(world)
            .map(ChildOf::parent)
            .collect::<Vec<_>>();
        let mut fitting = 0;
        for (body, node, layout) in world
            .query::<(Entity, &ComputedNode, &Node)>()
            .iter(world)
            .filter(|(entity, _, _)| {
                world
                    .get::<ScrollablePanel>(*entity)
                    .is_some_and(|panel| panel.inspection)
            })
        {
            let mut framed = false;
            for frame in &images {
                let mut ancestor = Some(*frame);
                while let Some(entity) = ancestor {
                    framed |= entity == body;
                    ancestor = world.get::<ChildOf>(entity).map(ChildOf::parent);
                }
            }
            assert!(framed, "{label}: the body shows a card image");
            let capped = match layout.max_height {
                Val::Px(cap) => node.size().y >= cap - 1.,
                _ => false,
            };
            if dialog_height < size.y * 0.9 - 1. && !capped {
                fitting += 1;
                assert!(
                    node.content_size().y <= node.size().y + 0.5,
                    "{label} at {size:?}: content {} in a fitting body of {}",
                    node.content_size().y,
                    node.size().y
                );
            }
        }
        fitting
    }

    #[test]
    fn card_image_frames_add_no_scrollable_height_to_reading_or_inspection() {
        use bevy::ecs::system::RunSystemOnce;
        let sizes = [Vec2::new(1600., 1000.), Vec2::new(1920., 1200.)];
        let image = |app: &mut App, role: &'static str| {
            let image = app
                .world_mut()
                .resource_mut::<Assets<Image>>()
                .add(Image::default());
            app.insert_resource(art::Art::with_images_for_test([(role, image)]));
            app.world_mut().resource_mut::<Table>().dirty = true;
        };
        let settle = |app: &mut App, size: Vec2| {
            crate::tabletop_layout::heading_checks::viewport::<MainCamera>(app, size, 1.);
            for _ in 0..4 {
                app.update();
            }
        };

        // A portrait encounter card in its reading stop.
        let directory = tempfile::tempdir().unwrap();
        let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Harm(
            crate::gathering::HarmFixture::WardCancellation,
        ));
        let mut app = super::super::tests::host_with_table(
            Table::new(session, directory.path().join("encounter.json")),
            false,
        );
        image(&mut app, "card-01163");
        let mut fitting = 0;
        for size in sizes {
            settle(&mut app, size);
            fitting += assert_fitting_bodies_do_not_scroll(&mut app, size, "Encounter reading");
        }
        assert!(fitting > 0, "An encounter reading fits its content");

        // A landscape investigator card, rotated inside its frame, in inspection.
        let mut app = super::super::tests::host(directory.path().join("inspection.json"));
        image(&mut app, "card-06003");
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::Inspect("Tony Morgan".into(), "06003".into()));
        let mut fitting = 0;
        for size in sizes {
            settle(&mut app, size);
            fitting += assert_fitting_bodies_do_not_scroll(&mut app, size, "Inspection");
        }
        assert!(fitting > 0, "An inspection fits its content");

        // An act's reverse in its story reading.
        let mut fitting = 0;
        for size in sizes {
            let mut app = super::super::tests::host(directory.path().join("story.json"));
            image(&mut app, "card-01108-back");
            settle(&mut app, size);
            let panels = app
                .world_mut()
                .query_filtered::<Entity, With<Panel>>()
                .iter(app.world())
                .collect::<Vec<_>>();
            for panel in panels {
                app.world_mut().despawn(panel);
            }
            app.world_mut().resource_mut::<Table>().dirty = false;
            app.world_mut()
                .run_system_once(move |mut commands: Commands, art: Res<art::Art>| {
                    story_reading(&mut commands, &art, size, story::StoryCard::ActOne)
                })
                .unwrap();
            settle(&mut app, size);
            fitting += assert_fitting_bodies_do_not_scroll(&mut app, size, "Story reading");
        }
        assert!(fitting > 0, "A story reading fits its content");
    }
}
