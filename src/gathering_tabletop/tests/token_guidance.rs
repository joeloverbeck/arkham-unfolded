use super::*;

#[test]
fn olive_scope_label_geometry_fits_at_selected_scroll_offsets() {
    use crate::tabletop_layout::heading_checks::{rendered_text_bounds, viewport};
    let (_, session) = super::review_saves::olive_review::checkpoints().remove(1);
    let view = session.observe();
    let labels: Vec<_> = view
        .choices
        .iter()
        .filter_map(|choice| {
            view.play
                .as_ref()
                .unwrap()
                .tokens
                .as_ref()
                .unwrap()
                .ability_label(choice)
        })
        .collect();
    assert_eq!(labels.len(), 5);
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("labels.json")), true);
    let disclosure = choice_button(&mut app, "▸ Use Jacqueline · show").unwrap();
    app.world_mut().trigger(Activate { entity: disclosure });
    app.update();
    for size in [Vec2::new(1280., 720.), Vec2::new(1600., 1000.)] {
        for scale in [1., 2.] {
            viewport::<MainCamera>(&mut app, size, scale);
            for _ in 0..4 {
                app.update();
            }
            // Geometry only; the window-event test below owns input coverage.
            for label in &labels {
                let button = choice_button(&mut app, label).unwrap();
                let world = app.world_mut();
                let text = world.get::<Children>(button).unwrap()[0];
                let mut scroller = button;
                // ScrollPosition is a required Node component, including on
                // buttons. The marker identifies the production scroll owner.
                while world.get::<ScrollablePanel>(scroller).is_none() {
                    scroller = world.get::<ChildOf>(scroller).unwrap().parent();
                }
                let node = world.get::<ComputedNode>(scroller).unwrap();
                let maximum = (node.content_size().y - node.size().y).max(0.) / scale;
                let top = world
                    .get::<UiGlobalTransform>(scroller)
                    .unwrap()
                    .transform_point2(-node.size() / 2.)
                    .y
                    / scale;
                let text_top = rendered_text_bounds(world, text, scale).min.y;
                let current = world.get::<ScrollPosition>(scroller).unwrap().y;
                world.get_mut::<ScrollPosition>(scroller).unwrap().y =
                    (current + text_top - top - 2.).clamp(0., maximum);
                app.update();
                let world = app.world_mut();
                let node = world.get::<ComputedNode>(scroller).unwrap();
                let at = world.get::<UiGlobalTransform>(scroller).unwrap();
                let region = Rect::from_corners(
                    at.transform_point2(-node.size() / 2.) / scale,
                    at.transform_point2(node.size() / 2.) / scale,
                );
                let glyphs = rendered_text_bounds(world, text, scale);
                assert!(
                    glyphs.min.y >= region.min.y - 1. && glyphs.max.y <= region.max.y + 1.,
                    "{size:?}/{scale}: {label}: {glyphs:?} outside {region:?}; scroll maximum {maximum}"
                );
            }
            assert_eq!(app.world().resource::<Table>().view(), view);
        }
    }
}

#[test]
fn native_window_wheel_scrolls_olive_options_without_changing_game_or_camera() {
    use bevy::{
        input::mouse::{MouseScrollUnit, MouseWheel},
        picking::{hover::HoverMap, pointer::PointerId},
        window::{CursorMoved, WindowEvent},
    };
    for scale in [1., 2.] {
        let (_, session) = super::review_saves::olive_review::checkpoints().remove(1);
        let view = session.observe();
        let dir = tempfile::tempdir().unwrap();
        let mut app = host_with_table(Table::new(session, dir.path().join("wheel.json")), true);
        let disclosure = choice_button(&mut app, "▸ Use Jacqueline · show").unwrap();
        app.world_mut().trigger(Activate { entity: disclosure });
        crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
            &mut app,
            Vec2::new(1280., 720.),
            scale,
        );
        for _ in 0..5 {
            app.update();
        }
        let world = app.world_mut();
        let window = world
            .query_filtered::<Entity, With<Window>>()
            .single(world)
            .unwrap();
        let scroller = world
            .query_filtered::<Entity, With<ScrollablePanel>>()
            .iter(world)
            .find(|entity| {
                world
                    .get::<ComputedNode>(*entity)
                    .is_some_and(|node| node.content_size().y > node.size().y + 1.)
            })
            .expect("overflowing options");
        let position = world
            .get::<UiGlobalTransform>(scroller)
            .unwrap()
            .translation
            / scale;
        let camera = world
            .query_filtered::<&Transform, With<MainCamera>>()
            .single(world)
            .unwrap()
            .to_owned();
        world.write_message(WindowEvent::CursorMoved(CursorMoved {
            window,
            position,
            delta: None,
        }));
        app.update();
        let hits = &app.world().resource::<HoverMap>()[&PointerId::Mouse];
        assert!(
            hits.keys().any(|entity| {
                let mut at = *entity;
                loop {
                    if at == scroller {
                        return true;
                    }
                    let Some(parent) = app.world().get::<ChildOf>(at) else {
                        return false;
                    };
                    at = parent.parent();
                }
            }),
            "wheel must target the overflowing options panel: {hits:?}"
        );
        let before = app.world().get::<ScrollPosition>(scroller).unwrap().y;
        for (y, down) in [(-4., true), (4., false)] {
            app.world_mut()
                .write_message(WindowEvent::MouseWheel(MouseWheel {
                    unit: MouseScrollUnit::Line,
                    x: 0.,
                    y,
                    window,
                    phase: bevy::input::touch::TouchPhase::Moved,
                }));
            app.update();
            let scroll = app.world().get::<ScrollPosition>(scroller).unwrap().y;
            if down {
                assert!(
                    scroll > before,
                    "down must move options: {scroll} <= {before}"
                );
            } else {
                assert!((scroll - before).abs() < 0.01, "up must restore offset");
            }
            assert_eq!(app.world().resource::<Table>().view(), view);
            let world = app.world_mut();
            assert_eq!(
                *world
                    .query_filtered::<&Transform, With<MainCamera>>()
                    .single(world)
                    .unwrap(),
                camera
            );
        }
    }
}

#[test]
fn olive_tokens_return_from_their_displayed_positions_after_the_result() {
    let (_, session) = super::review_saves::olive_review::checkpoints()
        .into_iter()
        .find(|(name, _)| *name == "olive-two-kept")
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("return.json")), true);
    activate_choice(&mut app, "tokens:confirm");
    finish_presentation(&mut app);
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .result()
            .is_some()
    );
    let mut displayed = Vec::new();
    let world = app.world_mut();
    displayed.extend(
        world
            .query::<(&sealed::Piece, &Transform)>()
            .iter(world)
            .map(|(_, at)| *at),
    );
    assert_eq!(displayed.len(), 5);
    let button = choice_button(&mut app, "Continue").unwrap();
    app.world_mut().trigger(Activate { entity: button });
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_millis(100));
    for _ in 0..50 {
        app.update();
        let world = app.world_mut();
        let returning: Vec<_> = world
            .query_filtered::<&Transform, With<crate::tabletop_tokens::ReturningToken>>()
            .iter(world)
            .copied()
            .collect();
        if !returning.is_empty() {
            assert_eq!(returning.len(), displayed.len());
            for at in returning {
                assert!(
                    displayed
                        .iter()
                        .any(|shown| shown.translation.abs_diff_eq(at.translation, 0.001)
                            && shown.scale.abs_diff_eq(at.scale, 0.001)),
                    "Departure {at:?} is absent from {displayed:?}"
                );
            }
            return;
        }
        let latest: Vec<_> = world
            .query::<(&sealed::Piece, &Transform)>()
            .iter(world)
            .map(|(_, at)| *at)
            .collect();
        if !latest.is_empty() {
            displayed = latest;
        }
    }
    panic!("Completing the test must return its five tokens");
}

#[test]
fn olive_with_dark_prophecy_never_promises_five_when_jacqueline_will_reveal_nine() {
    let mut session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tokens(
        crate::gathering::TokenFixture::ProphecyOlive,
    ));
    for _ in 0..4 {
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
    for prefix in ["tokens:olive:", "tokens:prophecy:0.0:"] {
        let view = session.observe();
        let choice = view
            .choices
            .iter()
            .find(|choice| choice.id.starts_with(prefix))
            .unwrap();
        session.choose(view.revision, &choice.id).unwrap();
    }
    let view = session.observe();
    assert_eq!(
        view.play
            .as_ref()
            .unwrap()
            .tokens
            .as_ref()
            .unwrap()
            .fresh_reveal_count(),
        Some(7)
    );
    let shown = controls::source_choices(&view, &controls::Source::Bag);
    let choice = shown
        .iter()
        .find(|choice| choice.id == "tokens:jacqueline:at:0")
        .unwrap();
    assert!(!choice.label.contains("reveal 5"), "{}", choice.label);
    assert!(choice.label.contains("add 2"));
    session.choose(view.revision, &choice.id).unwrap();
    assert_eq!(
        session
            .observe()
            .play
            .unwrap()
            .tokens
            .unwrap()
            .fresh_reveal_count(),
        Some(9)
    );
}

#[test]
fn olive_activation_undo_and_stale_input_preserve_exhaustion_and_the_saved_plan() {
    let (_, session) = super::review_saves::olive_review::checkpoints().remove(0);
    let original = session.observe();
    let olive = original
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("tokens:olive:"))
        .unwrap()
        .id
        .clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("olive.json");
    let mut app = host_with_table(Table::new(session, path.clone()), true);
    let button = choice_button(&mut app, "Use Olive — reveal 3, resolve 2").unwrap();
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    let after = app.world().resource::<Table>().session.clone();
    assert_eq!(
        after
            .observe()
            .play
            .unwrap()
            .tokens
            .unwrap()
            .fresh_reveal_count(),
        Some(3)
    );
    assert!(
        !after.observe().play.unwrap().players[2]
            .assets
            .iter()
            .find(|asset| asset.card.code == "04197")
            .unwrap()
            .ready
    );
    let mut rejected = after.clone();
    assert!(rejected.choose(after.observe().revision, &olive).is_err());
    assert_eq!(rejected, after);
    let epoch = app.world().resource::<Table>().epoch;
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Choose(epoch, original.revision, olive.clone()));
    app.update();
    assert_eq!(app.world().resource::<Table>().session, after);
    assert_eq!(continuity::load_session(&path).unwrap(), after);
    activate_choice(&mut app, "undo");
    let undone = app.world().resource::<Table>().session.clone();
    assert!(
        undone
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == olive)
    );
    assert!(
        undone.observe().play.unwrap().players[2]
            .assets
            .iter()
            .find(|asset| asset.card.code == "04197")
            .unwrap()
            .ready
    );
    assert_eq!(
        undone
            .observe()
            .play
            .unwrap()
            .tokens
            .unwrap()
            .fresh_reveal_count(),
        Some(1)
    );
    assert_eq!(continuity::load_session(&path).unwrap(), undone);
}

#[test]
fn olive_optional_jacqueline_controls_preserve_every_legal_scope() {
    let (_, session) = super::review_saves::olive_review::checkpoints().remove(1);
    let before = session.observe();
    let choices: Vec<_> = before
        .choices
        .iter()
        .filter(|choice| choice.id.starts_with("tokens:jacqueline"))
        .collect();
    assert!(choices.len() >= 3);
    let mut labels = std::collections::HashSet::new();
    for choice in choices {
        let label = before
            .play
            .as_ref()
            .unwrap()
            .tokens
            .as_ref()
            .unwrap()
            .ability_label(choice)
            .unwrap();
        assert!(
            labels.insert(label.clone()),
            "Different scopes need distinct labels"
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scope.json");
        let mut app = host_with_table(Table::new(session.clone(), path.clone()), true);
        let disclosure = choice_button(&mut app, "▸ Use Jacqueline · show").unwrap();
        app.world_mut().trigger(Activate { entity: disclosure });
        app.update();
        assert_eq!(app.world().resource::<Table>().session, session);
        let button = choice_button(&mut app, &label).unwrap();
        app.world_mut().trigger(Activate { entity: button });
        app.update();
        let mut expected = session.clone();
        expected.choose(before.revision, &choice.id).unwrap();
        assert_eq!(app.world().resource::<Table>().session, expected);
        assert_eq!(continuity::load_session(path).unwrap(), expected);
        assert!(
            expected
                .observe()
                .play
                .unwrap()
                .tokens
                .unwrap()
                .revealed
                .is_empty()
        );
    }
}

#[test]
fn nested_group_badges_identify_current_choice_members_without_including_ignored_tokens() {
    let (_, mut session) = super::review_saves::olive_review::checkpoints().remove(1);
    for id in [
        "tokens:jacqueline",
        "test:reveal",
        "tokens:keep:0",
        "tokens:keep:1",
        "tokens:confirm",
    ] {
        session.choose(session.observe().revision, id).unwrap();
    }
    let view = session.observe();
    let tokens = view.play.as_ref().unwrap().tokens.as_ref().unwrap();
    assert_eq!(
        tokens.choice_members("tokens:cancel-position:0"),
        vec![0, 1]
    );
    assert!(tokens.ignored.contains(&2));
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("groups.json")), true);
    finish_presentation(&mut app);
    let world = app.world_mut();
    let badges: Vec<_> = world
        .query::<(&Text, &WorldLabel)>()
        .iter(world)
        .filter(|(text, _)| text.0.starts_with("Group "))
        .map(|(text, _)| text.0.clone())
        .collect();
    assert_eq!(
        badges
            .iter()
            .filter(|text| text.as_str() == "Group 1")
            .count(),
        2
    );
    assert_eq!(
        badges.len(),
        4,
        "Only four retained tokens belong to the cancellation choices"
    );
}

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
fn olive_and_jacquelines_five_tokens_fit_compactly_inside_test() {
    let mut session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tokens(
        crate::gathering::TokenFixture::JacquelineOlive,
    ));
    for _ in 0..4 {
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
    let olive = session
        .observe()
        .choices
        .into_iter()
        .find(|choice| choice.id.starts_with("tokens:olive:"))
        .unwrap()
        .id;
    for id in [olive.as_str(), "tokens:jacqueline:at:0", "test:reveal"] {
        session.choose(session.observe().revision, id).unwrap();
    }
    assert_eq!(
        session
            .observe()
            .play
            .unwrap()
            .tokens
            .unwrap()
            .revealed
            .len(),
        5
    );
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("olive.json")), true);
    finish_presentation(&mut app);
    let world = app.world_mut();
    let mut tokens: Vec<_> = world
        .query::<(&ContextTarget, &Transform)>()
        .iter(world)
        .filter_map(|(target, transform)| match target.source {
            controls::Source::Token(index) => Some((index, transform.translation)),
            _ => None,
        })
        .collect();
    tokens.sort_by_key(|(index, _)| *index);
    assert_eq!(tokens.len(), 5);
    for (_, at) in &tokens {
        // Accepted lower region: x +/-6.7, z 9.6..12.7. Include the .58 ring.
        assert!(at.x - 0.58 >= -6.7 && at.x + 0.58 <= 6.7, "{at:?}");
        assert!(at.z - 0.58 >= 9.6 && at.z + 0.58 <= 12.7, "{at:?}");
    }
    assert!(tokens[0].1.x < -5., "Start at the upper left");
    for pair in tokens.windows(2) {
        assert!((pair[0].1.z - pair[1].1.z).abs() < 0.001);
        let gap = pair[1].1.x - pair[0].1.x - 1.16;
        assert!((0.1..=0.35).contains(&gap), "Comfortable small gap: {gap}");
    }
}

#[test]
fn chaos_art_supplies_identity_and_disks_keep_accessible_names() {
    let session = super::review_saves::resolution_review::checkpoints()
        .unwrap()
        .into_iter()
        .find(|point| point.name == "jacqueline-three")
        .unwrap()
        .session;
    let faces = session.observe().play.unwrap().tokens.unwrap().revealed;
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("art.json")), true);
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    app.insert_resource(art::Art::with_images_for_test(
        faces
            .iter()
            .map(|face| (crate::tabletop_tokens::image_role(*face), handle.clone())),
    ));
    app.world_mut().resource_mut::<Table>().dirty = true;
    for _ in 0..4 {
        app.update();
    }
    finish_presentation(&mut app);
    let world = app.world_mut();
    let captions: Vec<_> = world
        .query::<(&Text, &crate::tabletop_tokens::Follow)>()
        .iter(world)
        .map(|(text, _)| text.0.clone())
        .collect();
    assert!(
        captions.is_empty(),
        "Artwork supplies token identity: {captions:?}"
    );
    for face in &faces {
        assert!(
            world
                .query_filtered::<&AccessibleLabel, With<sealed::Piece>>()
                .iter(world)
                .any(|label| label.0.contains(face.label()))
        );
    }
}

#[test]
fn jacquelines_three_tokens_share_one_row_and_clear_meaningful_headings() {
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
    assert_tokens_and_headings_are_separate(session, true);
}

#[test]
fn voice_of_ra_nested_tokens_clear_their_group_headings() {
    assert_tokens_and_headings_are_separate(voice_ra_selection(), false);
}

fn assert_tokens_and_headings_are_separate(session: Session, ordinary: bool) {
    use crate::tabletop_layout::heading_checks::{mesh_bounds, rendered_text_bounds, viewport};
    let group_names: Vec<_> = session
        .observe()
        .play
        .unwrap()
        .tokens
        .unwrap()
        .reveal_groups()
        .into_iter()
        .map(|group| crate::tabletop_support::readable_text(group.source))
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
            let tokens: Vec<_> = world
                .query_filtered::<(Entity, &AccessibleLabel), With<sealed::Piece>>()
                .iter(world)
                .map(|(entity, label)| (entity, label.0.clone()))
                .collect();
            assert_eq!(tokens.len(), if ordinary { 3 } else { 9 });
            if ordinary {
                let z = world.get::<Transform>(tokens[0].0).unwrap().translation.z;
                assert!(tokens.iter().all(|(token, _)| {
                    (world.get::<Transform>(*token).unwrap().translation.z - z).abs() < 0.001
                }));
            }
            let (camera, camera_at) = world
                .query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>()
                .single(world)
                .unwrap();
            let token_bounds: Vec<_> = tokens
                .iter()
                .map(|(entity, label)| {
                    (
                        label.clone(),
                        mesh_bounds(world, camera, camera_at, *entity),
                    )
                })
                .collect();
            // Retain the existing physical-token geometry. This check previously
            // measured routine captions; only group/card captions remain now.
            for (name, bounds) in token_bounds {
                assert!(
                    bounds.min.cmpge(Vec2::ZERO).all() && bounds.max.cmple(size).all(),
                    "{name}: {bounds:?}"
                );
            }
            let mut captions = Vec::new();
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
    let disclosure = choice_button(&mut app, "▸ Use Jacqueline · show").unwrap();
    crate::tabletop_layout::font_checks::assert_supported(app.world_mut());
    assert_eq!(
        app.world().get::<BackgroundColor>(disclosure).unwrap().0,
        Color::NONE
    );
    app.world_mut().trigger(Activate { entity: disclosure });
    app.update();
    assert_eq!(app.world().resource::<Table>().session, before);
    let expanded = choice_button(&mut app, "▾ Use Jacqueline · hide").unwrap();
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
fn selected_token_badge_is_opaque_green_and_clear_of_the_printed_face() {
    use bevy::camera::primitives::MeshAabb;
    let mut session = voice_ra_selection();
    session
        .choose(session.observe().revision, "tokens:keep:3")
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session, directory.path().join("badge.json")),
        true,
    );
    finish_presentation(&mut app);
    let world = app.world_mut();
    let children = world
        .query::<(&AccessibleLabel, &Children)>()
        .iter(world)
        .find(|(label, _)| label.0.ends_with("\nSelected"))
        .expect("Selected token")
        .1
        .to_vec();
    let materials = world.resource::<Assets<StandardMaterial>>();
    let meshes = world.resource::<Assets<Mesh>>();
    let mut dark_backing = false;
    let mut green_strokes = 0;
    for child in children {
        let entity = world.entity(child);
        let (Some(transform), Some(mesh), Some(material)) = (
            entity.get::<Transform>(),
            entity.get::<Mesh3d>(),
            entity.get::<MeshMaterial3d<StandardMaterial>>(),
        ) else {
            continue;
        };
        if transform.translation.x < 0.25 || transform.translation.z < 0.25 {
            continue;
        }
        let material = materials.get(&material.0).unwrap();
        let color = material.base_color.to_srgba();
        let green = color.green > 0.6 && color.green > color.red * 1.5;
        let dark = color.red < 0.15 && color.green < 0.15 && color.blue < 0.15;
        if !green && !dark {
            continue;
        }
        assert_eq!(material.alpha_mode, AlphaMode::Opaque);
        assert!(material.unlit);
        assert!(
            entity
                .get::<Pickable>()
                .is_some_and(|pickable| !pickable.is_hoverable)
        );
        let bounds = meshes.get(&mesh.0).unwrap().get_aabb().unwrap();
        for x in [bounds.min().x, bounds.max().x] {
            for z in [bounds.min().z, bounds.max().z] {
                let point = transform.transform_point(Vec3::new(x, 0., z));
                assert!(
                    point.x <= 0.5801 && point.z <= 0.5801,
                    "Badge must fit the existing token spacing: {point}"
                );
                assert!(
                    Vec2::new(point.x, point.z).length() > 0.25,
                    "Printed center stays clear"
                );
            }
        }
        dark_backing |= dark;
        green_strokes += usize::from(green);
    }
    assert!(
        dark_backing,
        "Selection needs an opaque badge at the lower-right edge"
    );
    assert_eq!(green_strokes, 2, "Two bold green strokes form the check");
}

#[test]
fn premonition_sealed_tokens_are_not_marked_as_selected() {
    use crate::gathering::{HostileFixture, TokenFixture};
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::PremonitionVoice));
    session
        .choose(session.observe().revision, "test:reveal")
        .unwrap();
    assert!(session.observe().play.unwrap().premonition.is_some());
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session, directory.path().join("sealed.json")),
        true,
    );
    finish_presentation(&mut app);
    let world = app.world_mut();
    let labels: Vec<_> = world
        .query::<&AccessibleLabel>()
        .iter(world)
        .filter(|label| label.0.starts_with("Token "))
        .map(|label| label.0.clone())
        .collect();
    assert!(
        labels.iter().any(|label| label.ends_with("\nSealed")),
        "Sealed token must say Sealed: {labels:?}"
    );
    assert!(!labels.iter().any(|label| label.ends_with("\nSelected")));
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
        sideboard::focus(Area::Overview).0
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
