use super::*;
use crate::gathering::{HostileFixture, TonyEffectFixture};

#[test]
fn chaos_report_rows_keep_each_token_without_repeating_numeric_contributions() {
    use crate::{
        TokenFace,
        gathering::{Skill, TestBreakdown, TestResult, TokenContribution},
    };
    use bevy::ecs::system::RunSystemOnce;

    let report = TestResult {
        tester: Investigator::Tony,
        source: "Report presentation fixture".into(),
        total: 0,
        difficulty: 2,
        success: false,
        failure_margin: 2,
        breakdown: TestBreakdown {
            skill: Skill::Combat,
            base: 5,
            cards: vec![],
            bonuses: vec![],
            tokens: vec![
                TokenContribution {
                    face: TokenFace::MinusTwo,
                    amount: -2,
                },
                TokenContribution {
                    face: TokenFace::MinusTwo,
                    amount: -2,
                },
                TokenContribution {
                    face: TokenFace::Skull,
                    amount: -2,
                },
                TokenContribution {
                    face: TokenFace::Curse,
                    amount: -2,
                },
                TokenContribution {
                    face: TokenFace::MinusTwo,
                    amount: -4,
                },
                TokenContribution {
                    face: TokenFace::AutoFail,
                    amount: 0,
                },
            ],
            token_total: -12,
            ignored: vec![TokenFace::MinusOne],
            canceled: vec![TokenFace::AutoFail],
            outcome_reason: None,
        },
    };
    let mut app = crate::tabletop_layout::heading_checks::app();
    app.init_resource::<art::Art>();
    app.world_mut()
        .run_system_once(move |mut commands: Commands, art: Res<art::Art>| {
            commands.spawn(Node::default()).with_children(|body| {
                super::super::results::details(body, &art, &report);
            });
        })
        .unwrap();
    let rows: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert_eq!(rows.iter().filter(|row| row.as_str() == "-2").count(), 2);
    for expected in [
        "Skull  /  -2",
        "Curse  /  -2",
        "-2 > -4",
        "Automatic failure",
        "-1  /  Ignored  /  no contribution",
        "Auto-fail  /  Canceled  /  no contribution",
    ] {
        assert!(
            rows.iter().any(|row| row == expected),
            "Missing {expected:?}: {rows:?}"
        );
    }
}

fn reach_reveal(session: &mut Session) {
    for _ in 0..12 {
        let view = session.observe();
        if view.choices.iter().any(|choice| choice.id == "test:reveal") {
            return;
        }
        let id = ["test-card:pass", "window:pass", "commit:confirm"]
            .into_iter()
            .find(|id| view.choices.iter().any(|choice| choice.id == *id))
            .unwrap();
        session.choose(view.revision, id).unwrap();
    }
    panic!("reveal was not reached");
}

fn acknowledge_result(app: &mut App) {
    let entity = button_with_label(app, "Continue").expect("A skill test has a reading stop");
    app.world_mut().trigger(Activate { entity });
    app.update();
}

fn activate_source_choice(app: &mut App, source: controls::Source, id: &str) {
    finish_presentation(app);
    click_source(app, source.clone());
    let view = app.world().resource::<Table>().view();
    let choices = controls::source_choices(&view, &source);
    let choice = choices.iter().find(|choice| choice.id == id).unwrap();
    let label = crate::tabletop::readable_text(ui::choice_label(choice).to_owned());
    let entity = button_with_label(app, &label).expect("The card's menu must offer its reaction");
    app.world_mut().trigger(Activate { entity });
    app.update();
}

#[test]
fn small_favor_defeat_keeps_reactions_usable_without_reopening_a_previous_test() {
    let session = Session::with_hostile_fixture(HostileFixture::Search(
        crate::gathering::SearchFixture::SmallFavorDefeat,
    ));
    let play = session.observe().play.unwrap();
    let enemy = |code: &str| {
        play.scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.code == code)
            .unwrap()
            .card
            .id
    };
    let favor = play.players[0]
        .hand
        .iter()
        .find(|card| card.code == "05277")
        .unwrap()
        .id;
    let bleeds = play.players[0]
        .hand
        .iter()
        .find(|card| card.code == "02225")
        .unwrap()
        .id;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("small-favor.json");
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(Table::new(session, path), false);

    // A failed evade leaves a real, acknowledged result before the testless event.
    activate_choice(
        &mut app,
        &format!("action:ordinary:evade:{}", enemy("01159")),
    );
    for id in [
        "window:pass",
        "commit:confirm",
        "window:pass",
        "test:reveal",
    ] {
        activate_choice(&mut app, id);
    }
    finish_presentation(&mut app);
    assert!(rendered_text(&mut app).contains("Chaos tokens"));
    acknowledge_result(&mut app);
    finish_presentation(&mut app);
    let previous = app
        .world()
        .resource::<Table>()
        .view()
        .play
        .unwrap()
        .completed_tests;
    assert_eq!(previous.len(), 1);

    activate_choice(
        &mut app,
        &format!(
            "action:ordinary:play:tony:{favor}:05277:{}:1:1",
            enemy("01161")
        ),
    );
    activate_source_choice(
        &mut app,
        controls::Source::Card(enemy("01159")),
        &format!("opportunity:{}", enemy("01159")),
    );
    finish_presentation(&mut app);
    let view = app.world().resource::<Table>().view();
    let after = view.play.as_ref().unwrap();
    assert!(after.test.is_none());
    assert_eq!(after.completed_tests, previous);
    assert!(view.choices.iter().any(|choice| choice.id == "defeat:done"));
    assert!(
        !rendered_text(&mut app).contains("Chaos tokens"),
        "A non-test defeat must not display the previous test's result"
    );
    assert!(button_with_label(&mut app, "Continue").is_none());

    activate_source_choice(
        &mut app,
        controls::Source::Card(bleeds),
        &format!("defeat:bleeds:{bleeds}"),
    );
    finish_presentation(&mut app);
    let healed = app.world().resource::<Table>().view().play.unwrap();
    assert_eq!(healed.players[0].horror, 0);
    assert_eq!(healed.players[0].resources, after.players[0].resources - 1);
    assert!(
        healed.players[0]
            .discard
            .iter()
            .any(|card| card.id == bleeds)
    );
    activate_choice(&mut app, "defeat:done");
    activate_choice(&mut app, "search:finish-event");
    finish_presentation(&mut app);
    assert!(!rendered_text(&mut app).contains("Chaos tokens"));
    assert!(
        app.world()
            .resource::<Table>()
            .view()
            .choices
            .iter()
            .any(|choice| choice.id == "action:ordinary:resource")
    );
}

#[test]
fn discard_ordering_preserves_the_acknowledgment_and_reopens_the_frozen_report() {
    let mut session = Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision);
    let cancel = session
        .observe()
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("test-of-will:"))
        .unwrap()
        .id
        .clone();
    session.choose(session.observe().revision, &cancel).unwrap();
    session
        .choose(session.observe().revision, "window:pass")
        .unwrap();
    let hand = session.observe().play.unwrap().players[1].hand.clone();
    let mut courage = 0;
    for code in ["04201", "01093"] {
        let card = hand.iter().find(|card| card.code == code).unwrap();
        session
            .choose(session.observe().revision, &format!("commit:{}", card.id))
            .unwrap();
        if code == "01093" {
            courage = card.id;
        }
    }
    session
        .choose(session.observe().revision, "commit:confirm")
        .unwrap();
    reach_reveal(&mut session);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("discard-order.json");
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(Table::new(session, path.clone()), false);
    activate_choice(&mut app, "test:reveal");
    finish_presentation(&mut app);
    acknowledge_result(&mut app);
    finish_presentation(&mut app);
    let play = app.world().resource::<Table>().view().play.unwrap();
    assert!(play.test.is_none() && play.pending.discard.is_some());
    assert!(play.completed_tests.is_empty());
    let mut reopened = host_with_table(
        Table::new(continuity::load_session(&path).unwrap(), path),
        false,
    );
    assert!(
        button_with_label(&mut reopened, "Continue").is_some(),
        "The saved discard-order decision must retain its readable frozen result"
    );
    activate_choice(&mut app, &format!("discard-card:{courage}"));
    finish_presentation(&mut app);
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .result()
            .is_none(),
        "ST8 cleanup must not repeat the result already read at ST6"
    );
}

#[test]
fn lita_and_if_it_bleeds_offer_direct_use_and_explicit_decline() {
    for (fixture, target, use_prefix, decline) in [
        (
            HostileFixture::GatheringLitaDamage,
            "lita:decline-damage",
            "lita:add-damage",
            "Decline Lita Chantler's extra damage",
        ),
        (
            HostileFixture::GatheringNonterminalDefeat,
            "defeat:done",
            "defeat:bleeds:",
            "Decline If it bleeds...",
        ),
    ] {
        let mut session = Session::with_hostile_fixture(fixture);
        let enemy = if target == "lita:decline-damage" {
            400
        } else {
            session
                .observe()
                .play
                .unwrap()
                .scenario
                .enemies
                .iter()
                .find(|enemy| enemy.card.code == "01160")
                .unwrap()
                .card
                .id
        };
        session
            .choose(
                session.observe().revision,
                &format!("action:ordinary:fight:{enemy}"),
            )
            .unwrap();
        for _ in 0..16 {
            let view = session.observe();
            if view.choices.iter().any(|choice| choice.id == target) {
                break;
            }
            let id = [
                "window:pass",
                "commit:confirm",
                "test:reveal",
                "lucky:decline",
            ]
            .into_iter()
            .find(|id| view.choices.iter().any(|choice| choice.id == *id))
            .unwrap();
            session.choose(view.revision, id).unwrap();
        }
        let view = session.observe();
        let use_effect = view
            .choices
            .iter()
            .find(|choice| choice.id.starts_with(use_prefix))
            .unwrap();
        let label =
            crate::tabletop::readable_text(controls::display_history(&view, &use_effect.label));
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("optional.json");
        continuity::save_session(&path, &session).unwrap();
        let mut app = host_with_table(Table::new(session, path), false);
        acknowledge_result(&mut app);
        assert!(button_with_label(&mut app, &label).is_some());
        assert!(button_with_label(&mut app, decline).is_some());
    }
}

#[test]
fn mists_movement_is_offered_directly_after_reading_the_result() {
    let mut session = Session::with_hostile_fixture(HostileFixture::SelectedWitness(
        crate::gathering::SelectedWitness::Mists,
    ));
    let play = session.observe().play.unwrap();
    let mists = play.players[1]
        .assets
        .iter()
        .find(|asset| asset.card.code == "04029")
        .unwrap()
        .card
        .id;
    let enemy = play
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01160")
        .unwrap()
        .card
        .id;
    session
        .choose(
            session.observe().revision,
            &format!("action:ordinary:evade:{enemy}:spell:{mists}"),
        )
        .unwrap();
    reach_reveal(&mut session);
    session
        .choose(session.observe().revision, "test:reveal")
        .unwrap();
    let view = session.observe();
    let movement = view
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("spell-move:") && choice.id != "spell-move:decline")
        .unwrap();
    let label = format!(
        "Move to {} (Mists of R'lyeh)",
        play::location_name(movement.id.strip_prefix("spell-move:").unwrap())
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("mists.json");
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(Table::new(session, path), false);
    acknowledge_result(&mut app);
    let entity = button_with_label(&mut app, &label)
        .expect("The available movement must be visible without opening a location menu");
    assert!(button_with_label(&mut app, "Decline Mists of R'lyeh movement").is_some());
    app.world_mut().trigger(Activate { entity });
    app.update();
    assert_eq!(
        app.world()
            .resource::<Table>()
            .session
            .observe()
            .play
            .unwrap()
            .players[1]
            .location,
        movement.id.strip_prefix("spell-move:").unwrap()
    );
}

#[test]
fn lethal_test_consequences_do_not_consume_the_survivors_next_result() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Retry(
        crate::gathering::RetryFixture::LethalFight,
    ));
    session
        .choose(session.observe().revision, "action:ordinary:fight:400")
        .unwrap();
    reach_reveal(&mut session);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("lethal.json");
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(Table::new(session, path), false);
    activate_choice(&mut app, "test:reveal");
    finish_presentation(&mut app);
    assert_eq!(
        app.world()
            .resource::<Table>()
            .presentation
            .result()
            .unwrap()
            .tester,
        Investigator::Patrice
    );
    acknowledge_result(&mut app);
    finish_presentation(&mut app);
    let play = app
        .world()
        .resource::<Table>()
        .session
        .observe()
        .play
        .unwrap();
    assert!(play.players[1].defeated);
    assert!(play.completed_tests.is_empty());
    activate_choice(&mut app, "elimination:lead:tony");
    finish_presentation(&mut app);
    if app
        .world()
        .resource::<Table>()
        .view()
        .choices
        .iter()
        .any(|choice| choice.id == "turn:tony")
    {
        activate_choice(&mut app, "turn:tony");
    }
    activate_choice(&mut app, "action:ordinary:investigate");
    for _ in 0..16 {
        finish_presentation(&mut app);
        if app
            .world()
            .resource::<Table>()
            .presentation
            .result()
            .is_some()
        {
            break;
        }
        let view = app.world().resource::<Table>().view();
        let id = [
            "test-card:pass",
            "window:pass",
            "commit:confirm",
            "test:reveal",
        ]
        .into_iter()
        .find(|id| view.choices.iter().any(|choice| choice.id == *id));
        let Some(id) = id else {
            break;
        };
        activate_choice(&mut app, id);
    }
    assert_eq!(
        app.world()
            .resource::<Table>()
            .presentation
            .result()
            .map(|report| report.tester),
        Some(Investigator::Tony),
        "The survivor's test needs its own acknowledgment even when ST7 prevented prior cleanup"
    );
}

fn quick_thinking_session() -> Session {
    quick_thinking_until("tony:quick-thinking:tony")
}

/// Challenge the production renderer with public-observation variants. These
/// choices test presentation only; they do not claim new legal rules windows.
fn render_opportunity_choices(choices: &[(&str, &str)]) -> App {
    use bevy::ecs::system::RunSystemOnce;

    let table = Table::new(Session::with_seed(19), PathBuf::new());
    let mut view = table.view();
    view.choices = choices
        .iter()
        .map(|(id, label)| crate::Choice {
            id: (*id).into(),
            label: (*label).into(),
            cancel_faces: vec![],
        })
        .collect();
    let mut app = crate::tabletop_layout::heading_checks::app();
    app.init_resource::<art::Art>().init_resource::<Actions>();
    app.world_mut()
        .run_system_once(move |mut commands: Commands, art: Res<art::Art>| {
            super::super::results::render(
                &mut commands,
                &table,
                &view,
                &art,
                Vec2::new(1600., 1000.),
            );
        })
        .unwrap();
    app
}

#[test]
fn opportunity_controls_preserve_unknown_use_order_and_first_decline() {
    let mut app = render_opportunity_choices(&[
        ("future:z", "Unrecognized last-letter effect"),
        ("after-test:pass", "Original pass wording"),
        ("undo", "Undo"),
        ("tony:quick-thinking:patrice", "Original use wording"),
        ("lucky:decline", "Second decline"),
        ("future:a", "Unrecognized first-letter effect"),
    ]);
    let labels = [
        "Unrecognized last-letter effect",
        "Take Patrice's extra action (Quick Thinking)",
        "Unrecognized first-letter effect",
    ];
    let buttons = labels.map(|label| button_with_label(&mut app, label).unwrap());
    let world = app.world();
    let parent = world.get::<ChildOf>(buttons[0]).unwrap().parent();
    assert_eq!(
        world
            .get::<Children>(parent)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        buttons,
        "Use controls retain observation order, including unrecognized choices"
    );
    assert!(world.get::<AutoFocus>(buttons[0]).is_some());
    for button in &buttons[1..] {
        assert!(world.get::<AutoFocus>(*button).is_none());
    }
    let decline = button_with_label(&mut app, "Decline all listed effects").unwrap();
    assert!(app.world().get::<AutoFocus>(decline).is_none());
    assert!(button_with_label(&mut app, "Undo").is_none());
    assert!(button_with_label(&mut app, "Second decline").is_none());
    for (button, id) in buttons
        .into_iter()
        .zip(["future:z", "tony:quick-thinking:patrice", "future:a"])
        .chain([(decline, "after-test:pass")])
    {
        app.world_mut().trigger(Activate { entity: button });
        assert!(matches!(
            app.world_mut().resource_mut::<Actions>().0.pop(),
            Some(Action::ResultChoice(_, 0, chosen)) if chosen == id
        ));
    }
}

#[test]
fn opportunity_footer_focus_and_plural_wording_include_other_declines() {
    for (choices, footer_focused) in [
        (
            vec![("after-test:pass", "Pass"), ("success:pass", "Other pass")],
            true,
        ),
        (
            vec![
                ("tony:quick-thinking:tony", "Use Quick Thinking"),
                ("after-test:pass", "Pass"),
                ("success:pass", "Other pass"),
            ],
            false,
        ),
    ] {
        let mut app = render_opportunity_choices(&choices);
        let footer = button_with_label(&mut app, "Decline all listed effects").unwrap();
        assert_eq!(
            app.world().get::<AutoFocus>(footer).is_some(),
            footer_focused
        );
        assert!(button_with_label(&mut app, "Decline Quick Thinking").is_none());
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<Entity, With<AutoFocus>>()
                .iter(world)
                .count(),
            1
        );
        world.trigger(Activate { entity: footer });
        assert!(matches!(
            world.resource_mut::<Actions>().0.pop(),
            Some(Action::ResultChoice(_, 0, chosen)) if chosen == "after-test:pass"
        ));
    }
}

#[test]
fn result_and_optional_effect_controls_stay_visible_when_details_scroll() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("layout.json");
    let session = quick_thinking_session();
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(Table::new(session, path), false);
    let size = Vec2::new(960., 540.);
    let scale = 1.25;
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, scale);
    app.world_mut().resource_mut::<Table>().dirty = true;
    for _ in 0..4 {
        app.update();
    }
    assert_unclipped_button(&mut app, "Continue", size, scale);
    let button = button_with_label(&mut app, "Continue").unwrap();
    app.world_mut().trigger(Activate { entity: button });
    for _ in 0..4 {
        app.update();
    }
    for label in [
        "Take Tony's extra action (Quick Thinking)",
        "Decline Quick Thinking",
    ] {
        assert_unclipped_button(&mut app, label, size, scale);
    }
}

fn quick_thinking_until(stop: &str) -> Session {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::QuickThinking));
    let play = session.observe().play.unwrap();
    let quick = play.players[0]
        .hand
        .iter()
        .find(|card| card.code == "02229")
        .unwrap()
        .id;
    let enemy = play.scenario.enemies[0].card.id;
    session
        .choose(
            session.observe().revision,
            &format!("action:ordinary:fight:{enemy}"),
        )
        .unwrap();
    let mut committed = false;
    for _ in 0..30 {
        let view = session.observe();
        if view.choices.iter().any(|choice| choice.id == stop) {
            return session;
        }
        let commit = format!("commit:{quick}");
        let id = if !committed && view.choices.iter().any(|choice| choice.id == commit) {
            committed = true;
            commit
        } else {
            [
                "test-card:pass",
                "window:pass",
                "commit:confirm",
                "test:reveal",
                "lucky:decline",
                "success:pass",
                "revelation:resolve",
            ]
            .into_iter()
            .find(|id| view.choices.iter().any(|choice| choice.id == *id))
            .map(str::to_owned)
            .or_else(|| {
                view.choices
                    .iter()
                    .find(|choice| {
                        choice.id.starts_with("draw:reveal:") || choice.id.starts_with("result:")
                    })
                    .map(|choice| choice.id.clone())
            })
            .unwrap_or_else(|| panic!("Unhandled test window {:?}", view.choices))
        };
        session.choose(view.revision, &id).unwrap();
    }
    panic!("Quick Thinking was never offered")
}

#[test]
fn live_result_blocks_queued_choices_and_duplicate_acknowledgments() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("live.json");
    let session = quick_thinking_until("test:reveal");
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(Table::new(session, path), false);
    activate_choice(&mut app, "test:reveal");
    // Play the genuine cancellation/outcome decisions until ST6 fixes the result.
    for _ in 0..40 {
        if app
            .world()
            .resource::<Table>()
            .presentation
            .result()
            .is_some()
        {
            break;
        }
        finish_presentation(&mut app);
        if app
            .world()
            .resource::<Table>()
            .presentation
            .result()
            .is_some()
        {
            break;
        }
        let view = app.world().resource::<Table>().view();
        let id = ["lucky:decline", "success:pass"]
            .into_iter()
            .find(|id| view.choices.iter().any(|choice| choice.id == *id))
            .unwrap();
        let table = app.world().resource::<Table>();
        let action = Action::ResultChoice(table.epoch, view.revision, id.into());
        app.world_mut().resource_mut::<Actions>().0.push(action);
        app.update();
    }
    let table = app.world().resource::<Table>();
    assert!(table.presentation.result().is_some());
    let epoch = table.epoch;
    let before = table.session.clone();
    let revision = table.session.observe().revision;
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::Choose(epoch, revision, "after-test:pass".into()),
        Action::Close,
    ]);
    app.update();
    assert_eq!(app.world().resource::<Table>().session.clone(), before);
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .result()
            .is_some()
    );
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::ContinueResult(epoch),
        Action::ContinueResult(epoch),
        Action::ResultChoice(epoch, revision, "after-test:pass".into()),
    ]);
    app.update();
    finish_presentation(&mut app);
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .result()
            .is_none(),
        "Cleanup must not repeat the acknowledged ST6 result"
    );
    assert!(button_with_label(&mut app, "Decline Quick Thinking").is_some());
    assert_eq!(app.world().resource::<Table>().session.clone(), before);
}

#[test]
fn explicit_decline_is_saved_and_reopening_the_report_does_not_change_rules() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("decline.json");
    let session = quick_thinking_session();
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(
        Table::new(continuity::load_session(&path).unwrap(), path.clone()),
        false,
    );
    let acknowledge = button_with_label(&mut app, "Continue").unwrap();
    app.world_mut().trigger(Activate {
        entity: acknowledge,
    });
    app.update();
    let decline = button_with_label(&mut app, "Decline Quick Thinking").unwrap();
    app.world_mut().trigger(Activate { entity: decline });
    app.update();
    let saved = continuity::load_session(&path).unwrap();
    assert_eq!(saved.observe().play.unwrap().actions_remaining[0], 2);
    assert!(
        !saved
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("granted:"))
    );
    let mut reopened = host_with_table(Table::new(saved.clone(), path), false);
    assert_eq!(reopened.world().resource::<Table>().session, saved);
    assert!(button_with_label(&mut reopened, "Decline Quick Thinking").is_none());
}

#[test]
fn undo_after_a_completed_test_does_not_reoffer_its_acknowledged_result() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("undo-reading.json");
    let session = quick_thinking_session();
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(Table::new(session, path.clone()), false);
    acknowledge_result(&mut app);
    let decline = button_with_label(&mut app, "Decline Quick Thinking").unwrap();
    app.world_mut().trigger(Activate { entity: decline });
    app.update();
    finish_presentation(&mut app);
    let before = app.world().resource::<Table>().view().play.unwrap();
    assert_eq!(before.completed_tests.len(), 1);
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_millis(100));
    for _ in 0..30 {
        app.update();
    }
    let revision = app.world().resource::<Table>().view().revision;
    activate_choice(&mut app, "action:ordinary:resource");
    finish_presentation(&mut app);
    let pending = app.world().resource::<Table>().view();
    assert_eq!(pending.revision, revision + 1);
    // The engaged enemy's attack of opportunity precedes gaining the resource.
    // Undo must restore the acknowledged result even at that pending consequence.
    assert!(pending.choices.iter().any(|choice| choice.id == "undo"));
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_millis(100));
    for _ in 0..30 {
        app.update();
    }
    activate_choice(&mut app, "undo");
    finish_presentation(&mut app);
    let restored = app.world().resource::<Table>().view();
    assert_eq!(
        restored.play.as_ref().unwrap().players[0].resources,
        before.players[0].resources
    );
    assert_eq!(
        restored.play.as_ref().unwrap().completed_tests,
        before.completed_tests
    );
    assert!(button_with_label(&mut app, "Continue").is_none());
    assert!(
        restored
            .choices
            .iter()
            .any(|choice| choice.id == "action:ordinary:resource")
    );
    assert_eq!(continuity::load_session(&path).unwrap().observe(), restored);
}

#[test]
fn result_waits_for_continue_then_offers_quick_thinking_explicitly() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("result.json");
    let session = quick_thinking_session();
    continuity::save_session(&path, &session).unwrap();
    let before = session.clone();
    let mut app = host_with_table(Table::new(session, path.clone()), false);
    let continue_button = button_with_label(&mut app, "Continue")
        .expect("Every completed test waits for an explicit acknowledgment");
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_secs(120));
    app.update();
    assert!(button_with_label(&mut app, "Continue").is_some());
    assert_eq!(app.world().resource::<Table>().session.clone(), before);
    app.world_mut().trigger(Activate {
        entity: continue_button,
    });
    app.update();
    assert!(button_with_label(&mut app, "Decline Quick Thinking").is_some());
    let use_effect = button_with_label(&mut app, "Take Tony's extra action (Quick Thinking)")
        .expect("Quick Thinking is offered directly");
    assert!(
        button_with_label(&mut app, "Continue").is_none(),
        "Acknowledgment cannot decline an optional effect"
    );
    app.world_mut().trigger(Activate { entity: use_effect });
    app.update();
    assert!(
        app.world()
            .resource::<Table>()
            .session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("granted:"))
    );
    assert_eq!(
        continuity::load_session(&path).unwrap(),
        app.world().resource::<Table>().session
    );
}
