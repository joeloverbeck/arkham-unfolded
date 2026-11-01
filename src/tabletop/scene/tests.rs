use super::super::*;
use super::*;
use crate::tabletop_pacing::PresentationTime as Time;
use std::time::Duration;

use crate::tabletop_standee_checks as standee_checks;

mod encounter_discard;

#[test]
fn production_test_heading_clears_surfaces_after_text_layout() {
    use crate::tabletop_layout::heading_checks;

    let directory = tempfile::tempdir().unwrap();
    for scale in [1.0, 2.0] {
        let mut app = app_for_session_in(
            heading_checks::app(),
            Session::new(Fixture::curse()),
            directory.path().join(format!("heading-{scale}.json")),
        );
        heading_checks::assert_heading(&mut app, MainCamera, scale);
    }
}

fn pending_app(path: PathBuf) -> App {
    let mut session = Session::new(Fixture::curse());
    session.choose(0, "commit-promise").unwrap();
    session.choose(1, "use-jacqueline").unwrap();
    app_for_session(session, path)
}

#[test]
fn promise_commitment_shows_willpower_bonus_and_keeps_its_curse_consequence() {
    let directory = tempfile::tempdir().unwrap();
    for wide in [false, true] {
        for symbols in [false, true] {
            let mut app = app_for_session(
                Session::new(Fixture::curse()),
                directory
                    .path()
                    .join(format!("promise-{wide}-{symbols}.json")),
            );
            app.world_mut().resource_mut::<Art>().symbol_font = symbols.then(Handle::default);
            let world = app.world_mut();
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(if wide { 1280.0 } else { 700.0 }, 720.0);
            let before = app.world().resource::<Tabletop>().view();
            act(
                &mut app,
                Action::Select(Source::Card(1), Vec2::new(200.0, 200.0)),
            );
            let expected_label = if wide {
                "Commit +4 willpower"
            } else {
                "Commit Promise of Power (+4 willpower; add a curse)"
            };
            let world = app.world_mut();
            let (text, children) = world
                .query::<(&AccessibleLabel, &Text, Option<&Children>)>()
                .iter(world)
                .find(|(label, _, _)| label.0 == expected_label)
                .map(|(_, text, children)| (text, children))
                .unwrap_or_else(|| panic!("missing Promise skill label: {expected_label}"));
            let mut visible = text.0.clone();
            if let Some(children) = children {
                for child in children {
                    if let Some(span) = world.get::<TextSpan>(*child) {
                        visible.push_str(&span.0);
                    }
                }
            }
            assert_eq!(
                visible,
                if symbols {
                    expected_label.replace("willpower", "\u{2060}f\u{2060}")
                } else {
                    expected_label.into()
                }
            );
            assert!(
                world
                    .query::<&AccessibleLabel>()
                    .iter(world)
                    .any(|label| label.0 == "Commit Promise of Power (+4 willpower; add a curse)")
            );
            assert_eq!(app.world().resource::<Tabletop>().view(), before);
            let generation = app.world().resource::<Context>().generation();
            act(
                &mut app,
                Action::ContextChoose(generation, before.revision, "commit-promise".into()),
            );
            let after = app.world().resource::<Tabletop>().view();
            assert_eq!(after.skill_value, 9);
            assert_eq!(after.bag_count, before.bag_count + 1);
        }
    }
}

#[test]
fn production_scene_keeps_physical_standees_and_readable_missing_portraits() {
    use crate::tabletop_layout::InvestigatorClass::{Mystic, Rogue, Survivor};

    let dir = tempfile::tempdir().unwrap();
    for missing in [None, Some(1)] {
        let mut app = app_for_session(
            Session::new(Fixture::curse()),
            dir.path().join("standees.json"),
        );
        app.init_resource::<Assets<Image>>();
        let mut expected = Vec::new();
        for (index, (name, class)) in [
            ("Tony Morgan", Rogue),
            ("Patrice Hathaway", Survivor),
            ("Jacqueline Fine", Mystic),
        ]
        .into_iter()
        .enumerate()
        {
            let image = (missing != Some(index)).then(|| {
                app.world_mut()
                    .resource_mut::<Assets<Image>>()
                    .add(Image::default())
            });
            if let Some(image) = &image {
                app.world_mut()
                    .resource_mut::<Art>()
                    .investigators
                    .push((name.into(), image.clone()));
            }
            expected.push((class, image));
        }
        app.update();
        standee_checks::assert_standees(app.world_mut(), &expected);
        for name in ["Tony", "Patrice", "Jacqueline"] {
            assert!(
                app.world_mut()
                    .query_filtered::<&Text, With<WorldLabel>>()
                    .iter(app.world())
                    .any(|label| label.0.contains(name)),
                "{name} remains readable with or without a portrait"
            );
        }
    }
}

#[test]
fn named_location_markers_keep_elsewhere_patrice_separate_from_study() {
    let dir = tempfile::tempdir().unwrap();
    for elsewhere in [false, true] {
        let fixture = if elsewhere {
            Fixture::curse().with_patrice_elsewhere()
        } else {
            Fixture::curse()
        };
        let mut app = app_for_session(Session::new(fixture), dir.path().join("locations.json"));
        let choices = app.world().resource::<Tabletop>().view().choices;
        app.update();
        assert_eq!(app.world().resource::<Tabletop>().view().choices, choices);
        let world = app.world_mut();
        let markers: Vec<_> = world
            .query::<&Name>()
            .iter(world)
            .filter(|name| name.as_str().starts_with("Location marker:"))
            .map(|name| name.as_str().to_owned())
            .collect();
        assert!(markers.iter().any(|name| name == "Location marker: Study"));
        assert_eq!(
            markers
                .iter()
                .any(|name| name == "Location marker: Hallway"),
            elsewhere
        );
        let labels: Vec<_> = world
            .query::<(&Text, &WorldLabel, &StandeeLabel)>()
            .iter(world)
            .map(|(text, anchor, _)| (text.0.clone(), anchor.0))
            .collect();
        let patrice = labels
            .iter()
            .find(|(text, _)| text.starts_with("Patrice\n"))
            .unwrap()
            .1;
        let tony = labels
            .iter()
            .find(|(text, _)| text.starts_with("Tony\n"))
            .unwrap()
            .1;
        assert!(tony.x > HALLWAY_AT.x + LOCATION_SIZE.x);
        if elsewhere {
            assert!(patrice.x < HALLWAY_AT.x);
        } else {
            assert!((patrice.x - tony.x).abs() < 0.001);
            assert!(patrice.z > tony.z);
        }
        let roots: Vec<_> = world
            .query::<(&Name, &Transform)>()
            .iter(world)
            .filter(|(name, _)| name.as_str() == "Investigator standee")
            .map(|(_, transform)| *transform)
            .collect();
        assert_eq!(roots.len(), 3);
        for root in roots {
            assert!(
                root.scale.x > 0.38,
                "standees enlarge beyond the previous scale"
            );
            assert_eq!(root.scale, Vec3::splat(root.scale.x));
        }
    }
}

#[test]
fn study_marker_clears_rotting_remains_card_footprint() {
    use bevy::mesh::VertexAttributeValues;

    let dir = tempfile::tempdir().unwrap();
    let mut app = app_for_session(
        Session::new(Fixture::curse()),
        dir.path().join("spacing.json"),
    );
    app.update();
    let world = app.world_mut();
    let (study_mesh, study_transform) = world
        .query::<(&Name, &Mesh3d, &Transform)>()
        .iter(world)
        .find(|(name, _, _)| name.as_str() == "Location marker: Study")
        .map(|(_, mesh, transform)| (mesh.0.clone(), *transform))
        .expect("the production scene must show the occupied Study");
    let remains = CARDS.iter().position(|card| card.code == "01163").unwrap();
    let (card_mesh, card_transform) = world
        .query::<(&context::Selectable, &Mesh3d, &Transform)>()
        .iter(world)
        .find(|(source, _, _)| source.0 == Source::Card(remains))
        .map(|(_, mesh, transform)| (mesh.0.clone(), *transform))
        .expect("the production scene must show Rotting Remains");
    let meshes = world.resource::<Assets<Mesh>>();
    let footprint = |handle: &Handle<Mesh>, transform: Transform| {
        let mesh = meshes.get(handle).unwrap();
        let Some(VertexAttributeValues::Float32x3(vertices)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("the rendered footprint must have vertex positions");
        };
        assert!(!vertices.is_empty());
        vertices.iter().fold(
            (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
            |(min, max), vertex| {
                let position = transform.transform_point(Vec3::from_array(*vertex)).xz();
                (min.min(position), max.max(position))
            },
        )
    };
    let (study_min, study_max) = footprint(&study_mesh, study_transform);
    let (card_min, card_max) = footprint(&card_mesh, card_transform);
    assert!(
        study_max.x < card_min.x
            || card_max.x < study_min.x
            || study_max.y < card_min.y
            || card_max.y < study_min.y,
        "Study and Rotting Remains need a positive gap on the table: \
         Study {study_min:?}..{study_max:?}, card {card_min:?}..{card_max:?}"
    );
}

#[test]
fn contextual_controls_preserve_every_offered_choice_and_token_identity() {
    for (fixture, expected) in [
        (
            Fixture::curse(),
            vec!["cancel-0-1", "cancel-0-2", "cancel-1-2"],
        ),
        (Fixture::auto_fail(), vec!["cancel-0", "cancel-1-2"]),
    ] {
        let mut session = Session::new(fixture);
        for next in [Some("commit-promise"), Some("use-jacqueline"), None] {
            let view = session.observe();
            let mut exposed: Vec<_> = view
                .choices
                .iter()
                .filter(|choice| context::source_less(&choice.id))
                .map(|choice| choice.id.as_str())
                .collect();
            for source in [
                Source::Card(0),
                Source::Card(1),
                Source::Card(2),
                Source::Bag,
                Source::Token(0),
            ] {
                exposed.extend(
                    context::choices(&view, source)
                        .iter()
                        .map(|choice| choice.id.as_str()),
                );
            }
            exposed.sort();
            let mut offered: Vec<_> = view
                .choices
                .iter()
                .map(|choice| choice.id.as_str())
                .collect();
            offered.sort();
            assert_eq!(
                exposed, offered,
                "the physical and source-less controls cover exactly the legal choices"
            );
            if let Some(id) = next {
                session.choose(view.revision, id).unwrap();
            }
        }
        let view = session.observe();
        for token in &view.tokens {
            let ids: Vec<_> = context::choices(&view, Source::Token(token.id))
                .iter()
                .map(|choice| choice.id.as_str())
                .collect();
            assert_eq!(
                ids, expected,
                "each physical token exposes all complete cancellation alternatives"
            );
        }
    }
}

#[test]
fn inspect_only_cards_open_directly_while_available_choices_keep_the_menu() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_for_session(
        Session::new(Fixture::curse()),
        dir.path().join("inspect.json"),
    );
    app.update();
    let before = app.world().resource::<Tabletop>().view();
    act(&mut app, Action::Select(Source::Card(0), Vec2::ZERO));
    assert_eq!(app.world().resource::<Tabletop>().inspect, Some(0));
    assert!(!app.world().resource::<Context>().menu_open());
    act(&mut app, Action::CloseInspect);
    assert_eq!(app.world().resource::<Tabletop>().view(), before);

    act(&mut app, Action::Select(Source::Card(1), Vec2::ZERO));
    assert_eq!(app.world().resource::<Tabletop>().inspect, None);
    assert!(app.world().resource::<Context>().menu_open());
    let generation = app.world().resource::<Context>().generation();
    act(
        &mut app,
        Action::ContextChoose(generation, before.revision, "commit-promise".into()),
    );
    let committed = app.world().resource::<Tabletop>().view();
    assert!(committed.promise_committed);
    act(&mut app, Action::Select(Source::Card(1), Vec2::ZERO));
    assert_eq!(app.world().resource::<Tabletop>().inspect, Some(1));
    assert!(!app.world().resource::<Context>().menu_open());
    act(&mut app, Action::CloseInspect);
    assert_eq!(app.world().resource::<Tabletop>().view(), committed);

    act(&mut app, Action::Save);
    act(&mut app, Action::Replay);
    act(&mut app, Action::Select(Source::Card(1), Vec2::ZERO));
    assert_eq!(app.world().resource::<Tabletop>().inspect, Some(1));
    assert!(!app.world().resource::<Context>().menu_open());
    assert_eq!(
        app.world().resource::<Tabletop>().session.observe(),
        committed
    );
}

#[test]
fn feedback_separates_menu_source_from_pending_cancellation_and_keeps_scene_entities() {
    use crate::tabletop_feedback::{CardFeedback, FeedbackSettings};
    let dir = tempfile::tempdir().unwrap();
    let mut app = pending_app(dir.path().join("feedback.json"));
    app.update();
    let marks = |app: &mut App| {
        app.world_mut()
            .query::<(Entity, &context::Selectable, &CardFeedback)>()
            .iter(app.world())
            .map(|(entity, source, mark)| (entity, source.0, mark.menu_source, mark.pending_choice))
            .collect::<Vec<_>>()
    };
    let before = marks(&mut app);
    assert!(
        before
            .iter()
            .all(|(_, _, source, pending)| !source && !pending)
    );
    act(&mut app, Action::Select(Source::Token(0), Vec2::ZERO));
    let generation = app.world().resource::<Context>().generation();
    act(
        &mut app,
        Action::PrepareCancellation(generation, "cancel-1-2".into()),
    );
    let selected = marks(&mut app);
    for (_, source, menu_source, pending) in &selected {
        assert_eq!(*menu_source, *source == Source::Token(0));
        assert_eq!(*pending, matches!(source, Source::Token(1 | 2)));
    }
    act(&mut app, Action::DismissContext);
    let dismissed = marks(&mut app);
    assert!(dismissed.iter().all(|(_, _, source, _)| !source));
    assert_eq!(
        before
            .iter()
            .map(|(entity, _, _, _)| entity)
            .collect::<Vec<_>>(),
        dismissed
            .iter()
            .map(|(entity, _, _, _)| entity)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        dismissed
            .iter()
            .filter(|(_, _, _, pending)| *pending)
            .count(),
        2
    );
    act(&mut app, Action::Select(Source::Card(0), Vec2::ZERO));
    assert!(app.world().resource::<FeedbackSettings>().blocked);
    assert_eq!(
        app.world().resource::<Context>().pending_cancellation(),
        Some("cancel-1-2")
    );
    act(&mut app, Action::CloseInspect);
    assert!(!app.world().resource::<FeedbackSettings>().blocked);
    act(&mut app, Action::Motion);
    assert!(app.world().resource::<FeedbackSettings>().reduced_motion);
    act(&mut app, Action::Save);
    act(&mut app, Action::Replay);
    assert!(
        marks(&mut app)
            .iter()
            .all(|(_, _, source, pending)| !source && !pending)
    );
}

#[test]
fn contextual_selection_inspection_and_cancellation_preserve_decision_until_confirmation() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = pending_app(dir.path().join("context.json"));
    app.update();
    let before = app.world().resource::<Tabletop>().session.observe();
    act(
        &mut app,
        Action::Select(Source::Token(0), Vec2::new(150.0, 200.0)),
    );
    let old_generation = app.world().resource::<Context>().generation();
    act(
        &mut app,
        Action::PrepareCancellation(old_generation, "cancel-1-2".into()),
    );
    assert_eq!(app.world().resource::<Tabletop>().session.observe(), before);
    act(&mut app, Action::DismissContext);
    act(&mut app, Action::Select(Source::Card(2), Vec2::ZERO));
    let card_generation = app.world().resource::<Context>().generation();
    act(
        &mut app,
        Action::ContextChoose(card_generation, before.revision, "cancel-1-2".into()),
    );
    assert_eq!(app.world().resource::<Tabletop>().session.observe(), before);
    act(&mut app, Action::Inspect(2));
    act(&mut app, Action::CloseInspect);
    let context = app.world().resource::<Context>();
    assert_eq!(context.selected_source(), Some(Source::Card(2)));
    assert_eq!(context.pending_cancellation(), Some("cancel-1-2"));
    assert!(!context.menu_open());
    act(
        &mut app,
        Action::ContextChoose(old_generation, before.revision, "cancel-1-2".into()),
    );
    assert_eq!(
        app.world().resource::<Tabletop>().session.observe(),
        before,
        "an old pre-inspection button cannot act"
    );
    act(&mut app, Action::Focus(tabletop_layout::Area::Test));
    act(
        &mut app,
        Action::Select(Source::Token(0), Vec2::new(150.0, 200.0)),
    );
    let generation = app.world().resource::<Context>().generation();
    assert_eq!(
        app.world().resource::<Context>().pending_cancellation(),
        Some("cancel-1-2")
    );
    act(
        &mut app,
        Action::ContextChoose(old_generation, before.revision, "cancel-1-2".into()),
    );
    assert_eq!(
        app.world().resource::<Tabletop>().session.observe(),
        before,
        "the same token, observation and staged cancellation still reject an old callback"
    );
    act(
        &mut app,
        Action::ContextChoose(generation, before.revision, "cancel-1-2".into()),
    );
    assert_eq!(
        app.world()
            .resource::<Tabletop>()
            .session
            .observe()
            .revision,
        before.revision + 1
    );
}

#[test]
fn contextual_actions_cannot_cross_replay_or_reopen_boundaries() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = pending_app(dir.path().join("modes.json"));
    app.update();
    act(&mut app, Action::Save);
    act(&mut app, Action::Select(Source::Token(0), Vec2::ZERO));
    let generation = app.world().resource::<Context>().generation();
    let before = app.world().resource::<Tabletop>().session.observe();
    act(
        &mut app,
        Action::PrepareCancellation(generation, "cancel-1-2".into()),
    );
    act(&mut app, Action::Replay);
    act(&mut app, Action::Select(Source::Token(0), Vec2::ZERO));
    act(
        &mut app,
        Action::ContextChoose(generation, before.revision, "cancel-1-2".into()),
    );
    act(&mut app, Action::Live);
    act(
        &mut app,
        Action::ContextChoose(generation, before.revision, "cancel-1-2".into()),
    );
    assert_eq!(app.world().resource::<Tabletop>().session.observe(), before);
    act(&mut app, Action::Select(Source::Token(0), Vec2::ZERO));
    let generation = app.world().resource::<Context>().generation();
    act(
        &mut app,
        Action::PrepareCancellation(generation, "cancel-1-2".into()),
    );
    act(&mut app, Action::Reopen);
    act(
        &mut app,
        Action::ContextChoose(generation, before.revision, "cancel-1-2".into()),
    );
    assert_eq!(app.world().resource::<Tabletop>().session.observe(), before);
}

#[test]
fn contextual_cancellation_survives_failed_commits_and_manual_save() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = pending_app(dir.path().to_path_buf()); // A directory cannot be a save file.
    act(&mut app, Action::Select(Source::Token(0), Vec2::ZERO));
    let generation = app.world().resource::<Context>().generation();
    let before = app.world().resource::<Tabletop>().session.observe();
    act(
        &mut app,
        Action::PrepareCancellation(generation, "cancel-1-2".into()),
    );
    for revision in [before.revision - 1, before.revision] {
        act(
            &mut app,
            Action::ContextChoose(generation, revision, "cancel-1-2".into()),
        );
        let table = app.world().resource::<Tabletop>();
        assert_eq!(table.session.observe(), before);
        assert!(table.notice.starts_with("Decision was not committed:"));
    }
    act(&mut app, Action::Save);
    assert!(
        app.world()
            .resource::<Tabletop>()
            .notice
            .starts_with("Save failed;")
    );

    let path = dir.path().join("saved.json");
    app.world_mut().resource_mut::<Tabletop>().save_path = path.clone();
    act(&mut app, Action::Save);
    assert_eq!(continuity::load_session(&path).unwrap().observe(), before);
    assert_eq!(
        app.world().resource::<Context>().pending_cancellation(),
        Some("cancel-1-2")
    );
    act(
        &mut app,
        Action::ContextChoose(generation, before.revision, "cancel-1-2".into()),
    );
    let after = app.world().resource::<Tabletop>().session.observe();
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(continuity::load_session(&path).unwrap().observe(), after);
}

#[test]
fn contextual_failed_and_no_op_transitions_expire_callbacks_before_attempting_the_operation() {
    let dir = tempfile::tempdir().unwrap();
    let blocked = dir.path().join("not-a-directory");
    std::fs::write(&blocked, "blocks all session and replay writes").unwrap();
    for (index, transition) in [
        Action::Reopen,
        Action::Replay,
        Action::New,
        Action::Back,
        Action::Next,
        Action::Live,
    ]
    .into_iter()
    .enumerate()
    {
        let mut app = pending_app(blocked.join("session.json"));
        act(&mut app, Action::Select(Source::Token(0), Vec2::ZERO));
        let old = app.world().resource::<Context>().generation();
        let before = app.world().resource::<Tabletop>().session.observe();
        act(
            &mut app,
            Action::PrepareCancellation(old, "cancel-1-2".into()),
        );
        act(&mut app, transition);
        if index < 3 {
            assert!(
                app.world()
                    .resource::<Tabletop>()
                    .notice
                    .starts_with("Could not")
            );
        }
        assert_eq!(app.world().resource::<Tabletop>().view(), before);

        // Make persistence possible so refusal cannot be attributed to a bad path.
        app.world_mut().resource_mut::<Tabletop>().save_path =
            dir.path().join(format!("retry-{index}.json"));
        act(
            &mut app,
            Action::SourceLess(old, before.revision, "end".into()),
        );
        assert_eq!(app.world().resource::<Tabletop>().session.observe(), before);
        act(&mut app, Action::Select(Source::Token(0), Vec2::ZERO));
        let current = app.world().resource::<Context>().generation();
        act(
            &mut app,
            Action::ContextChoose(current, before.revision, "cancel-1-2".into()),
        );
        assert_eq!(
            app.world().resource::<Tabletop>().session.observe(),
            before,
            "a failed or no-op mode transition discards the staged cancellation"
        );
        act(
            &mut app,
            Action::PrepareCancellation(current, "cancel-1-2".into()),
        );
        act(
            &mut app,
            Action::ContextChoose(current, before.revision, "cancel-1-2".into()),
        );
        assert_eq!(
            app.world()
                .resource::<Tabletop>()
                .session
                .observe()
                .revision,
            before.revision + 1
        );
    }
}

#[test]
fn contextual_callbacks_are_checked_after_each_action_in_one_update() {
    let dir = tempfile::tempdir().unwrap();
    for preceding in [
        Action::Select(Source::Token(0), Vec2::new(20.0, 30.0)),
        Action::DismissContext,
    ] {
        let mut app = pending_app(dir.path().join("queued.json"));
        act(&mut app, Action::Select(Source::Token(0), Vec2::ZERO));
        let generation = app.world().resource::<Context>().generation();
        let before = app.world().resource::<Tabletop>().session.observe();
        act(
            &mut app,
            Action::PrepareCancellation(generation, "cancel-1-2".into()),
        );
        app.world_mut().resource_mut::<Actions>().0.extend([
            preceding,
            Action::ContextChoose(generation, before.revision, "cancel-1-2".into()),
        ]);
        app.update();
        assert_eq!(app.world().resource::<Tabletop>().session.observe(), before);
    }

    let path = dir.path().join("commit.json");
    let mut app = app_for_session(Session::new(Fixture::curse()), path.clone());
    act(&mut app, Action::Select(Source::Card(1), Vec2::ZERO));
    let generation = app.world().resource::<Context>().generation();
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::ContextChoose(generation, 0, "commit-promise".into()),
        // End remains offered after commitment; even a matching new revision
        // cannot make a callback from before the successful choice current.
        Action::SourceLess(generation, 1, "end".into()),
    ]);
    app.update();
    let after = app.world().resource::<Tabletop>().session.observe();
    assert_eq!(after.stage, Stage::BeforeReveal);
    assert_eq!(after.revision, 1);
    assert_eq!(continuity::load_session(&path).unwrap().observe(), after);
}

#[test]
fn promise_moves_from_patrice_hand_to_shared_test_without_changing_rules() {
    let mut session = Session::new(Fixture::curse());
    let before = session.observe();
    let hand = context::card_position(1, &before);
    let patrice = tabletop_layout::investigator_center(1, 3);
    assert_eq!(hand.x, patrice.x, "a single hand card is centered");
    assert!(hand.z > patrice.z);
    session.choose(before.revision, "commit-promise").unwrap();
    let committed = context::card_position(1, &session.observe());
    assert!((committed.z - tabletop_layout::TEST_CENTER.z).abs() < 4.5);
    assert!(committed.z < patrice.z - 9.0);
    assert_eq!(
        context::card_position(2, &before),
        context::card_position(2, &session.observe())
    );
}

#[test]
fn production_scene_places_test_objects_and_personal_status_on_their_surfaces() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = app_for_session(
        Session::new(Fixture::curse()),
        directory.path().join("surface-layout.json"),
    );
    app.add_systems(Startup, setup_table);
    app.update();
    let world = app.world_mut();
    let position = |world: &mut World, source| {
        world
            .query::<(&context::Selectable, &Transform)>()
            .iter(world)
            .find(|(selectable, _)| selectable.0 == source)
            .map(|(_, transform)| transform.translation)
            .expect("the real scene must contain this selectable object")
    };
    let bag = position(world, Source::Bag);
    let test = tabletop_layout::TEST_CENTER;
    assert!(bag.x > test.x && bag.z < test.z, "bag is upper right");
    assert!((bag.x - test.x).abs() < 7.0 && (bag.z - test.z).abs() < 4.5);
    let source = position(world, Source::Card(0));
    assert!(
        source.x < test.x && source.z < test.z,
        "source is upper left"
    );
    let jacqueline = tabletop_layout::investigator_center(2, 3);
    let investigator = position(world, Source::Card(2));
    assert!(investigator.x < jacqueline.x && investigator.z < jacqueline.z);
    let labels: Vec<_> = world
        .query::<(&Text, &WorldLabel)>()
        .iter(world)
        .map(|(text, anchor)| (text.0.clone(), anchor.0))
        .collect();
    for (index, name) in ["Tony Morgan", "Patrice Hathaway", "Jacqueline Fine"]
        .into_iter()
        .enumerate()
    {
        let center = tabletop_layout::investigator_center(index, 3);
        assert!(
            labels
                .iter()
                .any(|(text, anchor)| text == name && anchor.z < center.z - 7.5)
        );
        assert!(labels.iter().any(|(text, anchor)| text.contains("Horror")
            && *anchor == tabletop_layout::investigator_harm(index, true) + Vec3::Z * 0.4));
    }
    assert!(labels.iter().any(|(text, _)| text == "Horror\n2/9"));
    assert!(labels.iter().any(|(text, _)| text == "Damage\nn/a"));
    assert_eq!(
        world
            .query_filtered::<Entity, With<crate::tabletop_harm_tokens::HarmToken>>()
            .iter(world)
            .count(),
        6
    );
    assert!(labels.iter().any(|(text, _)| text == "Revealed"));
    assert!(labels.iter().any(|(text, _)| text == "Canceled"));

    act(&mut app, Action::Choose(0, "commit-promise".into()));
    act(&mut app, Action::Choose(1, "use-jacqueline".into()));
    let world = app.world_mut();
    for id in 0..3 {
        assert_eq!(
            position(world, Source::Token(id)),
            tabletop_layout::test_token(false, usize::from(id))
        );
    }
    act(&mut app, Action::Choose(2, "cancel-1-2".into()));
    let world = app.world_mut();
    assert_eq!(
        position(world, Source::Token(0)),
        tabletop_layout::test_token(false, 0)
    );
    assert_eq!(
        position(world, Source::Token(1)),
        tabletop_layout::test_token(true, 0)
    );
    assert_eq!(
        position(world, Source::Token(2)),
        tabletop_layout::test_token(true, 1)
    );
    assert_eq!(
        position(world, Source::Token(3)),
        tabletop_layout::test_token(false, 1)
    );
}

#[test]
fn camera_pans_with_window_focus_but_captures_menu_modal_and_button_input() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = pending_app(dir.path().join("camera.json"));
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<AccumulatedMouseScroll>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .add_systems(
            Update,
            camera_controls.after(handle_actions).before(rebuild_panel),
        );
    let window = {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<Window>>()
            .single(world)
            .unwrap()
    };
    app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(window, bevy::input_focus::FocusCause::Navigated);
    app.world_mut().spawn((MainCamera, Transform::default()));
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(100));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    let start = app.world().resource::<Orbit>().pose().target;
    app.update();
    let panned = app.world().resource::<Orbit>().pose().target;
    assert!(
        panned.z < start.z,
        "initial window focus must permit plain WASD"
    );

    act(&mut app, Action::Select(Source::Token(0), Vec2::ZERO));
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        panned,
        "menu navigation captures WASD"
    );
    act(&mut app, Action::Inspect(2));
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        panned,
        "inspection captures WASD"
    );
    app.world_mut().resource_mut::<Tabletop>().inspect = None;
    let button = app.world_mut().spawn(Button).id();
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(button, bevy::input_focus::FocusCause::Navigated);
    app.update();
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        panned,
        "focused UI captures WASD"
    );

    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(window, bevy::input_focus::FocusCause::Navigated);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyW);
    app.update();
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        panned,
        "pan stops without inertia"
    );
}

fn app_for_session(session: Session, path: PathBuf) -> App {
    app_for_session_in(App::new(), session, path)
}

fn app_for_session_in(mut app: App, session: Session, path: PathBuf) -> App {
    let config = AssetConfig::load_at(path.with_extension("toml"), path.with_extension("assets"));
    app.insert_resource(Tabletop {
        fixture: session.fixture.clone(),
        session,
        replay: None,
        save_path: path,
        notice: String::new(),
        inspect: None,
        history: false,
        result_open: false,
        reduced_motion: false,
        dirty: true,
    })
    .insert_resource(Orbit::new(CameraMode::Jacqueline))
    .insert_resource(RenderHealth::ready_for_test())
    .insert_resource(Actions::default())
    .insert_resource(AssetSettings::new(config))
    .init_resource::<Assets<Mesh>>()
    .init_resource::<Assets<StandardMaterial>>()
    .init_resource::<Time>()
    .insert_resource(Art {
        handles: [None, None, None],
        available: 0,
        token_handles: vec![],
        reminder: None,
        investigators: Vec::new(),
        symbol_font: None,
        status: "Readable substitutes are active.".into(),
    })
    .add_systems(
        Update,
        (
            mark_window_resized,
            poll_asset_settings,
            handle_actions,
            rebuild_panel,
            button_colors,
        )
            .chain(),
    );
    app.world_mut().spawn(Window::default());
    install(&mut app);
    app
}

fn act(app: &mut App, action: Action) {
    app.world_mut().resource_mut::<Actions>().0.push(action);
    app.update();
}

#[test]
fn rendering_health_preserves_jacqueline_records_and_drains_blocked_input() {
    for health in [
        RenderHealth::default(),
        RenderHealth::failed_for_test(),
        RenderHealth::just_ready_for_test(),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jacqueline.json");
        let initial = Session::new(Fixture::curse());
        continuity::save_session(&path, &initial).unwrap();
        let saved = std::fs::read(&path).unwrap();
        let mut app = app_for_session(initial, path.clone());
        act(&mut app, Action::Choose(0, "commit-promise".into()));
        let before = app.world().resource::<Tabletop>().session.clone();
        std::fs::write(&path, &saved).unwrap();
        let file_count = std::fs::read_dir(dir.path()).unwrap().count();
        app.insert_resource(health);
        act(&mut app, Action::Choose(1, "use-jacqueline".into()));
        for action in [Action::Save, Action::Reopen, Action::Replay, Action::New] {
            act(&mut app, action);
        }
        let camera_before = app.world().resource::<Orbit>().pose();
        act(&mut app, Action::Focus(Area::Test));
        assert_ne!(app.world().resource::<Orbit>().pose(), camera_before);
        assert_eq!(app.world().resource::<Tabletop>().session, before);
        assert!(app.world().resource::<Tabletop>().replay.is_none());
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), file_count);
        assert!(app.world().resource::<Actions>().0.is_empty());

        app.insert_resource(RenderHealth::ready_for_test());
        app.update();
        assert_eq!(app.world().resource::<Tabletop>().session, before);
        act(&mut app, Action::Choose(1, "use-jacqueline".into()));
        assert_ne!(app.world().resource::<Tabletop>().session, before);
        assert_eq!(
            continuity::load_session(&path).unwrap(),
            app.world().resource::<Tabletop>().session
        );
    }
}

fn result_visible(app: &mut App) -> bool {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<ModalDialog>>()
        .iter(world)
        .next()
        .is_some()
}

fn result_text(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    let dialog = world
        .query_filtered::<Entity, With<ModalDialog>>()
        .single(world)
        .unwrap();
    world
        .query::<&Children>()
        .query(world)
        .iter_descendants(dialog)
        .filter_map(|entity| world.get::<Text>(entity).map(|text| text.0.clone()))
        .collect()
}

#[test]
fn result_dialog_preserves_recorded_explanations_in_live_reopened_and_replay_views() {
    for (name, fixture, choices, expected, absent) in [
        (
            "early", Fixture::curse(),
            ["commit-promise", "use-jacqueline", "cancel-1-2"].as_slice(),
            ["TEST SUCCEEDED", "Resolved: Curse, -2", "Revealed -2.", "0 additional horror", "Jacqueline's horror: 2 of 9 (unchanged)."].as_slice(),
            "Revealed -1, 0, -2.",
        ),
        (
            "late", Fixture::curse(),
            ["commit-promise", "reveal", "use-jacqueline", "cancel-1-2"].as_slice(),
            ["TEST SUCCEEDED", "Resolved: Curse, -2", "Revealed -1, 0, -2.", "0 additional horror", "Jacqueline's horror: 2 of 9 (unchanged)."].as_slice(),
            "Revealed -2.",
        ),
        (
            "autofail", Fixture::auto_fail(),
            ["commit-promise", "use-jacqueline", "cancel-1-2"].as_slice(),
            ["TEST FAILED", "Resolved: Auto-fail", "The uncanceled auto-fail makes the test fail with a final skill value of 0. Numerical modifiers do not change that result.", "3 additional horror", "Jacqueline's horror: 2 -> 5 of 9."].as_slice(),
            "Why another token?",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("live.json");
        let mut app = app_for_session(Session::new(fixture), path.clone());
        for (revision, &id) in choices.iter().enumerate() {
            act(&mut app, Action::Choose(revision as u64, id.into()));
        }
        let live = app.world().resource::<Tabletop>().view();
        let saved = std::fs::read(&path).unwrap();
        act(&mut app, Action::ShowResult);
        let original_text = result_text(&mut app);
        for &line in expected {
            assert!(original_text.iter().any(|text| text == line), "{name}: missing {line:?} in {original_text:?}");
        }
        assert!(!original_text.iter().any(|text| text == absent));
        assert!(!original_text.iter().any(|text| text.contains("[RR ST.3; 60401]")));

        // Open an independent baseline save through the actual Reopen action.
        // Replay then exports/reloads its recording through the production path.
        let legacy_path = dir.path().join("legacy.json");
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/result-explanation/{name}.session.json")),
            &legacy_path,
        ).unwrap();
        let legacy_bytes = std::fs::read(&legacy_path).unwrap();
        act(&mut app, Action::DismissResult);
        app.world_mut().resource_mut::<Tabletop>().save_path = legacy_path.clone();
        act(&mut app, Action::Reopen);
        assert_eq!(app.world().resource::<Tabletop>().view(), live);
        assert_eq!(result_visible(&mut app), name == "autofail");
        act(&mut app, Action::ShowResult);
        assert_eq!(result_text(&mut app), original_text);
        act(&mut app, Action::DismissResult);
        act(&mut app, Action::Replay);
        for _ in choices {
            act(&mut app, Action::Next);
        }
        assert!(app.world().resource::<Tabletop>().replay.is_some());
        assert_eq!(app.world().resource::<Tabletop>().view(), live);
        assert_eq!(result_visible(&mut app), name == "autofail");
        act(&mut app, Action::ShowResult);
        assert_eq!(result_text(&mut app), original_text);
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        assert_eq!(std::fs::read(&legacy_path).unwrap(), legacy_bytes);
    }
}

#[test]
fn curse_continuation_stays_visible_until_the_player_requests_the_result() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("live.json");
    let mut app = pending_app(path.clone());
    advance(&mut app, 1000);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(20));
    act(&mut app, Action::Choose(2, "cancel-1-2".into()));

    assert!(
        !result_visible(&mut app),
        "the additional token must not reveal behind a result popup"
    );
    assert_eq!(appearing_count(&mut app), 1);
    let world = app.world_mut();
    assert!(
        world
            .query::<&Text>()
            .iter(world)
            .any(|text| { text.0 == "Curse: -2. Reveal another token." })
    );
    let saved = continuity::load_session(&path).unwrap();
    assert_eq!(saved.observe().stage, Stage::Finished);
    assert_eq!(saved.observe().skill_value, 5);
    assert_eq!(saved.observe().tokens.len(), 4);
    let bytes = std::fs::read(&path).unwrap();

    advance(&mut app, 10_000);
    assert_eq!(appearing_count(&mut app), 0);
    assert!(!result_visible(&mut app), "there is no popup timer");
    act(&mut app, Action::ShowResult);
    assert!(result_visible(&mut app));
    act(&mut app, Action::DismissResult);
    assert!(!result_visible(&mut app));
    assert_eq!(appearing_count(&mut app), 0);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn replay_and_return_to_live_leave_the_curse_reveal_unobscured() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("live.json");
    let mut app = pending_app(path.clone());
    act(&mut app, Action::Choose(2, "cancel-1-2".into()));
    let saved = std::fs::read(&path).unwrap();
    act(&mut app, Action::Replay);
    act(&mut app, Action::Next);
    act(&mut app, Action::Next);
    advance(&mut app, 1000);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(20));
    act(&mut app, Action::Next);
    assert!(!result_visible(&mut app));
    assert_eq!(appearing_count(&mut app), 1);
    assert_eq!(app.world().resource::<Tabletop>().view().tokens.len(), 4);
    act(&mut app, Action::ShowResult);
    assert!(result_visible(&mut app));
    act(&mut app, Action::DismissResult);
    act(&mut app, Action::Back);
    assert_eq!(app.world().resource::<Tabletop>().view().tokens.len(), 3);
    act(&mut app, Action::Next);
    assert!(!result_visible(&mut app));
    act(&mut app, Action::Back);
    act(&mut app, Action::Live);
    assert!(!result_visible(&mut app));
    assert_eq!(app.world().resource::<Tabletop>().view().tokens.len(), 4);
    assert_eq!(std::fs::read(path).unwrap(), saved);
}

#[test]
fn reopening_a_completed_curse_test_preserves_the_visible_reveal_and_saved_outcome() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("live.json");
    let mut app = pending_app(path.clone());
    act(&mut app, Action::Choose(2, "cancel-1-2".into()));
    let saved = std::fs::read(&path).unwrap();
    act(&mut app, Action::ShowResult);
    assert!(result_visible(&mut app));
    act(&mut app, Action::DismissResult);
    act(&mut app, Action::Reopen);
    assert!(!result_visible(&mut app));
    let view = app.world().resource::<Tabletop>().view();
    assert_eq!(view.stage, Stage::Finished);
    assert_eq!(view.skill_value, 5);
    assert_eq!(view.tokens.len(), 4);
    act(&mut app, Action::ShowResult);
    assert!(result_visible(&mut app));
    assert_eq!(std::fs::read(path).unwrap(), saved);
}

#[test]
fn card_inspection_blocks_underlying_decisions_until_closed() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = pending_app(dir.path().join("live.json"));
    act(&mut app, Action::Inspect(2));
    act(&mut app, Action::Choose(2, "cancel-1-2".into()));
    assert_eq!(
        app.world().resource::<Tabletop>().view().stage,
        Stage::Cancellation
    );
    act(&mut app, Action::CloseInspect);
    act(&mut app, Action::Choose(2, "cancel-1-2".into()));
    assert_eq!(
        app.world().resource::<Tabletop>().view().stage,
        Stage::Finished
    );
}

#[test]
fn showing_and_dismissing_result_do_not_replay_token_reveal_animation() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = pending_app(dir.path().join("live.json"));
    act(&mut app, Action::Choose(2, "cancel-1-2".into()));
    act(&mut app, Action::ShowResult);
    assert_eq!(
        appearing_count(&mut app),
        4,
        "newly revealed tokens animate"
    );
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(20));
    app.update();
    let moving_tokens: Vec<_> = {
        let world = app.world_mut();
        world
            .query_filtered::<(Entity, &Transform), With<Appearing>>()
            .iter(world)
            .map(|(entity, transform)| (entity, transform.scale))
            .collect()
    };
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::ZERO);
    act(&mut app, Action::DismissResult);
    act(&mut app, Action::ShowResult);
    for (entity, scale) in moving_tokens {
        assert_eq!(
            app.world().get::<Transform>(entity).unwrap().scale,
            scale,
            "a modal redraw must preserve an in-flight token and its progress"
        );
    }
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(1));
    app.update();
    assert_eq!(
        appearing_count(&mut app),
        0,
        "tokens have reached full size"
    );
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::ZERO);
    for action in [
        Action::DismissResult,
        Action::ShowResult,
        Action::DismissResult,
    ] {
        act(&mut app, action);
        assert_eq!(
            appearing_count(&mut app),
            0,
            "modal actions must not replay an existing reveal"
        );
    }
}

#[test]
fn canceled_curse_result_opens_and_can_be_dismissed_saved_and_requested_again() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("live.json");
    let mut app = pending_app(path.clone());
    act(&mut app, Action::Choose(2, "cancel-0-1".into()));
    assert!(app.world().resource::<Tabletop>().result_open);
    assert_eq!(
        continuity::load_session(&path)
            .unwrap()
            .observe()
            .skill_value,
        9
    );
    // A queued underlying-table action cannot replace the completed run through the modal.
    act(&mut app, Action::New);
    assert_eq!(
        app.world().resource::<Tabletop>().view().stage,
        Stage::Finished
    );
    act(&mut app, Action::DismissResult);
    act(&mut app, Action::Save);
    assert!(!app.world().resource::<Tabletop>().result_open);
    act(&mut app, Action::ShowResult);
    assert!(app.world().resource::<Tabletop>().result_open);
    act(&mut app, Action::DismissResult);
    act(&mut app, Action::Reopen);
    assert!(app.world().resource::<Tabletop>().result_open);
}

#[test]
fn failed_autosave_does_not_present_an_uncommitted_result() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("live.json");
    std::fs::write(&path, "damaged save").unwrap();
    let mut app = pending_app(path.clone());
    act(&mut app, Action::Choose(2, "cancel-1-2".into()));
    let table = app.world().resource::<Tabletop>();
    assert!(!table.result_open);
    assert_eq!(table.view().stage, Stage::Cancellation);
    assert!(table.notice.contains("not committed"));
    assert_eq!(std::fs::read_to_string(path).unwrap(), "damaged save");
}

#[test]
fn replay_without_a_resolved_curse_presents_results_without_mutating_live_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("live.json");
    let mut app = pending_app(path.clone());
    act(&mut app, Action::Choose(2, "cancel-0-1".into()));
    let saved = std::fs::read(&path).unwrap();
    act(&mut app, Action::DismissResult);
    act(&mut app, Action::Replay);
    assert!(!app.world().resource::<Tabletop>().result_open);
    for _ in 0..3 {
        act(&mut app, Action::Next);
    }
    assert!(app.world().resource::<Tabletop>().result_open);
    act(&mut app, Action::DismissResult);
    act(&mut app, Action::Next); // Already at the last moment: no new presentation.
    assert!(!app.world().resource::<Tabletop>().result_open);
    act(&mut app, Action::Back);
    assert!(!app.world().resource::<Tabletop>().result_open);
    act(&mut app, Action::Next);
    assert!(app.world().resource::<Tabletop>().result_open);
    assert_eq!(std::fs::read(path).unwrap(), saved);
}

#[test]
fn replay_controls_and_reminder_follow_recorded_state_without_resetting_the_flip() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = pending_app(dir.path().join("live.json"));
    act(&mut app, Action::Choose(2, "cancel-1-2".into()));
    act(&mut app, Action::DismissResult);
    let reminder = app
        .world_mut()
        .query_filtered::<Entity, With<ReminderPose>>()
        .single(app.world())
        .unwrap();
    let has_text = |app: &mut App, needle: &str| {
        let world = app.world_mut();
        world
            .query::<&Text>()
            .iter(world)
            .any(|text| text.0.contains(needle))
    };
    act(&mut app, Action::Replay);
    assert!(has_text(&mut app, "Rotting Remains"));
    assert!(has_text(&mut app, "Next moment"));
    assert!(!has_text(&mut app, "Previous moment"));
    assert!(has_text(&mut app, "READY"));
    act(&mut app, Action::Next);
    assert!(has_text(&mut app, "Committed"));
    act(&mut app, Action::Next);
    assert!(has_text(&mut app, "Used this round"));
    assert_eq!(
        app.world().get::<ReminderPose>(reminder).unwrap().target,
        std::f32::consts::PI
    );
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(20));
    app.update();
    let angle = app.world().get::<ReminderPose>(reminder).unwrap().angle;
    assert!(angle > 0.0 && angle < std::f32::consts::PI);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::ZERO);
    act(&mut app, Action::History); // An unrelated rebuild must not restart the physical flip.
    assert_eq!(
        app.world().get::<ReminderPose>(reminder).unwrap().angle,
        angle
    );
    act(&mut app, Action::Next);
    assert!(!has_text(&mut app, "Next moment"));
    assert!(has_text(&mut app, "Previous moment"));
}

fn advance(app: &mut App, millis: u64) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(millis));
    app.update();
}

fn scene_snapshot(app: &mut App) -> Vec<(Entity, Option<Transform>, Option<String>)> {
    let world = app.world_mut();
    let mut entities: Vec<_> = world
        .query_filtered::<(Entity, Option<&Transform>, Option<&Text>), With<TransientScene>>()
        .iter(world)
        .map(|(entity, transform, text)| {
            (entity, transform.copied(), text.map(|text| text.0.clone()))
        })
        .collect();
    entities.sort_by_key(|(entity, _, _)| *entity);
    entities
}

fn reminder_state(app: &mut App) -> (Entity, Transform, f32) {
    let world = app.world_mut();
    let (entity, transform, pose) = world
        .query::<(Entity, &Transform, &ReminderPose)>()
        .single(world)
        .unwrap();
    (entity, *transform, pose.angle)
}

#[test]
fn panel_refreshes_preserve_scene_entities_and_in_flight_motion() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = pending_app(dir.path().join("live.json"));
    app.update();
    advance(&mut app, 20);
    let before = scene_snapshot(&mut app);
    let reminder = reminder_state(&mut app);
    assert!(reminder.2 > 0.0 && reminder.2 < std::f32::consts::PI);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::ZERO);
    for action in [
        Action::Select(Source::Token(0), Vec2::ZERO),
        Action::DismissContext,
        Action::Inspect(2),
        Action::CloseInspect,
        Action::History,
        Action::History,
    ] {
        act(&mut app, action);
        assert_eq!(scene_snapshot(&mut app), before);
        assert_eq!(reminder_state(&mut app), reminder);
    }
    app.world_mut()
        .query::<&mut Window>()
        .single_mut(app.world_mut())
        .unwrap()
        .resolution
        .set(1100.0, 700.0);
    app.update();
    assert_eq!(scene_snapshot(&mut app), before);
    assert_eq!(reminder_state(&mut app), reminder);

    // A real failed selection completes through the settings poll, without opening a dialog.
    let config_path = dir.path().join("invalid.toml");
    std::fs::write(&config_path, "invalid = [").unwrap();
    app.insert_resource(AssetSettings::new(AssetConfig::load_at(
        config_path,
        dir.path().join("assets"),
    )));
    act(&mut app, Action::AssetFolder);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while app.world().resource::<AssetSettings>().is_busy() {
        assert!(
            std::time::Instant::now() < deadline,
            "settings worker did not finish"
        );
        std::thread::yield_now();
        app.update();
    }
    assert!(
        app.world()
            .resource::<Tabletop>()
            .notice
            .contains("Asset folder was not saved")
    );
    assert_eq!(scene_snapshot(&mut app), before);
    assert_eq!(reminder_state(&mut app), reminder);
    advance(&mut app, 20);
    let moving = reminder_state(&mut app);
    assert_eq!(moving.0, reminder.0);
    assert!(moving.2 > reminder.2);

    act(&mut app, Action::Replay);
    act(&mut app, Action::Next);
    act(&mut app, Action::Next);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::ZERO);
    let before_follow = scene_snapshot(&mut app);
    let reminder = reminder_state(&mut app);
    assert!(app.world().resource::<Orbit>().is_following());
    act(&mut app, Action::Follow);
    assert!(!app.world().resource::<Orbit>().is_following());
    assert_eq!(scene_snapshot(&mut app), before_follow);
    assert_eq!(reminder_state(&mut app), reminder);
}

#[test]
fn three_displayed_tokens_continue_to_four_with_only_the_new_token_revealing() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = pending_app(dir.path().join("live.json"));
    app.update();
    advance(&mut app, 1000);
    let previous = scene_snapshot(&mut app);
    let reminder = reminder_state(&mut app).0;
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(20));
    act(&mut app, Action::Choose(2, "cancel-1-2".into()));
    for (entity, _, _) in previous {
        assert!(
            app.world().get_entity(entity).is_err(),
            "old scene pieces and labels must be removed"
        );
    }
    assert_eq!(
        reminder_state(&mut app).0,
        reminder,
        "the physical reminder persists"
    );
    let world = app.world_mut();
    let appearing: Vec<_> = world
        .query_filtered::<&Transform, With<Appearing>>()
        .iter(world)
        .copied()
        .collect();
    assert_eq!(appearing.len(), 1);
    assert!(
        appearing[0].scale.x > 0.5 && appearing[0].scale.x < 1.0,
        "new entities animate in the synchronization frame"
    );
    assert_eq!(
        appearing[0].translation,
        tabletop_layout::test_token(false, 1)
    );
    let mut pieces: Vec<_> = world
        .query_filtered::<(
            &context::Selectable,
            &Transform,
            &MeshMaterial3d<StandardMaterial>,
            Option<&Children>,
        ), With<TransientScene>>()
        .iter(world)
        .filter_map(|(source, transform, material, children)| {
            let Source::Token(id) = source.0 else {
                return None;
            };
            Some((
                id,
                *transform,
                material.0.clone(),
                children.map_or(0, |children| {
                    children
                        .iter()
                        .filter(|child| world.get::<Mesh3d>(*child).is_some())
                        .count()
                }),
            ))
        })
        .collect();
    pieces.sort_by_key(|piece| piece.0);
    assert_eq!(pieces.len(), 4);
    for (id, transform, material, children) in &pieces {
        let canceled = [1, 2].contains(id);
        let slot = match id {
            0 | 1 => 0,
            2 | 3 => 1,
            _ => unreachable!(),
        };
        assert_eq!(
            transform.translation,
            tabletop_layout::test_token(canceled, slot)
        );
        if *id != 3 {
            assert_eq!(transform.scale, Vec3::ONE);
        }
        let expected_color = if canceled {
            Color::srgb(0.37, 0.25, 0.28)
        } else {
            Color::srgb(0.28, 0.62, 0.51)
        };
        assert_eq!(
            world
                .resource::<Assets<StandardMaterial>>()
                .get(material)
                .unwrap()
                .base_color,
            expected_color
        );
        assert_eq!(
            *children,
            if canceled { 2 } else { 0 },
            "canceled tokens retain their crosses"
        );
    }
    let labels: Vec<_> = world
        .query_filtered::<&Text, Or<(With<WorldLabel>, With<tabletop_tokens::TokenLabel>)>>()
        .iter(world)
        .map(|text| text.0.as_str())
        .collect();
    assert!(labels.contains(&"#1  Curse\nResolved"));
    assert!(
        labels
            .iter()
            .any(|text| text.starts_with("#4") && text.ends_with("Resolved"))
    );
    assert_eq!(
        labels
            .iter()
            .filter(|text| text.starts_with('#') && text.ends_with("Canceled"))
            .count(),
        2
    );
}

fn appearing_count(app: &mut App) -> usize {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<Appearing>>()
        .iter(world)
        .count()
}

#[test]
fn replay_back_next_and_live_compare_complete_observations_and_only_previous_tokens() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("live.json");
    let mut app = pending_app(path.clone());
    app.update();
    act(&mut app, Action::Choose(2, "cancel-1-2".into()));
    let saved = std::fs::read(&path).unwrap();
    act(&mut app, Action::DismissResult);
    act(&mut app, Action::Replay);
    act(&mut app, Action::Next);
    act(&mut app, Action::Next);
    assert_eq!(appearing_count(&mut app), 3);
    act(&mut app, Action::Next);
    assert_eq!(appearing_count(&mut app), 1);
    act(&mut app, Action::DismissResult);
    act(&mut app, Action::Back);
    assert_eq!(app.world().resource::<Tabletop>().view().tokens.len(), 3);
    assert_eq!(
        appearing_count(&mut app),
        0,
        "tokens held in the previous moment do not reveal again"
    );
    act(&mut app, Action::Back);
    assert!(app.world().resource::<Tabletop>().view().tokens.is_empty());
    act(&mut app, Action::Next);
    assert_eq!(
        appearing_count(&mut app),
        3,
        "returning from a token-free moment reveals again; there is no seen-token cache"
    );
    act(&mut app, Action::Live);
    assert_eq!(
        app.world().resource::<Tabletop>().view().stage,
        Stage::Finished
    );
    assert_eq!(appearing_count(&mut app), 1);
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    act(&mut app, Action::DismissResult);
    act(&mut app, Action::Replay);
    act(&mut app, Action::Next);
    act(&mut app, Action::Next);

    // Privileged setup: a distinct live session at the same revision as the replay.
    let mut other = Session::new(Fixture::auto_fail());
    other.choose(0, "commit-promise").unwrap();
    other.choose(1, "use-jacqueline").unwrap();
    let expected = other.observe();
    assert_eq!(
        expected.revision,
        app.world().resource::<Tabletop>().view().revision
    );
    assert_ne!(expected, app.world().resource::<Tabletop>().view());
    let previous = scene_snapshot(&mut app);
    app.world_mut().resource_mut::<Tabletop>().session = other;
    act(&mut app, Action::Live);
    assert_eq!(app.world().resource::<Tabletop>().view(), expected);
    assert_eq!(
        appearing_count(&mut app),
        2,
        "same IDs with different faces are new; unchanged ID/face is retained"
    );
    for (entity, _, _) in previous {
        assert!(app.world().get_entity(entity).is_err());
    }

    // Scene synchronization also runs when the panel has already consumed its refresh.
    let mut other = Session::new(Fixture::curse());
    other.choose(0, "commit-promise").unwrap();
    other.choose(1, "use-jacqueline").unwrap();
    assert!(!app.world().resource::<Tabletop>().dirty);
    app.world_mut().resource_mut::<Tabletop>().session = other;
    app.update();
    assert_eq!(appearing_count(&mut app), 2);
    let world = app.world_mut();
    assert!(
        world
            .query_filtered::<&Text, Or<(With<WorldLabel>, With<tabletop_tokens::TokenLabel>)>>()
            .iter(world)
            .any(|text| text.0 == "#1  Curse\nRevealed")
    );
}

#[test]
fn world_labels_are_projected_on_spawn_and_follow_the_camera_without_scene_replacement() {
    use bevy::camera::{CameraProjection, ComputedCameraValues, RenderTargetInfo};
    let dir = tempfile::tempdir().unwrap();
    let mut app = pending_app(dir.path().join("live.json"));
    // This projection-only harness supplies its camera below and has no text/UI
    // layout plugins. Supply the measured rectangle as another projection input;
    // production obtains this size from Bevy's UI layout.
    app.add_systems(
        Update,
        (|mut headings: Query<&mut ComputedNode, With<tabletop_layout::TestAreaHeading>>| {
            for mut heading in &mut headings {
                heading.size = Vec2::new(148.0, 20.0);
                heading.inverse_scale_factor = 1.0;
            }
        })
        .after(synchronize)
        .before(label_positions),
    );
    app.world_mut()
        .resource_mut::<Orbit>()
        .focus(tabletop_layout::focus(Area::Scenario));
    let camera = app
        .world_mut()
        .spawn((
            Camera {
                computed: ComputedCameraValues {
                    clip_from_view: OrthographicProjection {
                        near: 0.1,
                        far: 100.0,
                        area: Rect::new(-8.0, -6.0, 8.0, 6.0),
                        ..OrthographicProjection::default_3d()
                    }
                    .get_clip_from_view(),
                    target_info: Some(RenderTargetInfo {
                        physical_size: UVec2::new(1600, 1200),
                        scale_factor: 1.0,
                    }),
                    ..default()
                },
                ..default()
            },
            GlobalTransform::from(
                Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::NEG_Z),
            ),
            MainCamera,
            Camera3d::default(),
        ))
        .id();
    app.update();
    let world = app.world_mut();
    let labels: Vec<_> = world
        .query_filtered::<(Entity, &Node, &Visibility, Option<&LabelDisplay>), Or<(With<WorldLabel>, With<tabletop_tokens::TokenLabel>)>>()
        .iter(world)
        .map(|(entity, node, visibility, display)| {
            assert_eq!(
                *visibility,
                if display == Some(&LabelDisplay::Overview) { Visibility::Hidden } else { Visibility::Visible },
                "new labels must reach projection in their spawn frame"
            );
            let Val::Px(left) = node.left else {
                panic!("label was not projected")
            };
            (entity, left)
        })
        .collect();
    assert!(
        labels.len() >= 11,
        "the expanded table retains the test labels"
    );
    let reminder = reminder_state(&mut app).0;
    // Pan one world unit right: this orthographic camera maps that to 100 pixels left.
    app.world_mut()
        .entity_mut(camera)
        .insert(GlobalTransform::from(
            Transform::from_xyz(1.0, 10.0, 0.0).looking_at(Vec3::X, Vec3::NEG_Z),
        ));
    app.update();
    for (entity, left) in &labels {
        let Val::Px(new_left) = app.world().get::<Node>(*entity).unwrap().left else {
            panic!("label was not projected")
        };
        assert!((new_left - (left - 100.0)).abs() < 0.01);
    }
    act(&mut app, Action::Choose(2, "cancel-1-2".into()));
    for (entity, _) in labels {
        assert!(app.world().get_entity(entity).is_err());
    }
    assert_eq!(reminder_state(&mut app).0, reminder);
    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<Entity, With<tabletop_tokens::TokenLabel>>()
            .iter(world)
            .count(),
        4,
        "the curse continuation adds a newly projected token label"
    );
    let labels: Vec<_> = world
        .query_filtered::<(&Node, &Visibility, Option<&LabelDisplay>), Or<(With<WorldLabel>, With<tabletop_tokens::TokenLabel>)>>()
        .iter(world)
        .collect();
    assert!(labels.iter().all(
        |(node, visibility, display)| matches!(node.left, Val::Px(_))
            && **visibility
                == if *display == Some(&LabelDisplay::Overview) {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                }
    ));

    let original_entities: Vec<_> = world
        .query_filtered::<Entity, With<WorldLabel>>()
        .iter(world)
        .collect();
    world
        .resource_mut::<Orbit>()
        .focus(tabletop_layout::focus(Area::Overview));
    app.update();
    let world = app.world_mut();
    let mut visible_labels = Vec::new();
    for (entity, display, visibility, text) in world
        .query::<(Entity, &LabelDisplay, &Visibility, &Text)>()
        .iter(world)
    {
        assert!(
            original_entities.contains(&entity),
            "zoom must preserve the observation scene"
        );
        assert_eq!(
            *visibility,
            if *display == LabelDisplay::Detail {
                Visibility::Hidden
            } else {
                Visibility::Visible
            }
        );
        if *visibility == Visibility::Visible {
            visible_labels.push(text.0.as_str());
        }
    }
    assert_eq!(
        visible_labels.len(),
        8,
        "overview shows five area names and three absent-art card names"
    );
    for name in [
        "Tony Morgan",
        "Patrice Hathaway",
        "Jacqueline Fine",
        "Scenario / Study",
        "Shared test area",
        "Promise of Power",
        "Rotting Remains",
    ] {
        assert!(
            visible_labels.contains(&name),
            "missing overview identity: {name}"
        );
    }
    world
        .resource_mut::<Orbit>()
        .focus(tabletop_layout::focus(Area::Investigator(0)));
    app.update();
    for entity in original_entities {
        let display = app.world().get::<LabelDisplay>(entity).unwrap();
        assert_eq!(
            *app.world().get::<Visibility>(entity).unwrap(),
            if *display == LabelDisplay::Overview {
                Visibility::Hidden
            } else {
                Visibility::Visible
            }
        );
    }
}

#[test]
fn world_labels_stay_below_panels_when_observations_change_with_history_open() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = pending_app(dir.path().join("live.json"));
    // Use Bevy's actual UI stack without a display or GPU renderer.
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::input::InputPlugin,
        bevy::input_focus::InputFocusPlugin,
        WindowPlugin {
            primary_window: None,
            ..default()
        },
        bevy::a11y::AccessibilityPlugin,
        bevy::image::ImagePlugin::default(),
        bevy::mesh::MeshPlugin,
        bevy::camera::CameraPlugin,
        bevy::transform::TransformPlugin,
        bevy::sprite::SpritePlugin,
        bevy::text::TextPlugin,
        bevy::ui::UiPlugin,
        bevy::picking::DefaultPickingPlugins,
    ));
    app.finish();
    app.cleanup();
    app.update();
    let assert_layers = |app: &mut App| {
        let world = app.world_mut();
        let labels: Vec<_> = world
            .query_filtered::<Entity, Or<(With<WorldLabel>, With<tabletop_tokens::TokenLabel>)>>()
            .iter(world)
            .collect();
        let panels: Vec<_> = world
            .query_filtered::<Entity, With<Panel>>()
            .iter(world)
            .collect();
        assert!(!labels.is_empty() && !panels.is_empty());
        let stack = &world.resource::<bevy::ui::UiStack>().uinodes;
        let top_label = labels
            .iter()
            .map(|entity| stack.iter().position(|node| node == entity).unwrap())
            .max()
            .unwrap();
        for panel in panels {
            assert!(
                stack.iter().position(|node| *node == panel).unwrap() > top_label,
                "world labels must stay beneath panel content"
            );
        }
    };
    assert_layers(&mut app);
    act(&mut app, Action::History);
    assert_layers(&mut app);
    act(&mut app, Action::Choose(2, "cancel-1-2".into()));
    assert_layers(&mut app);
    act(&mut app, Action::DismissResult);
    assert_layers(&mut app);
}

fn orbit_mouse_app() -> (App, Entity) {
    let directory = tempfile::tempdir().unwrap();
    let mut app = pending_app(directory.path().join("orbit.json"));
    app.add_plugins(bevy::input::InputPlugin)
        .add_message::<bevy::window::CursorMoved>()
        .init_resource::<bevy::picking::hover::HoverMap>()
        .add_systems(
            Update,
            camera_controls.after(handle_actions).before(rebuild_panel),
        );
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    {
        let mut window = app.world_mut().get_mut::<Window>(window).unwrap();
        window.resolution = (1600, 1000).into();
        window.focused = true;
        window.set_cursor_position(Some(Vec2::new(800.0, 400.0)));
    }
    app.world_mut().spawn((MainCamera, Transform::default()));
    app.update();
    (app, window)
}

fn contextual_input_app(session: Session, path: PathBuf) -> (App, Entity, Entity) {
    use bevy::{
        camera::{ComputedCameraValues, RenderTarget, RenderTargetInfo},
        input_focus::{InputDispatchPlugin, InputFocusPlugin},
        picking::pointer::{Location, PointerId, PointerLocation},
        window::{PrimaryWindow, WindowRef},
    };

    // Exercise the rendered menu through Bevy layout, UI picking and keyboard
    // dispatch, without a desktop window or a replacement interaction model.
    let mut app = App::new();
    app.add_plugins((
        bevy::app::TaskPoolPlugin::default(),
        bevy::asset::AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        bevy::text::TextPlugin,
        bevy::transform::TransformPlugin,
        bevy::camera::visibility::VisibilityPlugin,
        bevy::input::InputPlugin,
        InputFocusPlugin,
        InputDispatchPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        bevy::ui::UiPlugin,
        bevy::ui_widgets::ButtonPlugin,
    ))
    .init_resource::<Assets<TextureAtlasLayout>>()
    .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
    .init_resource::<bevy::prelude::Time<Real>>()
    .add_message::<bevy::window::CursorMoved>();
    let mut app = app_for_session_in(app, session, path);
    app.add_systems(
        Update,
        (
            context::keyboard.before(handle_actions),
            camera_controls.after(handle_actions).before(rebuild_panel),
        ),
    );
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(window).insert(PrimaryWindow);
    {
        let mut window = app.world_mut().get_mut::<Window>(window).unwrap();
        window.resolution = (1600, 1000).into();
        window.focused = true;
        window.set_cursor_position(Some(Vec2::new(800.0, 400.0)));
    }
    let target = RenderTarget::Window(WindowRef::Entity(window));
    app.world_mut().spawn((
        MainCamera,
        Transform::default(),
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(1600, 1000),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
        target.clone(),
    ));
    let pointer = app
        .world_mut()
        .spawn((
            PointerId::Mouse,
            PointerLocation::new(Location {
                target: target.normalize(Some(window)).unwrap(),
                position: Vec2::new(800.0, 400.0),
            }),
        ))
        .id();
    app.update();
    (app, window, pointer)
}

fn contextual_cursor(app: &mut App, window: Entity, pointer: Entity, position: Vec2) {
    app.world_mut()
        .get_mut::<bevy::picking::pointer::PointerLocation>(pointer)
        .unwrap()
        .location
        .as_mut()
        .unwrap()
        .position = position;
    orbit_cursor(app, window, position);
}

#[test]
fn contextual_menu_backdrop_allows_orbit_and_still_dismisses_on_left_click() {
    use bevy::picking::pointer::{PointerAction, PointerId, PointerInput, PointerLocation};

    let dir = tempfile::tempdir().unwrap();
    let (mut app, window, pointer) =
        contextual_input_app(Session::new(Fixture::curse()), dir.path().join("menu.json"));
    let before = app.world().resource::<Tabletop>().view();
    act(
        &mut app,
        Action::Select(Source::Card(1), Vec2::new(200.0, 200.0)),
    );
    contextual_cursor(&mut app, window, pointer, Vec2::new(800.0, 400.0));
    let yaw = app.world().resource::<Orbit>().pose().yaw;
    orbit_button(&mut app, window, bevy::input::ButtonState::Pressed);
    contextual_cursor(&mut app, window, pointer, Vec2::new(840.0, 410.0));
    assert_ne!(app.world().resource::<Orbit>().pose().yaw, yaw);
    assert!(!app.world().resource::<Context>().menu_open());
    assert_eq!(app.world().resource::<Tabletop>().view(), before);
    orbit_button(&mut app, window, bevy::input::ButtonState::Released);

    act(
        &mut app,
        Action::Select(Source::Card(1), Vec2::new(200.0, 200.0)),
    );
    contextual_cursor(&mut app, window, pointer, Vec2::new(800.0, 400.0));
    for action in [
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        let location = app
            .world()
            .get::<PointerLocation>(pointer)
            .unwrap()
            .location
            .clone()
            .unwrap();
        app.world_mut()
            .write_message(PointerInput::new(PointerId::Mouse, location, action));
        app.update();
    }
    assert!(!app.world().resource::<Context>().menu_open());
    assert_eq!(app.world().resource::<Tabletop>().view(), before);
}

fn contextual_key(app: &mut App, window: Entity, key_code: KeyCode) {
    contextual_key_chord(app, window, key_code, false);
}

fn contextual_key_chord(app: &mut App, window: Entity, key_code: KeyCode, shift: bool) {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    let logical_key = match key_code {
        KeyCode::Tab => Key::Tab,
        KeyCode::Enter => Key::Enter,
        KeyCode::Space => Key::Space,
        KeyCode::Escape => Key::Escape,
        KeyCode::KeyW => Key::Character("w".into()),
        _ => unreachable!(),
    };
    let mut events = Vec::new();
    if shift {
        events.push((KeyCode::ShiftRight, Key::Shift, ButtonState::Pressed));
    }
    events.extend([
        (key_code, logical_key.clone(), ButtonState::Pressed),
        (key_code, logical_key, ButtonState::Released),
    ]);
    if shift {
        events.push((KeyCode::ShiftRight, Key::Shift, ButtonState::Released));
    }
    for (key_code, logical_key, state) in events {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key,
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
}

#[test]
fn contextual_menu_tab_navigation_inspects_then_commits_without_moving_camera() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, window, _) =
        contextual_input_app(Session::new(Fixture::curse()), dir.path().join("keys.json"));
    let before = app.world().resource::<Tabletop>().view();
    act(
        &mut app,
        Action::Select(Source::Card(1), Vec2::new(200.0, 200.0)),
    );
    let target = app.world().resource::<Orbit>().pose().target;
    let yaw = app.world().resource::<Orbit>().pose().yaw;
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(100));
    let initial_focus = app
        .world()
        .resource::<bevy::input_focus::InputFocus>()
        .get();
    contextual_key(&mut app, window, KeyCode::Tab); // Inspect.
    assert_ne!(
        app.world()
            .resource::<bevy::input_focus::InputFocus>()
            .get(),
        initial_focus
    );
    contextual_key_chord(&mut app, window, KeyCode::Tab, true);
    assert_eq!(
        app.world()
            .resource::<bevy::input_focus::InputFocus>()
            .get(),
        initial_focus
    );
    contextual_key(&mut app, window, KeyCode::Tab); // Inspect again.
    contextual_key(&mut app, window, KeyCode::KeyW);
    assert_eq!(app.world().resource::<Orbit>().pose().target, target);
    assert_eq!(app.world().resource::<Orbit>().pose().yaw, yaw);
    contextual_key(&mut app, window, KeyCode::Enter);
    assert_eq!(app.world().resource::<Tabletop>().inspect, Some(1));
    assert_eq!(app.world().resource::<Tabletop>().view(), before);
    assert_eq!(
        app.world().resource::<Context>().selected_source(),
        Some(Source::Card(1))
    );

    contextual_key(&mut app, window, KeyCode::Escape);
    assert!(app.world().resource::<Tabletop>().inspect.is_none());
    assert_eq!(app.world().resource::<Tabletop>().view(), before);
    act(
        &mut app,
        Action::Select(Source::Card(1), Vec2::new(200.0, 200.0)),
    );
    // The first action acquires focus when its contextual menu opens.
    contextual_key(&mut app, window, KeyCode::Space);
    let after = app.world().resource::<Tabletop>().view();
    assert_eq!(after.stage, Stage::BeforeReveal);
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(app.world().resource::<Orbit>().pose().target, target);
}

fn orbit_button(app: &mut App, window: Entity, state: bevy::input::ButtonState) {
    app.world_mut()
        .write_message(bevy::input::mouse::MouseButtonInput {
            button: MouseButton::Right,
            state,
            window,
        });
    app.update();
}

fn orbit_cursor(app: &mut App, window: Entity, position: Vec2) {
    let delta = {
        let mut window = app.world_mut().get_mut::<Window>(window).unwrap();
        let delta = window.cursor_position().map(|previous| position - previous);
        window.set_cursor_position(Some(position));
        delta
    };
    // This is the window-event path used by Winit, independently of raw motion.
    app.world_mut().write_message(bevy::window::CursorMoved {
        window,
        position,
        delta,
    });
    app.update();
}

#[test]
fn right_drag_uses_logical_cursor_distance_once_when_raw_motion_also_arrives() {
    let (mut app, window) = orbit_mouse_app();
    let before = app.world().resource::<Orbit>().pose().yaw;
    orbit_button(&mut app, window, bevy::input::ButtonState::Pressed);
    // A high-DPI device can report a larger physical delta for the same cursor move.
    app.world_mut()
        .write_message(bevy::input::mouse::MouseMotion {
            delta: Vec2::new(80.0, 20.0),
        });
    orbit_cursor(&mut app, window, Vec2::new(840.0, 410.0));
    assert!((app.world().resource::<Orbit>().pose().yaw - (before - 40.0 * 0.006)).abs() < 0.00001);
}

#[test]
fn camera_adapter_blocks_jacqueline_utilities_wheel_results_and_escape_frame_keys() {
    use bevy::ecs::system::RunSystemOnce;

    let (mut app, _) = orbit_mouse_app();
    let before = app.world().resource::<Orbit>().pose();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(100));
    app.world_mut()
        .resource_mut::<AccumulatedMouseScroll>()
        .delta
        .y = 1.0;
    app.world_mut().resource_mut::<Context>().toggle_utilities();
    app.world_mut().run_system_once(camera_controls).unwrap();
    assert_eq!(app.world().resource::<Orbit>().pose(), before);
    app.world_mut().resource_mut::<Context>().toggle_utilities();
    app.world_mut().resource_mut::<Tabletop>().result_open = true;
    app.world_mut().run_system_once(camera_controls).unwrap();
    assert_eq!(app.world().resource::<Orbit>().pose(), before);
    app.world_mut().resource_mut::<Tabletop>().result_open = false;
    app.world_mut()
        .resource_mut::<AccumulatedMouseScroll>()
        .delta
        .y = 0.0;
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::Escape);
        keys.press(KeyCode::KeyW);
    }
    app.world_mut().run_system_once(camera_controls).unwrap();
    assert_eq!(app.world().resource::<Orbit>().pose(), before);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut().run_system_once(camera_controls).unwrap();
    assert!(app.world().resource::<Orbit>().pose().target.z < before.target.z);
}

#[test]
fn camera_adapter_keeps_jacqueline_replay_framing_and_freezes_on_follow_off() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = orbit_mouse_app();
    app.world_mut().resource_mut::<Tabletop>().save_path = dir.path().join("replay.json");
    act(&mut app, Action::Focus(Area::Overview));
    let initial = app.world().resource::<Orbit>().pose();
    act(&mut app, Action::Replay);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(100));
    app.update();
    let moving = app.world().resource::<Orbit>().pose();
    let (target, distance) = tabletop_layout::focus(Area::Test);
    assert!(
        (moving.distance - (initial.distance + (distance - initial.distance) * 0.4)).abs()
            < 0.00001
    );
    assert!(moving.target.distance(initial.target.lerp(target, 0.4)) < 0.00001);
    act(&mut app, Action::Follow);
    assert!(!app.world().resource::<Orbit>().is_following());
    app.update();
    let stopped = app.world().resource::<Orbit>().pose();
    assert_eq!(stopped.distance, moving.distance);
    assert!(stopped.target.distance(moving.target) < 0.00001);
}
