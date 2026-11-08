use super::*;
use crate::gathering::{HostileFixture, SearchFixture};

pub(super) fn continue_reading(app: &mut App) {
    assert!(rendered_text(app).contains("ENCOUNTER CARD"));
    let entity = choice_button(app, "Continue").unwrap();
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
fn reopening_an_unstarted_treachery_keeps_cancellation_and_acknowledgment_never_saves() {
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
    let entity = choice_button(&mut app, "Continue").unwrap();
    app.world_mut().trigger(Activate { entity });
    app.world_mut().trigger(Activate { entity });
    app.world_mut().resource_mut::<Actions>().0.push(old_choice);
    app.update();
    assert_eq!(app.world().resource::<Table>().session, session);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(app.world().resource::<Table>().view().choices, view.choices);

    // An unchanged in-place reopen and scene rebuild do not manufacture a draw.
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Reopen);
    app.update();
    app.world_mut().resource_mut::<Table>().dirty = true;
    app.update();
    assert!(!rendered_text(&mut app).contains("ENCOUNTER CARD"));
    activate_choice(&mut app, &ward);
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
fn encounter_surround_can_pause_and_resumes_without_catching_up_after_focus_loss() {
    let directory = tempfile::tempdir().unwrap();
    let session = Session::with_hostile_fixture(HostileFixture::Harm(
        crate::gathering::HarmFixture::WardCancellation,
    ));
    let mut app = host_with_table(
        Table::new(session.clone(), directory.path().join("motion.json")),
        false,
    );
    let pause = choice_button(&mut app, "Pause motion")
        .expect("Motion must be controllable without dismissing the card");
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
    app.world_mut().trigger(Activate { entity: pause });
    app.update();
    let paused = surrounds(&mut app);
    draws::tick(&mut app, 30.);
    assert_eq!(surrounds(&mut app), paused);
    let resume = choice_button(&mut app, "Resume motion").unwrap();
    app.world_mut().trigger(Activate { entity: resume });
    app.update();
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
        assert_eq!(app.world().resource::<Table>().session, before);
        match code {
            "01163" => {
                assert!(
                    app.world()
                        .resource::<Table>()
                        .view()
                        .choices
                        .iter()
                        .any(|choice| choice.id.starts_with("ward:"))
                );
                activate_choice(&mut app, "revelation:resolve");
                finish_presentation(&mut app);
                let play = app.world().resource::<Table>().view().play.unwrap();
                let test = play.test.unwrap();
                assert_eq!(test.tester, Investigator::Jacqueline);
                assert_eq!(test.difficulty, 3);
                let saved = app.world().resource::<Table>().session.clone();
                let (_directory, mut reopened) = review_saves::open_review_save(&saved);
                assert!(!rendered_text(&mut reopened).contains("ENCOUNTER CARD"));
            }
            "01166" => {
                assert_eq!(
                    app.world()
                        .resource::<Table>()
                        .view()
                        .play
                        .unwrap()
                        .agenda_doom,
                    1
                );
                activate_choice(&mut app, "revelation:resolve");
                finish_presentation(&mut app);
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
