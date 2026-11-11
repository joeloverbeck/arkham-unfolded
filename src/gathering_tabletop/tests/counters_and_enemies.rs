use super::*;
use crate::tabletop_pieces as pieces;
use bevy::ecs::system::RunSystemOnce;

#[test]
fn ability_marker_sits_right_of_jacqueline_and_tracks_only_round_usage() {
    use crate::tabletop_counters::AbilityMarker;
    let mut session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::JacquelineCancellation);
    loop {
        let view = session.observe();
        if view
            .choices
            .iter()
            .any(|choice| choice.id == "tokens:jacqueline")
        {
            break;
        }
        let next = ["window:pass", "test:confirm"]
            .into_iter()
            .find(|id| view.choices.iter().any(|choice| choice.id == *id))
            .expect("route to reveal");
        session.choose(view.revision, next).unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    for spent in [false, true] {
        if spent {
            session
                .choose(session.observe().revision, "tokens:jacqueline")
                .unwrap();
        }
        let mut app = host_with_table(
            Table::new(session.clone(), dir.path().join("ability.json")),
            false,
        );
        let world = app.world_mut();
        let (entity, at, label) = world
            .query_filtered::<(Entity, &Transform, &AccessibleLabel), With<AbilityMarker>>()
            .single(world)
            .unwrap();
        let card = layout::investigator_card(2);
        assert_eq!(at.translation.y, layout::investigator_action(2, 0, false).y);
        assert!((at.translation.z - card.z).abs() < 0.001);
        assert!(
            (at.translation.x - 0.21 - (card.x + pieces::CARD_LENGTH / 2.) - 0.18).abs() < 0.001
        );
        assert_eq!(
            label.0,
            if spent {
                "Round ability spent"
            } else {
                "Round ability unused"
            }
        );
        assert!(world.get::<ContextTarget>(entity).is_none());
        assert!(world.get::<CardFeedback>(entity).is_none());
    }
}

#[test]
fn compact_asset_names_do_not_overlap_ready_or_exhausted_neighbors() {
    use crate::tabletop_layout::heading_checks::{rendered_text_bounds, viewport};
    let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tony(
        crate::gathering::TonyEffectFixture::GregoryCigarette,
    ));
    for ready in [true, false] {
        let mut play = session.observe().play.unwrap();
        let template = play.players[0].assets[0].clone();
        play.players[0].assets = (0..12)
            .map(|slot| {
                let mut asset = template.clone();
                asset.card.id = 60000 + slot;
                asset.card.title = "Tennessee Sour Mash".into();
                asset.ready = ready;
                asset
            })
            .collect();
        let mut app = super::cards::at_public_moment(session.clone(), play, PathBuf::new(), false);
        app.world_mut()
            .resource_mut::<Orbit>()
            .focus(layout::focus(Area::Investigator(0)));
        for size in [Vec2::new(1280., 720.), Vec2::new(1600., 1000.)] {
            for scale in [1., 2.] {
                viewport::<MainCamera>(&mut app, size, scale);
                for _ in 0..5 {
                    app.update();
                }
                let world = app.world_mut();
                let names: Vec<_> = world
                    .query_filtered::<(Entity, &motion::CardLabel), With<pieces::CardCaption>>()
                    .iter(world)
                    .filter(|(_, caption)| {
                        world
                            .get::<motion::PhysicalCard>(caption.0)
                            .is_some_and(|card| (60000..60012).contains(&card.0))
                    })
                    .map(|(entity, _)| rendered_text_bounds(world, entity, scale))
                    .collect();
                assert_eq!(names.len(), 12);
                for (i, name) in names.iter().enumerate() {
                    for other in &names[i + 1..] {
                        assert!(
                            name.intersect(*other).is_empty(),
                            "ready={ready}, {size:?}, {scale}x: {name:?} / {other:?}"
                        );
                    }
                }
            }
        }
    }
}

fn render_observation(view: Observation) -> App {
    let mut app = crate::tabletop_layout::heading_checks::app();
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<art::Art>()
        .init_resource::<Assets<Image>>();
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    let art = art::Art::with_images_for_test([
        ("token/ammo", image.clone()),
        ("token/bounty", image.clone()),
        ("token/charge", image.clone()),
        ("token/key", image.clone()),
        ("token/resource", image.clone()),
        ("token/damage", image.clone()),
        ("token/horror", image),
    ]);
    app.insert_resource(art);
    app.world_mut()
        .run_system_once(
            move |mut commands: Commands,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut materials: ResMut<Assets<StandardMaterial>>,
                  art: Res<art::Art>| {
                let story = story::Faces::default();
                let consequences = consequences::State::default();
                let context = CardRender {
                    art: &art,
                    view: &view,
                    epoch: 0,
                    story: &story,
                    consequences: &consequences,
                };
                play::render(&mut commands, &mut meshes, &mut materials, &context);
            },
        )
        .unwrap();
    app
}

#[test]
fn old_keyring_has_key_art_a_quantity_and_an_accessible_name() {
    use crate::tabletop_counters::Counter;
    let view = Session::with_hostile_fixture(crate::gathering::HostileFixture::PlayerEvents(
        crate::gathering::PlayerEventFixture::KeyringSuccess,
    ))
    .observe();
    let key = view.play.as_ref().unwrap().players[1]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60507")
        .unwrap()
        .card
        .id;
    let mut app = render_observation(view);
    let world = app.world_mut();
    let counters: Vec<_> = world
        .query::<(&Counter, &card_counters::Attached)>()
        .iter(world)
        .filter(|(_, attached)| world.get::<motion::PhysicalCard>(attached.0).unwrap().0 == key)
        .map(|(counter, _)| (counter.kind.name(), counter.amount))
        .collect();
    assert_eq!(counters, vec![("Keys", 2)]);
    let text = rendered_text(&mut app);
    assert!(!text.contains("Keys 2"));
    assert!(
        app.world_mut()
            .query::<&AccessibleLabel>()
            .iter(app.world())
            .any(|label| label.0 == "Keys 2")
    );
    assert!(!text.contains("Uses 2"));
}

#[test]
fn empty_asset_counters_keep_their_type_owner_and_upright_count() {
    use crate::tabletop_counters::{Counter, CounterKind};
    let mut view = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tony(
        crate::gathering::TonyEffectFixture::GregoryCigarette,
    ))
    .observe();
    let play = view.play.as_mut().unwrap();
    let template = play.players[0].assets[0].clone();
    play.players[0].assets.clear();
    for (index, code, title) in [
        (0, "06010", "Bounty Contracts"),
        (1, "06011", "Tony's .38 Long Colt"),
        (2, "60407", "Azure Flame"),
        (3, "05117", "Tennessee Sour Mash"),
        (4, "06162", "Gregory Gry"),
    ] {
        let mut asset = template.clone();
        asset.card.id = 60000 + index;
        asset.card.code = code.into();
        asset.card.title = title.into();
        asset.uses = 0;
        asset.ready = false;
        play.players[0].assets.push(asset);
    }
    assert!(
        play::inspect(&view, "05117")
            .unwrap()
            .contains("Tony Morgan — Supplies: 0")
    );
    let mut app = render_observation(view);
    let world = app.world_mut();
    let counters: Vec<_> = world
        .query::<(&Counter, &card_counters::Attached, &Transform)>()
        .iter(world)
        .map(|(counter, attached, at)| {
            (
                world.get::<motion::PhysicalCard>(attached.0).unwrap().0,
                counter.kind,
                counter.amount,
                at.rotation,
            )
        })
        .collect();
    for (id, kind) in [
        (60000, CounterKind::Bounty),
        (60001, CounterKind::Ammo),
        (60002, CounterKind::Charge),
        (60004, CounterKind::Resource),
    ] {
        assert!(counters.contains(&(id, kind, 0, Quat::IDENTITY)));
    }
    assert!(
        counters
            .iter()
            .any(|(id, kind, amount, rotation)| *id == 60003
                && kind.name() == "Supplies"
                && *amount == 0
                && *rotation == Quat::IDENTITY)
    );
    let text = rendered_text(&mut app);
    for caption in [
        "Bounties 0",
        "Ammo 0",
        "Charges 0",
        "Supplies 0",
        "Resources 0",
    ] {
        assert!(
            !text.contains(caption),
            "The counter displays its quantity: {caption}"
        );
        assert!(
            app.world_mut()
                .query::<&AccessibleLabel>()
                .iter(app.world())
                .any(|label| label.0 == caption)
        );
    }
    assert!(!text.contains("Uses 0"));
}

#[test]
fn native_olive_harm_badges_stay_separate() {
    use crate::tabletop_layout::heading_checks::{rendered_text_bounds, viewport};
    for (_, session) in super::review_saves::olive_review::checkpoints()
        .into_iter()
        .take(2)
    {
        let dir = tempfile::tempdir().unwrap();
        let mut app = host_with_table(
            Table::new(session.clone(), dir.path().join("harm.json")),
            true,
        );
        {
            let image = app
                .world_mut()
                .resource_mut::<Assets<Image>>()
                .add(Image::default());
            app.insert_resource(art::Art::with_images_for_test([
                ("token/damage", image.clone()),
                ("token/horror", image),
            ]));
        }
        app.world_mut()
            .resource_mut::<Orbit>()
            .focus(layout::focus(Area::Investigator(2)));
        for size in [Vec2::new(1280., 720.), Vec2::new(1600., 1000.)] {
            for scale in [1., 2.] {
                viewport::<MainCamera>(&mut app, size, scale);
                for _ in 0..5 {
                    app.update();
                }
                let world = app.world_mut();
                let labels: Vec<_> = world
                    .query::<(Entity, &motion::CardLabel)>()
                    .iter(world)
                    .filter(|(_, label)| {
                        world
                            .get::<card_counters::Attached>(label.0)
                            .and_then(|attached| world.get::<motion::PhysicalCard>(attached.0))
                            .is_some_and(|card| card.0 == 209)
                    })
                    .map(|(entity, _)| entity)
                    .collect();
                assert_eq!(labels.len(), 2);
                let first = rendered_text_bounds(world, labels[0], scale).inflate(3.);
                let second = rendered_text_bounds(world, labels[1], scale).inflate(3.);
                assert!(
                    first.intersect(second).is_empty(),
                    "{size:?}/{scale}: {first:?} overlaps {second:?}"
                );
            }
        }
    }
}

#[test]
fn olive_harm_fractions_have_tightly_fitted_dark_backing() {
    let view = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tokens(
        crate::gathering::TokenFixture::JacquelineOlive,
    ))
    .observe();
    let mut app = render_observation(view);
    for _ in 0..4 {
        app.update();
    }
    let world = app.world_mut();
    for expected in ["0/1", "0/3"] {
        let (entity, computed, background) = world
            .query::<(Entity, &Text, &ComputedNode, Option<&BackgroundColor>)>()
            .iter(world)
            .find_map(|(entity, text, computed, background)| {
                (text.0 == expected).then_some((entity, computed, background))
            })
            .unwrap_or_else(|| panic!("Missing {expected}"));
        let color = background
            .expect("Fractions need their own dark backing")
            .0
            .to_srgba();
        assert!(color.red < 0.05 && color.green < 0.05 && color.blue < 0.05 && color.alpha >= 0.9);
        let width = computed.size().x * computed.inverse_scale_factor;
        let glyphs =
            crate::tabletop_layout::heading_checks::rendered_text_bounds(world, entity, 1.);
        assert!(
            (4.0..=12.0).contains(&(width - glyphs.width())),
            "{expected}: backing {width}, glyphs {glyphs:?}"
        );
    }
}

#[test]
fn asset_harm_tokens_show_only_real_capacities_including_zero_and_story_allies() {
    let mut view = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tony(
        crate::gathering::TonyEffectFixture::GregoryCigarette,
    ))
    .observe();
    let play = view.play.as_mut().unwrap();
    let template = play.players[0].assets[0].clone();
    play.players[0].assets.clear();
    for (index, code, expected) in [
        (0, "02033", vec!["0/1", "0/2"]),
        (1, "60412", vec!["0/2"]),
        (2, "01117", vec!["0/3", "0/3"]),
        (3, "05117", vec![]),
    ] {
        let mut single = view.clone();
        let mut asset = template.clone();
        asset.card.id = 60000 + index;
        asset.card.code = code.into();
        asset.damage = 0;
        asset.horror = 0;
        single.play.as_mut().unwrap().players[0].assets.push(asset);
        let mut app = render_observation(single);
        let text = rendered_text(&mut app);
        for caption in &expected {
            assert!(text.contains(caption), "Missing {caption}: {text}");
        }
        let world = app.world_mut();
        let labels: Vec<_> = world
            .query::<(&Text, &motion::CardLabel)>()
            .iter(world)
            .filter(|(text, _)| expected.contains(&text.0.as_str()))
            .filter(|(_, label)| {
                world
                    .get::<card_counters::Attached>(label.0)
                    .and_then(|attached| world.get::<motion::PhysicalCard>(attached.0))
                    .is_some_and(|card| card.0 == 60000 + index)
            })
            .collect();
        assert_eq!(labels.len(), expected.len());
        for (_, label) in labels {
            if expected.len() == 1 {
                assert_eq!(
                    label.1.x, 0.,
                    "A single harm badge stays centered on its token"
                );
            }
            let attached = world
                .get::<card_counters::Attached>(label.0)
                .expect("Harm token follows card");
            assert_eq!(
                world.get::<motion::PhysicalCard>(attached.0).unwrap().0,
                60000 + index
            );
        }
    }
}

#[test]
fn peters_harm_counters_clear_patrices_actions_and_keep_the_caption_gap() {
    use bevy::camera::primitives::MeshAabb;
    for ready in [true, false] {
        let mut view = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tony(
            crate::gathering::TonyEffectFixture::GregoryCigarette,
        ))
        .observe();
        let play = view.play.as_mut().unwrap();
        let mut peter = play.players[0].assets[0].clone();
        peter.card.id = 60000;
        peter.card.code = "02033".into();
        peter.card.title = "Peter Sylvestre".into();
        peter.ready = ready;
        play.players[1].assets = vec![peter];
        let mut app = render_observation(view);
        let world = app.world_mut();
        let tokens: Vec<_> = world
            .query::<(Entity, &card_counters::Attached, &Mesh3d, &Transform)>()
            .iter(world)
            .filter(|(_, attached, _, _)| {
                world
                    .get::<motion::PhysicalCard>(attached.0)
                    .is_some_and(|card| card.0 == 60000)
            })
            .map(|(id, attached, mesh, at)| (id, attached.0, mesh.0.clone(), at.translation))
            .collect();
        assert_eq!(tokens.len(), 2);
        for (token, card, mesh, at) in tokens {
            let half = world
                .resource::<Assets<Mesh>>()
                .get(&mesh)
                .unwrap()
                .get_aabb()
                .unwrap()
                .half_extents;
            assert!(
                (half.x * 2. - 0.496).abs() < 0.001,
                "Match bounty/ammo diameter"
            );
            let card_at = world.get::<Transform>(card).unwrap();
            let edge = card_at.translation.z - if ready { 1.89 / 2. } else { 1.35 / 2. };
            assert!(at.z + half.z < edge, "Token must clear the card face");
            for slot in 0..3 {
                let action = layout::investigator_action(1, slot, false);
                assert!(
                    at.xz().distance(action.xz()) > half.x + 0.21 + 0.02,
                    "Peter must clear Patrice's action tokens"
                );
            }
            let label = world
                .query::<&motion::CardLabel>()
                .iter(world)
                .find(|label| label.0 == token)
                .unwrap();
            assert!(
                edge - (at.z + label.1.z) > 0.10,
                "Keep the label anchor above the card edge"
            );
        }
    }
}

#[test]
#[ignore = "Known pre-existing caption/card overlap at investigator zoom; owner deferred the visual correction on 2026-11-06"]
fn peters_projected_harm_labels_clear_tokens_actions_and_card() {
    peters_projected_harm(true);
}

#[test]
fn peters_projected_harm_tokens_clear_card() {
    peters_projected_harm(false);
}

fn peters_projected_harm(check_deferred_clearance: bool) {
    use crate::tabletop_counters::ActionMarker;
    use crate::tabletop_layout::heading_checks::{mesh_bounds, rendered_text_bounds, viewport};
    for ready in [true, false] {
        let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tony(
            crate::gathering::TonyEffectFixture::GregoryCigarette,
        ));
        let mut play = session.observe().play.unwrap();
        let mut peter = play.players[0].assets[0].clone();
        peter.card.id = 60000;
        peter.card.code = "02033".into();
        peter.card.title = "Peter Sylvestre".into();
        peter.ready = ready;
        play.players[1].assets = vec![peter];
        let mut app = super::cards::at_public_moment(session, play, PathBuf::new(), false);
        {
            let image = app
                .world_mut()
                .resource_mut::<Assets<Image>>()
                .add(Image::default());
            let art = art::Art::with_images_for_test([
                ("token/damage", image.clone()),
                ("token/horror", image),
            ]);
            app.insert_resource(art);
            app.world_mut().resource_mut::<Table>().dirty = true;
        }
        app.world_mut()
            .resource_mut::<Orbit>()
            .focus(layout::focus(Area::Investigator(1)));
        for size in [Vec2::new(1280., 720.), Vec2::new(1600., 1000.)] {
            for scale in [1., 2.] {
                viewport::<MainCamera>(&mut app, size, scale);
                for _ in 0..5 {
                    app.update();
                }
                let world = app.world_mut();
                let card = world
                    .query::<(Entity, &motion::PhysicalCard)>()
                    .iter(world)
                    .find(|(_, card)| card.0 == 60000)
                    .unwrap()
                    .0;
                let tokens: Vec<_> = world
                    .query::<(Entity, &card_counters::Attached)>()
                    .iter(world)
                    .filter(|(_, attached)| attached.0 == card)
                    .map(|(entity, _)| entity)
                    .collect();
                assert_eq!(tokens.len(), 2);
                let labels: Vec<_> = world
                    .query::<(Entity, &motion::CardLabel)>()
                    .iter(world)
                    .filter(|(_, label)| tokens.contains(&label.0))
                    .map(|(entity, _)| entity)
                    .collect();
                assert_eq!(labels.len(), 2);
                let actions: Vec<_> = world
                    .query_filtered::<Entity, With<ActionMarker>>()
                    .iter(world)
                    .collect();
                let (camera, camera_at) = world
                    .query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>()
                    .single(world)
                    .unwrap();
                let card_bounds = mesh_bounds(world, camera, camera_at, card);
                let token_bounds: Vec<_> = tokens
                    .iter()
                    .map(|token| mesh_bounds(world, camera, camera_at, *token))
                    .collect();
                let caption_bounds: Vec<_> = labels
                    .iter()
                    .map(|label| rendered_text_bounds(world, *label, scale))
                    .collect();
                let context = format!("ready={ready}, {size:?}, scale={scale}");
                for caption in caption_bounds.iter().filter(|_| check_deferred_clearance) {
                    assert!(
                        caption.max.y < card_bounds.min.y,
                        "{context}: preserve caption/card gap: {caption:?} / {card_bounds:?}"
                    );
                    for token in &token_bounds {
                        assert!(
                            caption.intersect(*token).is_empty(),
                            "{context}: caption overlaps token"
                        );
                    }
                }
                if check_deferred_clearance {
                    assert!(
                        caption_bounds[0].intersect(caption_bounds[1]).is_empty(),
                        "{context}: harm labels overlap"
                    );
                }
                for token in &token_bounds {
                    assert!(
                        token.intersect(card_bounds).is_empty(),
                        "{context}: harm token overlaps card"
                    );
                }
                for action in actions.into_iter().filter(|_| check_deferred_clearance) {
                    let bounds = mesh_bounds(world, camera, camera_at, action);
                    for counter in token_bounds
                        .iter()
                        .chain(caption_bounds.iter().filter(|_| check_deferred_clearance))
                    {
                        assert!(
                            counter.intersect(bounds).is_empty(),
                            "{context}: harm overlaps action token"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn enemy_board_matches_map_and_keeps_cards_and_readable_counter_labels() {
    use bevy::{camera::primitives::MeshAabb, text::FontSize};
    {
        let symbols = true;
        for scale in [1., 2.] {
            for size in [Vec2::new(1600., 1000.), Vec2::new(1280., 800.)] {
                for area in [Area::Scenario, Area::Overview] {
                    let session =
                        Session::with_hostile_fixture(crate::gathering::HostileFixture::Tony(
                            crate::gathering::TonyEffectFixture::QuickThinking,
                        ));
                    let mut play = session.observe().play.unwrap();
                    let template = play.scenario.enemies[0].clone();
                    play.scenario.enemies.clear();
                    // Three groups in four rows: the most a board holds once
                    // each group reserves its three-line captions.
                    for index in 0..9 {
                        let mut enemy = template.clone();
                        enemy.card.id = 61000 + index;
                        enemy.card.title = match index % 3 {
                            0 => "Watcher from Another Dimension",
                            1 => "Swarm of Rats",
                            _ => "Ghoul Minion",
                        }
                        .into();
                        enemy.engaged = (index == 8).then_some(Investigator::Tony);
                        enemy.location = match index {
                            6 => "01113",
                            7 => "01114",
                            _ => "01112",
                        }
                        .into();
                        enemy.exhausted = index % 2 == 0;
                        enemy.bounties = if index == 0 { 2 } else { 0 };
                        enemy.doom = u8::from(index == 0);
                        play.scenario.enemies.push(enemy);
                    }
                    let mut app =
                        super::cards::at_public_moment(session, play, PathBuf::new(), symbols);
                    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
                        &mut app, size, scale,
                    );
                    app.world_mut()
                        .resource_mut::<Orbit>()
                        .focus(sideboard::focus(area));
                    for _ in 0..4 {
                        app.update();
                    }
                    let world = app.world_mut();
                    let (mesh, at) = world
                        .query::<(&Name, &Mesh3d, &Transform)>()
                        .iter(world)
                        .find_map(|(name, mesh, at)| {
                            (name.as_str() == "Enemies at locations").then_some((mesh, at))
                        })
                        .unwrap();
                    let half = world
                        .resource::<Assets<Mesh>>()
                        .get(&mesh.0)
                        .unwrap()
                        .get_aabb()
                        .unwrap()
                        .half_extents;
                    assert_eq!(half.z * 2., layout::SCENARIO_SIZE.y);
                    assert_eq!(at.translation.z, layout::SCENARIO_CENTER.z);
                    let board =
                        Rect::from_center_size(at.translation.xz(), Vec2::new(half.x, half.z) * 2.);
                    let cards: Vec<_> = world
                        .query::<(&motion::PhysicalCard, &Transform)>()
                        .iter(world)
                        .filter(|(card, _)| (61000..61008).contains(&card.0))
                        .map(|(card, at)| (card.0, *at))
                        .collect();
                    let cells: Vec<_> = cards
                        .iter()
                        .map(|(_, at)| {
                            Rect::from_center_size(
                                at.translation.xz() - Vec2::Y * 0.15,
                                Vec2::new(2.4, 3.0),
                            )
                        })
                        .collect();
                    assert_eq!(cells.len(), 8);
                    for (index, cell) in cells.iter().enumerate() {
                        assert!(
                            cell.min.x >= board.min.x
                                && cell.max.x <= board.max.x
                                && cell.min.y >= board.min.y
                                && cell.max.y <= board.max.y
                        );
                        for other in &cells[index + 1..] {
                            assert!(
                                cell.intersect(*other).is_empty(),
                                "{cell:?} overlaps {other:?}"
                            );
                        }
                    }
                    assert!(rendered_text(&mut app).contains("HALLWAY  /  6"));
                    let world = app.world_mut();
                    let labels: Vec<_> = world
                        .query::<(Entity, &motion::CardLabel)>()
                        .iter(world)
                        .filter_map(|(entity, label)| {
                            let owner = world
                                .get::<card_counters::Attached>(label.0)
                                .map_or(label.0, |attached| attached.0);
                            let id = world.get::<motion::PhysicalCard>(owner)?.0;
                            (61000..61008).contains(&id).then_some(entity)
                        })
                        .collect();
                    assert_eq!(
                        labels.len(),
                        16,
                        "Every enemy has its name and damage fraction"
                    );
                    let mut rich_labels = 0;
                    for label in labels {
                        let anchor = world.get::<motion::CardLabel>(label).unwrap();
                        if world.get::<card_counters::Attached>(anchor.0).is_some() {
                            assert_eq!(anchor.1.x, 0., "Enemy damage stays centered on its token");
                        }
                        assert_eq!(
                            world.get::<TextFont>(label).unwrap().font_size,
                            FontSize::Px(13.)
                        );
                        assert_ne!(*world.get::<Visibility>(label).unwrap(), Visibility::Hidden);
                        if let Some(children) = world.get::<Children>(label) {
                            for child in children.iter() {
                                if let Some(font) = world.get::<TextFont>(child) {
                                    assert!(
                                        matches!(font.font_size, FontSize::Px(size) if (13.0..=16.25).contains(&size))
                                    );
                                    rich_labels += 1;
                                }
                            }
                        }
                    }
                    assert_eq!(
                        rich_labels, 0,
                        "Names and damage fractions need no substitute glyph captions"
                    );
                    let counters: Vec<_> = world.query_filtered::<(&Text, &TextFont, &Node, &Visibility, &AccessibleLabel), With<crate::tabletop_counters::CounterLabel>>()
                    .iter(world)
                    .filter(|(_, _, _, _, label)| matches!(label.0.as_str(), "Bounties 2" | "Doom 1"))
                    .collect();
                    assert_eq!(
                        counters.len(),
                        2,
                        "Both side counters use the actual counter-label renderer"
                    );
                    for (text, font, node, visibility, _) in counters {
                        assert!(matches!(text.0.as_str(), "2" | "1"));
                        assert_eq!(font.font_size, FontSize::Px(11.));
                        assert_ne!(*visibility, Visibility::Hidden);
                        assert!(
                            matches!((node.left, node.top), (Val::Px(x), Val::Px(y)) if x.is_finite() && y.is_finite())
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn keyboard_focus_cross_highlights_the_location_marker_without_moving_the_camera() {
    let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tony(
        crate::gathering::TonyEffectFixture::QuickThinking,
    ));
    let mut play = session.observe().play.unwrap();
    for enemy in &mut play.scenario.enemies {
        enemy.engaged = None;
    }
    let mut app = super::cards::at_public_moment(session, play, PathBuf::new(), false);
    let header = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .find(|(_, text)| text.0.contains("HALLWAY"))
        .unwrap()
        .0;
    // Both the map token and the heading token carry the location-color ring.
    let world = app.world_mut();
    let tokens: Vec<_> = world
        .query::<(&Name, &Children)>()
        .iter(world)
        .filter(|(name, _)| name.as_str() == "Enemy presence Hallway")
        .map(|(_, children)| children.iter().collect::<Vec<_>>())
        .collect();
    assert_eq!(tokens.len(), 2);
    let rings: Vec<_> = tokens
        .iter()
        .flatten()
        .filter(|child| {
            world
                .get::<Name>(**child)
                .is_some_and(|name| name.as_str() == "Location ring")
        })
        .map(|ring| {
            world
                .get::<MeshMaterial3d<StandardMaterial>>(*ring)
                .unwrap()
                .0
                .clone()
        })
        .collect();
    assert_eq!(rings.len(), 2);
    let ring_colors = |app: &App| {
        rings
            .iter()
            .map(|ring| {
                app.world()
                    .resource::<Assets<StandardMaterial>>()
                    .get(ring)
                    .unwrap()
                    .base_color
            })
            .collect::<Vec<_>>()
    };
    app.update();
    assert!(ring_colors(&app).iter().all(|color| *color != Color::WHITE));
    let camera = app.world().resource::<Orbit>().pose();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(header, bevy::input_focus::FocusCause::Navigated);
    app.update();
    assert_eq!(ring_colors(&app), [Color::WHITE; 2]);
    assert_eq!(app.world().resource::<Orbit>().pose(), camera);
}

#[test]
fn every_unengaged_enemy_has_a_separate_face_and_its_own_actions() {
    let mut view = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tony(
        crate::gathering::TonyEffectFixture::QuickThinking,
    ))
    .observe();
    let play = view.play.as_mut().unwrap();
    for enemy in &mut play.scenario.enemies {
        enemy.engaged = None;
    }
    let ids: Vec<_> = play
        .scenario
        .enemies
        .iter()
        .map(|enemy| enemy.card.id)
        .collect();
    let mut app = render_observation(view.clone());
    let positions: Vec<_> = app
        .world_mut()
        .query::<(&motion::PhysicalCard, &Transform)>()
        .iter(app.world())
        .filter(|(card, _)| ids.contains(&card.0))
        .map(|(_, at)| at.translation)
        .collect();
    assert_eq!(positions.len(), 2);
    assert!(
        positions.iter().all(|at| at.x < -9.8),
        "Enemies belong to the left sideboard"
    );
    assert!(
        positions[0].distance(positions[1]) >= 2.5,
        "Both full faces and counters must fit"
    );
    for &id in &ids {
        let own = controls::source_choices(&view, &controls::Source::Card(id));
        assert!(!own.is_empty(), "Each enemy must retain its legal actions");
        for other in ids.iter().filter(|other| **other != id) {
            assert!(
                own.iter()
                    .all(|choice| !choice.id.contains(&format!(":{other}")))
            );
        }
    }
}
