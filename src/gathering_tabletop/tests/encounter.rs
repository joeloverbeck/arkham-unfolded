use super::*;
use crate::gathering::{HostileFixture, SearchFixture};

pub(super) fn continue_reading(app: &mut App) {
    assert!(rendered_text(app).contains("ENCOUNTER CARD"));
    let entity = choice_button(app, "Continue").unwrap();
    app.world_mut().trigger(Activate { entity });
    app.update();
}

pub(super) fn respond(app: &mut App, id: &str) {
    assert!(rendered_text(app).contains("ENCOUNTER CARD"));
    let label = app
        .world()
        .resource::<Table>()
        .session
        .observe()
        .choices
        .into_iter()
        .find(|choice| choice.id == id)
        .unwrap()
        .label;
    let entity = choice_button(app, &label).expect("The legal response is visible in the modal");
    app.world_mut().trigger(Activate { entity });
    app.update();
}

fn wait_for_reading(app: &mut App) {
    for _ in 0..100 {
        if rendered_text(app).contains("ENCOUNTER CARD") {
            return;
        }
        draws::tick(app, 0.1);
    }
    panic!("The committed encounter draw must reach its reading stop");
}

#[test]
fn encounter_reading_waits_for_the_draw_and_continue_keeps_the_enemy_spawn_rules() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("encounter.json");
    let session = Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HuntTargets));
    let mut app = host_with_table(Table::new(session, path.clone()), false);
    activate_choice(&mut app, "mythos:draw");
    assert!(!rendered_text(&mut app).contains("ENCOUNTER CARD"));
    activate_choice(&mut app, "search:decline-hunt");
    let drawn = app.world().resource::<Table>().session.clone();
    let revelation = app
        .world()
        .resource::<Table>()
        .view()
        .play
        .unwrap()
        .pending_revelation
        .unwrap();
    assert_eq!(revelation.card.code, "01118");
    assert_eq!(revelation.drawn_by, Investigator::Tony);
    assert!(!rendered_text(&mut app).contains("ENCOUNTER CARD"));
    for _ in 0..100 {
        draws::tick(&mut app, 0.1);
        if rendered_text(&mut app).contains("ENCOUNTER CARD") {
            break;
        }
    }
    let text = rendered_text(&mut app);
    assert!(text.contains("ENCOUNTER CARD"), "{text}");
    assert!(text.contains("Drawn by Tony Morgan"));
    assert!(text.contains("Flesh-Eater  /  Enemy"), "{text}");
    assert_eq!(app.world().resource::<Table>().session, drawn);
    assert_eq!(continuity::load_session(&path).unwrap(), drawn);
    draws::tick(&mut app, 30.);
    assert_eq!(app.world().resource::<Table>().session, drawn);

    let continue_button = choice_button(&mut app, "Continue").unwrap();
    app.world_mut().trigger(Activate {
        entity: continue_button,
    });
    app.world_mut().trigger(Activate {
        entity: continue_button,
    });
    app.update();
    finish_presentation(&mut app);
    assert_eq!(app.world().resource::<Table>().session, drawn);
    let settled = app.world().resource::<Table>().session.observe();
    let play = settled.play.unwrap();
    assert!(play.pending_revelation.is_none());
    assert!(
        play.encounter_discard
            .iter()
            .any(|card| card.id == revelation.card.id)
    );
    assert!(
        !play
            .scenario
            .enemies
            .iter()
            .any(|enemy| enemy.card.id == revelation.card.id)
    );
    assert!(!rendered_text(&mut app).contains("ENCOUNTER CARD"));
}

#[test]
fn reopening_an_unstarted_treachery_can_cancel_from_the_modal_without_duplicate_costs() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ward.json");
    let session = Session::with_hostile_fixture(HostileFixture::Harm(
        crate::gathering::HarmFixture::WardCancellation,
    ));
    continuity::save_session(&path, &session).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let mut app = host_with_table(
        Table::new(continuity::load_session(&path).unwrap(), path.clone()),
        false,
    );
    let text = rendered_text(&mut app);
    assert!(text.contains("ENCOUNTER CARD"));
    assert!(text.contains("Drawn by Jacqueline Fine"));
    assert!(text.contains("Rotting Remains  /  Treachery"));
    let view = session.observe();
    let ward = view
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("ward:"))
        .unwrap()
        .id
        .clone();
    let epoch = app.world().resource::<Table>().epoch;
    let old_choice = Action::Choose(epoch, view.revision, ward.clone());
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(old_choice.clone());
    draws::tick(&mut app, 30.);
    assert_eq!(app.world().resource::<Table>().session, session);
    let rules = choice_button(&mut app, "Read rules").unwrap();
    app.world_mut().trigger(Activate { entity: rules });
    app.update();
    assert_eq!(app.world().resource::<Table>().session, session);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let label = &view
        .choices
        .iter()
        .find(|choice| choice.id == ward)
        .unwrap()
        .label;
    let entity = choice_button(&mut app, label).expect("Ward is available before Continue");
    app.world_mut().trigger(Activate { entity });
    app.world_mut().trigger(Activate { entity });
    app.world_mut().resource_mut::<Actions>().0.push(old_choice);
    app.update();
    let mut expected = session;
    expected.choose(view.revision, &ward).unwrap();
    assert_eq!(app.world().resource::<Table>().session, expected);
    assert_eq!(continuity::load_session(&path).unwrap(), expected);
    finish_presentation(&mut app);

    // An unchanged in-place reopen and scene rebuild do not manufacture a draw.
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Reopen);
    app.update();
    app.world_mut().resource_mut::<Table>().dirty = true;
    app.update();
    assert!(!rendered_text(&mut app).contains("ENCOUNTER CARD"));
    finish_presentation(&mut app);
    assert!(rendered_text(&mut app).contains("1 horror on Jacqueline Fine"));
    assert!(!rendered_text(&mut app).contains("ENCOUNTER CARD"));
    let mut reopened = host_with_table(
        Table::new(continuity::load_session(&path).unwrap(), path),
        false,
    );
    assert!(!rendered_text(&mut reopened).contains("ENCOUNTER CARD"));
    assert!(rendered_text(&mut reopened).contains("1 horror on Jacqueline Fine"));
}

#[test]
fn encounter_continue_commits_once_and_a_failed_save_keeps_the_card_and_responses() {
    let directory = tempfile::tempdir().unwrap();
    let session = Session::with_hostile_fixture(HostileFixture::Harm(
        crate::gathering::HarmFixture::WardCancellation,
    ));
    let obstruction = directory.path().join("obstruction");
    std::fs::write(&obstruction, "original bytes").unwrap();
    let mut app = host_with_table(
        Table::new(session.clone(), obstruction.join("save.json")),
        false,
    );
    continue_reading(&mut app);
    assert_eq!(app.world().resource::<Table>().session, session);
    let text = rendered_text(&mut app);
    assert!(text.contains("ENCOUNTER CARD"));
    assert!(text.contains("Decision not committed"));
    assert!(text.contains("Ward of Protection"));
    assert_eq!(
        std::fs::read_to_string(&obstruction).unwrap(),
        "original bytes"
    );

    let path = directory.path().join("save.json");
    app.world_mut().resource_mut::<Table>().save_path = path.clone();
    let view = session.observe();
    let old_epoch = app.world().resource::<Table>().epoch;
    let button = choice_button(&mut app, "Continue").unwrap();
    app.world_mut().trigger(Activate { entity: button });
    app.world_mut().trigger(Activate { entity: button });
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::EncounterChoice(
            old_epoch,
            view.revision,
            view.choices
                .iter()
                .find(|choice| choice.id.starts_with("ward:"))
                .unwrap()
                .id
                .clone(),
        ));
    app.update();
    let mut expected = session;
    expected
        .choose(view.revision, "revelation:resolve")
        .unwrap();
    assert_eq!(app.world().resource::<Table>().session, expected);
    assert_eq!(continuity::load_session(&path).unwrap(), expected);
    finish_presentation(&mut app);
    assert!(!rendered_text(&mut app).contains("ENCOUNTER CARD"));
    assert_eq!(
        app.world()
            .resource::<Table>()
            .view()
            .play
            .unwrap()
            .test
            .unwrap()
            .difficulty,
        3
    );
}

#[test]
fn encounter_surround_has_no_pause_control_and_resumes_without_a_focus_time_jump() {
    let directory = tempfile::tempdir().unwrap();
    let session = Session::with_hostile_fixture(HostileFixture::Harm(
        crate::gathering::HarmFixture::WardCancellation,
    ));
    let mut app = host_with_table(
        Table::new(session.clone(), directory.path().join("motion.json")),
        false,
    );
    assert!(choice_button(&mut app, "Pause motion").is_none());
    let surrounds = |app: &mut App| {
        let world = app.world_mut();
        world
            .query::<(&Name, &UiTransform)>()
            .iter(world)
            .filter(|(name, _)| name.as_str() == "Encounter tendril")
            .map(|(_, transform)| *transform)
            .collect::<Vec<_>>()
    };
    let initial = surrounds(&mut app);
    assert!(!initial.is_empty());
    for _ in 0..20 {
        draws::tick(&mut app, 0.05);
    }
    assert_ne!(surrounds(&mut app), initial);
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
    let before = surrounds(&mut app);
    draws::tick(&mut app, 60.);
    assert_eq!(surrounds(&mut app), before);
    app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
    draws::tick(&mut app, 60.);
    assert_eq!(
        surrounds(&mut app),
        before,
        "The first returning frame cannot inherit unfocused time"
    );
    draws::tick(&mut app, 0.05);
    assert_ne!(surrounds(&mut app), before);
    assert_eq!(app.world().resource::<Table>().session, session);
}

#[test]
fn on_the_hunt_search_is_not_a_draw_but_its_fallback_is() {
    for fixture in [SearchFixture::HuntTargets, SearchFixture::HuntNoMatch] {
        let directory = tempfile::tempdir().unwrap();
        let session = Session::with_hostile_fixture(HostileFixture::Search(fixture));
        let mut app = host_with_table(
            Table::new(session, directory.path().join("hunt.json")),
            false,
        );
        activate_choice(&mut app, "mythos:draw");
        let hunt = app
            .world()
            .resource::<Table>()
            .view()
            .choices
            .iter()
            .find(|choice| choice.id.starts_with("search:play-hunt:"))
            .unwrap()
            .id
            .clone();
        activate_choice(&mut app, &hunt);
        finish_presentation(&mut app);
        let view = app.world().resource::<Table>().view();
        assert_eq!(view.play.as_ref().unwrap().searched_encounter.len(), 9);
        assert!(!rendered_text(&mut app).contains("ENCOUNTER CARD"));
        if fixture == SearchFixture::HuntTargets {
            let enemy = view
                .play
                .unwrap()
                .searched_encounter
                .into_iter()
                .find(|card| card.code == "01160")
                .unwrap();
            activate_choice(&mut app, &format!("search:enemy:{}", enemy.id));
            finish_presentation(&mut app);
            assert!(!rendered_text(&mut app).contains("ENCOUNTER CARD"));
            assert!(
                app.world()
                    .resource::<Table>()
                    .view()
                    .play
                    .unwrap()
                    .scenario
                    .enemies
                    .iter()
                    .any(|placed| placed.card.id == enemy.id
                        && placed.engaged == Some(Investigator::Tony))
            );
        } else {
            activate_choice(&mut app, "search:no-match");
            wait_for_reading(&mut app);
            assert!(rendered_text(&mut app).contains("Drawn by Tony Morgan"));
            continue_reading(&mut app);
            finish_presentation(&mut app);
        }
    }
}

#[test]
fn reduced_motion_and_small_viewports_keep_the_reading_controls_and_effective_text_usable() {
    let directory = tempfile::tempdir().unwrap();
    let session = Session::with_hostile_fixture(HostileFixture::Harm(
        crate::gathering::HarmFixture::WardCancellation,
    ));
    let mut table = Table::new(session.clone(), directory.path().join("small.json"));
    table.reduced_motion = true;
    let mut app = host_with_table(table, false);
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    app.insert_resource(art::Art::with_images_for_test([(
        "card-01163",
        image.clone(),
    )]));
    app.world_mut().resource_mut::<Table>().dirty = true;
    for size in [
        Vec2::new(1600., 1000.),
        Vec2::new(900., 600.),
        Vec2::new(480., 640.),
    ] {
        crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, 1.);
        for _ in 0..3 {
            app.update();
        }
        let world = app.world_mut();
        let (_, frame, transform) = world
            .query::<(&ImageNode, &ComputedNode, &UiTransform)>()
            .iter(world)
            .find(|(node, _, _)| node.image == image)
            .expect("The enlarged face uses the prepared card image");
        assert!((frame.size().x / frame.size().y - 5. / 7.).abs() < 0.001);
        assert_eq!(transform.rotation, Rot2::IDENTITY);
        assert_unclipped_button(&mut app, "Continue", size, 1.);
        assert_unclipped_button(&mut app, "Read rules", size, 1.);
        assert!(choice_button(&mut app, "Pause motion").is_none());
        let button = choice_button(&mut app, "Read rules").unwrap();
        app.world_mut().trigger(Activate { entity: button });
        app.update();
        assert!(rendered_text(&mut app).contains("For each point you fail by, take 1 horror."));
        assert_unclipped_button(&mut app, "Continue", size, 1.);
        let button = choice_button(&mut app, "Show card").unwrap();
        app.world_mut().trigger(Activate { entity: button });
        app.update();
    }
    assert_eq!(app.world().resource::<Table>().session, session);
}

#[test]
fn encounter_response_buttons_and_their_labels_fit_without_overlap() {
    let point = review_saves::draw_review::encounter_checkpoint(77).unwrap();
    let (_directory, mut app) = open_reading_checkpoint(&point.session, "01163");
    for (size, scale) in [
        (Vec2::new(1600., 1000.), 1.),
        (Vec2::new(1280., 800.), 2.),
        (Vec2::new(900., 600.), 1.),
        (Vec2::new(480., 640.), 1.),
    ] {
        crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, scale);
        for _ in 0..3 {
            app.update();
        }
        let world = app.world_mut();
        let buttons: Vec<_> = world
            .query_filtered::<(Entity, &AccessibleLabel, &ComputedNode, &UiGlobalTransform), With<Button>>()
            .iter(world)
            .filter(|(entity, _, _, _)| {
                let mut parent = world.get::<ChildOf>(*entity).map(ChildOf::parent);
                while let Some(ancestor) = parent {
                    if world.get::<TabGroup>(ancestor).is_some_and(|group| group.modal) {
                        return true;
                    }
                    parent = world.get::<ChildOf>(ancestor).map(ChildOf::parent);
                }
                false
            })
            .map(|(entity, label, node, transform)| {
                (entity, label.0.clone(), Rect {
                    min: transform.transform_point2(-node.size() / 2.) / scale,
                    max: transform.transform_point2(node.size() / 2.) / scale,
                })
            })
            .collect();
        assert_eq!(
            buttons.len(),
            4,
            "Continue, Read rules and both legal Ward copies"
        );
        for (index, (entity, label, bounds)) in buttons.iter().enumerate() {
            assert!(
                bounds.min.cmpge(Vec2::splat(4.)).all()
                    && bounds.max.cmple(size - Vec2::splat(4.)).all(),
                "{label}: {bounds:?}"
            );
            for child in world.get::<Children>(*entity).unwrap() {
                if world.get::<Text>(*child).is_some() {
                    let ink = crate::tabletop_layout::heading_checks::rendered_text_bounds(
                        world, *child, scale,
                    );
                    assert!(
                        ink.min.cmpge(bounds.min).all() && ink.max.cmple(bounds.max).all(),
                        "{label}'s text {ink:?} exceeds its button {bounds:?}"
                    );
                }
            }
            for (_, other_label, other) in &buttons[index + 1..] {
                assert!(
                    bounds.max.x + 8. <= other.min.x + 0.1
                        || other.max.x + 8. <= bounds.min.x + 0.1
                        || bounds.max.y + 8. <= other.min.y + 0.1
                        || other.max.y + 8. <= bounds.min.y + 0.1,
                    "{label} {bounds:?} and {other_label} {other:?} need separate focus outlines"
                );
            }
        }
    }
}

fn review_points() -> Vec<(&'static str, Session, &'static str)> {
    [
        ("rotting-remains", 77, "01163"),
        ("ancient-evils", 6, "01166"),
        ("ghoul-minion", 16, "01160"),
    ]
    .into_iter()
    .map(|(name, seed, code)| {
        let point = review_saves::draw_review::encounter_checkpoint(seed).unwrap();
        assert_eq!(point.trigger, "mythos:draw");
        (name, point.session, code)
    })
    .collect()
}

fn open_reading_checkpoint(session: &Session, code: &str) -> (tempfile::TempDir, App) {
    let (directory, mut app) =
        review_saves::open_review_save_in_view(session, Some("scenario")).unwrap();
    wait_for_reading(&mut app);
    let table = app.world().resource::<Table>();
    let drawn = table.presentation.encounter().current().unwrap();
    assert_eq!(drawn.card.code, code);
    assert_eq!(drawn.drawn_by, Investigator::Jacqueline);
    assert_eq!(
        continuity::load_session(&table.save_path).unwrap(),
        table.session
    );
    (directory, app)
}

#[test]
fn encounter_drawer_closes_when_its_only_card_moves_to_test() {
    let point = review_saves::draw_review::encounter_checkpoint(77).unwrap();
    for reduced in [false, true] {
        let (_directory, mut app) = open_reading_checkpoint(&point.session, "01163");
        app.world_mut().resource_mut::<Table>().reduced_motion = reduced;
        let card = app
            .world()
            .resource::<Table>()
            .presentation
            .encounter()
            .current()
            .unwrap()
            .card
            .id;
        let world = app.world_mut();
        assert!(
            world
                .query::<(&Name, &Visibility)>()
                .iter(world)
                .any(|(name, visibility)| name.as_str() == "Resolution tray"
                    && *visibility != Visibility::Hidden),
            "The encounter arrives in its open drawer before reading"
        );

        continue_reading(&mut app);
        settle_automatic_resolution(&mut app);
        let committed = app.world().resource::<Table>().session.clone();
        let (_reopened_directory, mut reopened) = review_saves::open_review_save(&committed);
        for current in [&mut app, &mut reopened] {
            assert_eq!(current.world().resource::<Table>().session, committed);
            assert!(
                draws::physical(current, card)
                    .translation
                    .abs_diff_eq(layout::test_source(), 0.001)
            );
            assert_eq!(
                (
                    button_with_label(current, "Resolve").is_some(),
                    rendered_text(current).contains("Resolving"),
                ),
                (false, false),
                "An empty drawer must leave neither its navigation shortcut nor its caption"
            );
            let world = current.world_mut();
            assert!(
                world
                    .query::<(&Name, &Visibility)>()
                    .iter(world)
                    .filter(|(name, _)| name.as_str() == "Resolution tray")
                    .all(|(_, visibility)| *visibility == Visibility::Hidden),
                "The empty investigator drawer must hide once Rotting Remains reaches Test"
            );
        }
    }
}

#[test]
fn ordinary_encounter_review_saves_preserve_reading_reactions_tests_doom_and_engagement() {
    for (_, session, code) in review_points() {
        let (_directory, mut app) = open_reading_checkpoint(&session, code);
        let card = app
            .world()
            .resource::<Table>()
            .presentation
            .encounter()
            .current()
            .unwrap()
            .card
            .clone();
        let before = app.world().resource::<Table>().session.clone();
        if code == "01163" {
            let (_reopened_directory, mut reopened) = review_saves::open_review_save(&before);
            assert!(rendered_text(&mut reopened).contains("ENCOUNTER CARD"));
            assert_eq!(reopened.world().resource::<Table>().session, before);
        }
        continue_reading(&mut app);
        finish_presentation(&mut app);
        match code {
            "01163" => {
                let play = app.world().resource::<Table>().view().play.unwrap();
                let test = play
                    .test
                    .expect("Continue must start the encounter test directly");
                assert_eq!(test.tester, Investigator::Jacqueline);
                assert_eq!(test.difficulty, 3);
                let saved = app.world().resource::<Table>().session.clone();
                let (_directory, mut reopened) = review_saves::open_review_save(&saved);
                assert!(!rendered_text(&mut reopened).contains("ENCOUNTER CARD"));
            }
            "01166" => {
                let play = app.world().resource::<Table>().view().play.unwrap();
                assert_eq!(play.agenda_doom, 2);
                assert_eq!(
                    play.encounter_discard
                        .iter()
                        .filter(|discard| discard.id == card.id)
                        .count(),
                    1
                );
            }
            "01160" => {
                assert_eq!(app.world().resource::<Table>().session, before);
                let play = app.world().resource::<Table>().view().play.unwrap();
                assert!(
                    play.scenario
                        .enemies
                        .iter()
                        .any(|enemy| enemy.card.id == card.id
                            && enemy.engaged == Some(Investigator::Jacqueline))
                );
            }
            _ => unreachable!(),
        }
        assert!(!rendered_text(&mut app).contains("ENCOUNTER CARD"));
    }
}

#[test]
#[ignore = "exports ordinary encounter checkpoints to ARKHAM_ENCOUNTER_REVIEW_DIR"]
fn export_encounter_review_saves() {
    let output = PathBuf::from(
        std::env::var_os("ARKHAM_ENCOUNTER_REVIEW_DIR").expect("Set a fresh directory"),
    );
    std::fs::create_dir(&output).unwrap();
    for (name, session, code) in review_points() {
        let path = output.join(format!("{name}.json"));
        continuity::save_session(&path, &session).unwrap();
        // Production startup runs on an exact loaded copy: preserve the pre-draw
        // source, since automatic startup legitimately commits the impending draw.
        let loaded = continuity::load_session(&path).unwrap();
        assert_eq!(loaded, session);
        let (_directory, app) = open_reading_checkpoint(&loaded, code);
        if code == "01163" {
            let pending = app.world().resource::<Table>().session.clone();
            continuity::save_session(output.join("rotting-remains-pending.json"), &pending)
                .unwrap();
            let (_directory, mut reopened) = review_saves::open_review_save(&pending);
            assert!(rendered_text(&mut reopened).contains("ENCOUNTER CARD"));
        }
        println!(
            "{}: revision {}, expected card {code}",
            path.display(),
            session.observe().revision
        );
    }
}
