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
    }
    "Setup · no investigator turn in progress".into()
}

fn investigator_summary(view: &Observation) -> String {
    if let Some(play) = &view.play {
        return play
            .players
            .iter()
            .map(|player| {
                format!(
                    "{}: {} resources · {} clues · {} actions{}",
                    short_name(player.investigator),
                    player.resources,
                    player.clues,
                    play.actions_remaining[player.investigator.index()],
                    if player.presentation.turn_finished {
                        " (turn ended)"
                    } else {
                        ""
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("     |     ");
    }
    view.investigators
        .iter()
        .map(|player| {
            format!(
                "{}: {} resources · {} cards in hand{}",
                short_name(player.investigator),
                player.resources,
                player.hand.len(),
                if player.mulligan_complete {
                    " · mulligan finished"
                } else {
                    ""
                }
            )
        })
        .collect::<Vec<_>>()
        .join("     |     ")
}

fn test_summary(play: &crate::gathering::PlayObservation) -> Option<String> {
    use crate::{gathering::Skill, tabletop::symbols::Skill as Symbol};
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
    let mut label = format!("{} · {summary}", test.tester.name());
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
            Some(format!(
                "{}'s {} ({:+})",
                short_name(selected.owner),
                card.title,
                selected.contribution
            ))
        })
        .collect::<Vec<_>>();
    (!selected.is_empty())
        .then(|| format!("Selected: {}. Select again to remove.", selected.join(", ")))
}

pub(super) fn button(
    art: &art::Art,
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: Action,
) -> Entity {
    let entity = screen::button(parent, art.symbol_font(), label);
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
        p.spawn(art.text(investigator_summary(view), 13., PAPER));
    });
    screen::menu_button(commands, Panel, |p| {
        button(art, p, "Menu", Action::Utilities);
    });
    let portraits = portraits(art, images);
    screen::navigation(commands, (Panel, TabGroup::new(0)), |p| {
        for (index, who) in Investigator::ALL.into_iter().enumerate() {
            let current_turn = view
                .play
                .as_ref()
                .is_some_and(|play| play.scenario.outcome.is_none() && play.actor == Some(who));
            let entity = screen::portrait(
                p,
                art.symbol_font(),
                who.name(),
                portraits[index].as_ref(),
                current_turn,
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
        p.spawn(art.text("WASD: pan\nRight drag: orbit\nWheel: zoom\nHome: overview\n\nFocus does not select who acts.", 11., MUTED));
    });
    screen::decision_panel(
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
                p.spawn(art.text(&view.prompt, 18., PAPER));
            }
            if let Some(status) = play::status(view) {
                p.spawn(art.text(status, 15., GOLD));
            }
            if let Some(summary) = view.play.as_ref().and_then(test_summary) {
                p.spawn(art.text(summary, 15., GOLD));
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
                        if index == 0
                            && !table.menu_open
                            && !table.context.is_open()
                            && !reveal_open
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
                    "Restart The Gathering...",
                    Action::Restart(table.epoch),
                );
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
            button(art, p, "Close menu", Action::Dismiss);
        });
    }
    if table.presentation.result().is_some() {
        results::render(commands, table, view, art, size);
    } else if table.restart_confirmation {
        restart_confirmation(commands, table, art, size);
    } else if let Some(card) = table.presentation.story() {
        story_reading(commands, art, size, card);
    } else if table.result_opportunities() {
        results::render(commands, table, view, art, size);
    } else if table.modal() {
        inspection(commands, table, view, art, size, context);
    }
}

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
    screen::story_reading(
        commands,
        Panel,
        ScrollablePanel { inspection: true },
        size,
        art.card(card.code(), true).is_some(),
        card.title(),
        |panel| {
            panel.spawn(art.text(kind, 16., MUTED));
            panel.spawn(art.text(card.title(), 27., GOLD));
        },
        |body, width| {
            if let Some(image) = art.card(card.code(), true) {
                inspection_image(body, image, card.code(), width);
            }
        },
        |text| {
            if let Some(rules) = crate::gathering::inspection::scenario_card_reverse(card.code()) {
                text.spawn(art.text(rules, 19., PAPER));
            }
        },
        |panel| {
            screen::controls(panel, |row| {
                let first = { control(art, row, "Continue", Action::ContinueStory) };
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
                                "Test {} · {} · {} · {} vs {}",
                                index + 1,
                                report.tester.name(),
                                report.source,
                                report.total,
                                report.difficulty
                            ),
                            22.,
                            GOLD,
                        ));
                        results::details(body, art, report);
                    }
                }
                for line in &view.history {
                    body.spawn((
                        art.text(controls::display_history(view, line), 18., PAPER),
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
}
