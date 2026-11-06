use super::*;
use bevy::ecs::system::RunSystemOnce;

fn render_observation(view: Observation) -> App {
    render_with_counter_art(view, false)
}

fn render_with_counter_art(view: Observation, loaded: bool) -> App {
    let mut app = crate::tabletop_layout::heading_checks::app();
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<art::Art>()
        .init_resource::<Assets<Image>>();
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    let mut art = art::Art::with_images_for_test([
        ("token/ammo", image.clone()),
        ("token/bounty", image.clone()),
        ("token/charge", image.clone()),
        ("token/resource", image.clone()),
        ("token/damage", image),
    ]);
    if loaded {
        art.refresh_loaded(app.world().resource::<Assets<Image>>());
    }
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
fn empty_asset_counters_keep_their_type_owner_and_upright_count_with_or_without_loaded_art() {
    use crate::tabletop_counters::{Counter, CounterKind};
    for loaded in [false, true] {
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
        let mut app = render_with_counter_art(view, loaded);
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
        for caption in ["Bounties 0", "Ammo 0", "Charges 0", "Supplies 0"] {
            assert_eq!(
                text.contains(caption),
                !loaded,
                "{caption}: loaded={loaded}"
            );
        }
        assert!(!text.contains("Uses 0"));
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
        (0, "02033", vec!["Damage 0/1", "Horror 0/2"]),
        (1, "60412", vec!["Damage 0/2"]),
        (2, "01117", vec!["Damage 0/3", "Horror 0/3"]),
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
            .filter(|(text, _)| text.0.starts_with("Damage ") || text.0.starts_with("Horror "))
            .filter(|(_, label)| {
                world
                    .get::<card_counters::Attached>(label.0)
                    .and_then(|attached| world.get::<motion::PhysicalCard>(attached.0))
                    .is_some_and(|card| card.0 == 60000 + index)
            })
            .collect();
        assert_eq!(labels.len(), expected.len());
        for (_, label) in labels {
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
        let mut app = render_with_counter_art(view, true);
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
fn enemy_board_matches_map_and_keeps_cards_and_readable_counter_labels() {
    use bevy::{camera::primitives::MeshAabb, text::FontSize};
    for symbols in [false, true] {
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
                    for index in 0..13 {
                        let mut enemy = template.clone();
                        enemy.card.id = 61000 + index;
                        enemy.card.title = match index % 3 {
                            0 => "Watcher from Another Dimension",
                            1 => "Swarm of Rats",
                            _ => "Ghoul Minion",
                        }
                        .into();
                        enemy.engaged = (index == 12).then_some(Investigator::Tony);
                        enemy.location = match index {
                            9 => "01113",
                            10 => "01114",
                            11 => "01115",
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
                        .focus(enemy_board::focus(area));
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
                        .filter(|(card, _)| (61000..61012).contains(&card.0))
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
                    assert_eq!(cells.len(), 12);
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
                    assert!(rendered_text(&mut app).contains("H  /  HALLWAY  /  9"));
                    let world = app.world_mut();
                    let labels: Vec<_> = world
                        .query::<(Entity, &motion::CardLabel)>()
                        .iter(world)
                        .filter_map(|(entity, label)| {
                            let owner = world
                                .get::<card_counters::Attached>(label.0)
                                .map_or(label.0, |attached| attached.0);
                            let id = world.get::<motion::PhysicalCard>(owner)?.0;
                            (61000..61012).contains(&id).then_some(entity)
                        })
                        .collect();
                    assert_eq!(
                        labels.len(),
                        24,
                        "Every enemy has its name and damage fraction"
                    );
                    let mut rich_labels = 0;
                    for label in labels {
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
                    assert_eq!(rich_labels > 0, symbols);
                    let counters: Vec<_> = world.query_filtered::<(&Text, &TextFont, &Node, &Visibility), With<crate::tabletop_counters::CounterLabel>>()
                    .iter(world)
                    .filter(|(text, _, _, _)| matches!(text.0.as_str(), "Bounties 2" | "Doom 1"))
                    .collect();
                    assert_eq!(
                        counters.len(),
                        2,
                        "Both side counters use the actual counter-label renderer"
                    );
                    for (_, font, node, visibility) in counters {
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
    let marker = app
        .world_mut()
        .query::<(&Name, &MeshMaterial3d<StandardMaterial>)>()
        .iter(app.world())
        .find(|(name, _)| name.as_str() == "Enemy presence H")
        .unwrap()
        .1
        .0
        .clone();
    let camera = app.world().resource::<Orbit>().pose();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(header, bevy::input_focus::FocusCause::Navigated);
    app.update();
    assert_eq!(
        app.world()
            .resource::<Assets<StandardMaterial>>()
            .get(&marker)
            .unwrap()
            .base_color,
        Color::WHITE
    );
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
