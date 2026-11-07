use super::*;

#[test]
fn curse_followup_offers_one_new_jacqueline_declaration_while_prior_tokens_stay_held() {
    let mut session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tokens(
        crate::gathering::TokenFixture::ArbiterCurse,
    ));
    for _ in 0..5 {
        let view = session.observe();
        if view.choices.iter().any(|choice| choice.id == "test:reveal") {
            break;
        }
        let id = if view
            .choices
            .iter()
            .any(|choice| choice.id == "commit:confirm")
        {
            "commit:confirm"
        } else {
            "window:pass"
        };
        session.choose(view.revision, id).unwrap();
    }
    let arbiter = session
        .observe()
        .choices
        .into_iter()
        .find(|choice| choice.id.starts_with("tokens:jacqueline:arbiter:"))
        .unwrap()
        .id;
    for id in [
        arbiter.as_str(),
        "test:reveal",
        "tokens:cancel:1",
        "tokens:cancel:2",
        "tokens:confirm",
    ] {
        session.choose(session.observe().revision, id).unwrap();
    }
    let held = session.observe().play.unwrap().tokens.unwrap().revealed;
    assert_eq!(held.len(), 3);
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("curse.json")), true);
    let action = choice_button(&mut app, "Use Jacqueline — reveal 3 tokens")
        .expect("The new curse reveal has the same direct declaration control");
    assert!(choice_button(&mut app, "Use Jacqueline — spend round use").is_none());
    app.world_mut().trigger(Activate { entity: action });
    app.update();
    let tokens = app
        .world()
        .resource::<Table>()
        .session
        .observe()
        .play
        .unwrap()
        .tokens
        .unwrap();
    assert_eq!(
        tokens.revealed, held,
        "Declaring never releases or redraws held tokens"
    );
    assert_eq!(tokens.fresh_reveal_count(), Some(3));
}

#[test]
fn jacquelines_three_tokens_share_one_row_and_have_separate_readable_captions() {
    let session = super::review_saves::resolution_review::checkpoints()
        .unwrap()
        .into_iter()
        .find(|point| point.name == "jacqueline-three")
        .unwrap()
        .session;
    let groups = session
        .observe()
        .play
        .unwrap()
        .tokens
        .unwrap()
        .reveal_groups();
    assert_eq!(groups.len(), 1, "The original draw has no special status");
    assert_eq!(groups[0].members.len(), 3);
    assert_token_captions_are_separate(session, true);
}

#[test]
fn voice_of_ra_nested_tokens_have_separate_readable_captions() {
    assert_token_captions_are_separate(voice_ra_selection(), false);
}

fn assert_token_captions_are_separate(session: Session, ordinary: bool) {
    use crate::tabletop_layout::heading_checks::{rendered_text_bounds, viewport};
    let group_names: Vec<_> = session
        .observe()
        .play
        .unwrap()
        .tokens
        .unwrap()
        .reveal_groups()
        .into_iter()
        .map(|group| crate::tabletop::readable_text(group.source))
        .collect();
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("tokens.json")), true);
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(if ordinary {
            Action::Focus(Area::Test)
        } else {
            Action::Resolve
        });
    finish_presentation(&mut app);
    for size in [Vec2::new(1280., 720.), Vec2::new(1600., 1000.)] {
        for scale in [1., 2.] {
            viewport::<MainCamera>(&mut app, size, scale);
            for _ in 0..5 {
                app.update();
            }
            let world = app.world_mut();
            let labels: Vec<_> = world
                .query::<(Entity, &crate::tabletop_tokens::Follow)>()
                .iter(world)
                .map(|(entity, follow)| (entity, follow.0))
                .collect();
            assert_eq!(labels.len(), if ordinary { 3 } else { 9 });
            if ordinary {
                let z = world.get::<Transform>(labels[0].1).unwrap().translation.z;
                assert!(labels.iter().all(|(_, token)| {
                    (world.get::<Transform>(*token).unwrap().translation.z - z).abs() < 0.001
                }));
            }
            let mut captions: Vec<_> = labels
                .iter()
                .map(|(entity, _)| {
                    (
                        world.get::<Text>(*entity).unwrap().0.clone(),
                        rendered_text_bounds(world, *entity, scale),
                    )
                })
                .collect();
            let headings: Vec<_> = world
                .query::<(Entity, &WorldLabel, &Text)>()
                .iter(world)
                .filter(|(_, _, text)| {
                    group_names.contains(&text.0) || (ordinary && text.0.contains("Watch this!"))
                })
                .map(|(entity, _, text)| (entity, text.0.clone()))
                .collect();
            if ordinary {
                assert_eq!(
                    headings
                        .iter()
                        .filter(|(_, text)| text.contains("Watch this!"))
                        .count(),
                    1,
                    "The committed Watch this! card must keep its caption"
                );
            } else {
                assert!(
                    headings.len() >= group_names.len(),
                    "Meaningful nested groups keep their headings"
                );
            }
            for (entity, text) in headings {
                assert!(
                    world.get::<InheritedVisibility>(entity).unwrap().get(),
                    "Group/card caption must remain visible at Resolve focus: {text}"
                );
                captions.push((text, rendered_text_bounds(world, entity, scale)));
            }
            for (index, (name, bounds)) in captions.iter().enumerate() {
                assert!(
                    bounds.min.cmpge(Vec2::ZERO).all() && bounds.max.cmple(size).all(),
                    "{name}: {bounds:?}"
                );
                for (other, next) in &captions[index + 1..] {
                    assert!(
                        bounds.intersect(*next).is_empty(),
                        "{size:?}/{scale}: {name} {bounds:?} overlaps {other} {next:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn plain_jacqueline_control_declares_three_tokens_without_revealing_or_changing_tester() {
    let mut session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::JacquelineCancellation);
    for _ in 0..5 {
        let view = session.observe();
        if view.choices.iter().any(|choice| choice.id == "test:reveal") {
            break;
        }
        let id = if view
            .choices
            .iter()
            .any(|choice| choice.id == "commit:confirm")
        {
            "commit:confirm"
        } else {
            "window:pass"
        };
        session.choose(view.revision, id).unwrap();
    }
    let revealer = session.observe().play.unwrap().tokens.unwrap().revealer;
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("tokens.json")), true);
    let action = choice_button(&mut app, "Use Jacqueline — reveal 3 tokens")
        .expect("One direct ability action");
    crate::tabletop_layout::font_checks::assert_supported(app.world_mut());
    assert!(button_with_label(&mut app, "Jacqueline's ability...").is_none());
    app.world_mut().trigger(Activate { entity: action });
    app.update();
    let tokens = app
        .world()
        .resource::<Table>()
        .session
        .observe()
        .play
        .unwrap()
        .tokens
        .unwrap();
    assert_eq!(tokens.revealer, revealer);
    assert!(tokens.revealed.is_empty());
    assert_eq!(tokens.fresh_reveal_count(), Some(3));
}

#[test]
fn jacqueline_cost_disclosure_is_flat_aligned_and_does_not_execute_a_choice() {
    let mut session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tokens(
        crate::gathering::TokenFixture::ArbiterCancellation,
    ));
    for _ in 0..5 {
        let view = session.observe();
        if view.choices.iter().any(|choice| choice.id == "test:reveal") {
            break;
        }
        let id = if view
            .choices
            .iter()
            .any(|choice| choice.id == "commit:confirm")
        {
            "commit:confirm"
        } else {
            "window:pass"
        };
        session.choose(view.revision, id).unwrap();
    }
    let before = session.clone();
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("tokens.json")), true);
    let disclosure = choice_button(&mut app, "▸ Jacqueline's ability options · show").unwrap();
    crate::tabletop_layout::font_checks::assert_supported(app.world_mut());
    assert_eq!(
        app.world().get::<BackgroundColor>(disclosure).unwrap().0,
        Color::NONE
    );
    app.world_mut().trigger(Activate { entity: disclosure });
    app.update();
    assert_eq!(app.world().resource::<Table>().session, before);
    let expanded = choice_button(&mut app, "▾ Jacqueline's ability options · hide").unwrap();
    crate::tabletop_layout::font_checks::assert_supported(app.world_mut());
    let ordinary = choice_button(&mut app, "Use Jacqueline — spend round use").unwrap();
    let arbiter = choice_button(&mut app, "Use Jacqueline + Arbiter — preserve round use").unwrap();
    let resolve = choice_button(&mut app, "Resolve the declared token reveal").unwrap();
    let left = |entity| {
        let node = app.world().get::<ComputedNode>(entity).unwrap();
        app.world()
            .get::<UiGlobalTransform>(entity)
            .unwrap()
            .transform_point2(-node.size() / 2.)
            .x
    };
    assert!((left(expanded) - left(resolve)).abs() < 0.1);
    assert!((left(ordinary) - left(resolve)).abs() < 0.1);
    assert!(
        app.world()
            .get::<feedback::DisclosureButton>(arbiter)
            .is_none()
    );
    app.world_mut().trigger(Activate { entity: arbiter });
    app.update();
    let live = app.world().resource::<Table>().session.observe();
    assert!(
        !live
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("tokens:jacqueline"))
    );
    assert!(live.play.unwrap().tokens.unwrap().revealed.is_empty());
}

fn voice_ra_selection() -> Session {
    use crate::gathering::{HostileFixture, TokenFixture};
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::VoiceRaGuidance));
    let prophecy = session.observe().play.unwrap().players[2].hand[0].id;
    for id in [
        "tokens:jacqueline".to_owned(),
        format!("tokens:prophecy:0:{prophecy}"),
        "test:reveal".to_owned(),
    ] {
        session.choose(session.observe().revision, &id).unwrap();
    }
    session
}

#[test]
fn token_progress_tracks_selection_deselection_and_the_autofail_cancellation_alternative() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(voice_ra_selection(), dir.path().join("tokens.json")),
        true,
    );
    assert!(rendered_text(&mut app).contains("Selected 0 of 1 token"));

    // Dark Prophecy retains the Auto-fail in this fixed nine-token reveal. The
    // following cancellation can select that token alone or two other tokens.
    for (choice, progress, can_confirm) in [
        ("tokens:keep:3", "Selected 1 of 1 token", true),
        ("tokens:keep:3", "Selected 0 of 1 token", false),
        ("tokens:keep:3", "Selected 1 of 1 token", true),
        ("tokens:confirm", "Selected 0 of 2 tokens", false),
        ("tokens:cancel:3", "Selected 1 of 1 token", true),
        ("tokens:cancel:3", "Selected 0 of 2 tokens", false),
        ("tokens:cancel:5", "Selected 1 of 2 tokens", false),
        ("tokens:cancel:7", "Selected 2 of 2 tokens", true),
        ("tokens:cancel:7", "Selected 1 of 2 tokens", false),
        ("tokens:cancel:5", "Selected 0 of 2 tokens", false),
    ] {
        activate_choice(&mut app, choice);
        let text = rendered_text(&mut app);
        assert!(text.contains(progress), "After {choice}: {text}");
        let view = app.world().resource::<Table>().view();
        let confirm = view
            .choices
            .iter()
            .find(|choice| choice.id == "tokens:confirm");
        assert_eq!(confirm.is_some(), can_confirm, "After {choice}: {text}");
        if let Some(confirm) = confirm {
            assert!(choice_button(&mut app, ui::choice_label(confirm)).is_some());
        }
    }
}

#[test]
fn resolution_diagnostic_matches_resolve_and_preserves_home_and_the_pending_choice() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(voice_ra_selection(), dir.path().join("tokens.json")),
        true,
    );
    let before = app.world().resource::<Table>().session.clone();
    let resolve = button_with_label(&mut app, "Resolve").unwrap();
    app.world_mut().trigger(Activate { entity: resolve });
    app.update();
    let expected = app.world().resource::<Orbit>().pose();
    let options = Options::parse(
        AssetConfig::load_at(dir.path().join("settings"), dir.path().join("art")),
        ["--initial-view", "resolve"].into_iter().map(str::to_owned),
    )
    .unwrap()
    .unwrap();
    options.configure_diagnostics(&mut app).unwrap();
    assert_eq!(app.world().resource::<Orbit>().pose(), expected);
    assert!(!app.world().resource::<Orbit>().is_following());
    // Camera keys keep their normal focus ownership. Return keyboard focus
    // from the pending-choice controls to the table before pressing Home.
    app.insert_resource(InputFocus::default());
    keyboard(
        &mut app,
        KeyCode::Home,
        bevy::input::keyboard::Key::Home,
        true,
    );
    app.update();
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        enemy_board::focus(Area::Overview).0
    );
    assert_eq!(app.world().resource::<Table>().session, before);
    assert!(!dir.path().join("tokens.json").exists());
    assert!(!dir.path().join("settings").exists());
}

#[test]
fn resolution_diagnostic_refuses_a_session_without_an_active_drawer() {
    let dir = tempfile::tempdir().unwrap();
    let save = dir.path().join("setup.json");
    continuity::save_session(&save, &Session::with_seed(19)).unwrap();
    let saved_bytes = std::fs::read(&save).unwrap();
    let options = Options::parse(
        AssetConfig::load_at(dir.path().join("settings"), dir.path().join("art")),
        [
            "--save",
            save.to_str().unwrap(),
            "--initial-view",
            "resolve",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap()
    .unwrap();
    let mut app = host_with_table(options.open_table().unwrap(), false);
    let before = app.world().resource::<Table>().session.clone();
    let pose = app.world().resource::<Orbit>().pose();
    assert_eq!(
        options.configure_diagnostics(&mut app).unwrap_err(),
        "No active resolution drawer for --initial-view resolve."
    );
    assert_eq!(app.world().resource::<Orbit>().pose(), pose);
    assert_eq!(app.world().resource::<Table>().session, before);
    assert_eq!(std::fs::read(&save).unwrap(), saved_bytes);
    assert!(!dir.path().join("settings").exists());
}

#[test]
fn refused_resolution_startup_does_not_create_a_save_or_parent_directory() {
    let dir = tempfile::tempdir().unwrap();
    let save = dir.path().join("unused/setup.json");
    let options = Options::parse(
        AssetConfig::load_at(dir.path().join("settings"), dir.path().join("art")),
        [
            "--save",
            save.to_str().unwrap(),
            "--initial-view",
            "resolve",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap()
    .unwrap();
    let mut app = App::new();
    let startup = options.open_table().and_then(|table| {
        app.insert_resource(table);
        options.configure_diagnostics(&mut app)
    });
    let error = startup.unwrap_err();
    assert!(error.contains("--initial-view resolve"), "{error}");
    assert!(
        !save.exists(),
        "A refused diagnostic must not create a save"
    );
    assert!(!save.parent().unwrap().exists());
    assert!(!dir.path().join("settings").exists());
}

#[test]
fn nested_token_selection_has_visible_source_instruction_and_direct_controls() {
    use crate::gathering::{HostileFixture, TokenFixture};
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::NestedProphecyCleanup));
    let prophecy = session.observe().play.unwrap().players[2].hand[0].id;
    for id in [
        "tokens:jacqueline".to_owned(),
        format!("tokens:prophecy:0:{prophecy}"),
        "test:reveal".to_owned(),
    ] {
        session.choose(session.observe().revision, &id).unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let table = Table::new(session, dir.path().join("tokens.json"));
    let mut app = host_with_table(table, true);
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
        &mut app,
        Vec2::new(1600., 1000.),
        1.,
    );
    app.world_mut().resource_mut::<Table>().dirty = true;
    for _ in 0..4 {
        app.update();
    }
    let text = rendered_text(&mut app);
    assert!(
        text.contains("Dark Prophecy") && text.contains("Choose 1"),
        "{text}"
    );
    let choices = app.world().resource::<Table>().view().choices;
    for choice in choices
        .iter()
        .filter(|choice| choice.id.starts_with("tokens:keep:"))
    {
        assert_unclipped_button(
            &mut app,
            ui::choice_label(choice),
            Vec2::new(1600., 1000.),
            1.,
        );
    }
    assert!(!app.world().resource::<Table>().context.is_open());
}

#[test]
#[ignore = "Read-only startup check for a supplied ordinary pending-token save"]
fn supplied_token_save_opens_on_visible_guidance() {
    let path =
        std::env::var("ARKHAM_GATHERING_REVIEW_SAVE").expect("Supply a pending-token save path");
    let before = std::fs::read(&path).unwrap();
    let table = Table::open(path.clone().into()).unwrap();
    assert!(!table.modal());
    let mut app = host_with_table(table, true);
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
        &mut app,
        Vec2::new(1600., 1000.),
        1.,
    );
    app.world_mut().resource_mut::<Table>().dirty = true;
    for _ in 0..4 {
        app.update();
    }
    let view = app.world().resource::<Table>().view();
    let tokens = view
        .play
        .as_ref()
        .unwrap()
        .tokens
        .as_ref()
        .expect("Pending physical tokens");
    assert!(!tokens.revealed.is_empty());
    assert!(rendered_text(&mut app).contains("Jacqueline"));
    for choice in view
        .choices
        .iter()
        .filter(|choice| !tokens.choice_members(&choice.id).is_empty())
    {
        assert_unclipped_button(
            &mut app,
            ui::choice_label(choice),
            Vec2::new(1600., 1000.),
            1.,
        );
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
