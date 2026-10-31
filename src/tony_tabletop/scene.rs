//! Physical locations for the existing prepared position, not additional game state.
use super::*;
use crate::tabletop_discard as discard;

const STUDY_AT: Vec3 = Vec3::new(-2.5, 0.05, -3.8);

pub(super) fn investigator_at() -> Vec3 {
    layout::investigator_card(0)
}

pub(super) fn injury_at() -> Vec3 {
    layout::investigator_threat(0, 0)
}

pub(super) fn deck_at() -> Vec3 {
    layout::investigator_deck(0)
}

pub(super) fn token_at() -> Vec3 {
    layout::test_token(false, 0)
}

pub(super) fn card_transform(index: usize, at: Vec3, exhausted: bool) -> Transform {
    use layout::ScenarioBookSide;
    let side = match CARDS[index].code {
        "01105" => Some(ScenarioBookSide::Agenda),
        "01108" => Some(ScenarioBookSide::Act),
        _ => None,
    };
    if let Some(side) = side {
        return layout::scenario_book_card_transform(side, Vec2::new(CARD_WIDTH, CARD_LENGTH));
    }
    let rotation = if CARDS[index].landscape || exhausted {
        std::f32::consts::FRAC_PI_2
    } else {
        0.0
    };
    Transform::from_translation(at).with_rotation(Quat::from_rotation_y(rotation))
}
pub(super) fn ghoul_at(view: &Observation) -> Vec3 {
    if view.ghoul.engaged {
        layout::investigator_threat(0, 1)
    } else {
        Vec3::new(0.7, 0.05, -3.8)
    }
}
type SceneSnapshot = (Observation, std::collections::BTreeSet<u8>);

#[allow(clippy::too_many_arguments)]
pub(super) fn synchronize_scene(
    mut commands: Commands,
    table: Res<Table>,
    art: Res<Art>,
    entities: Query<Entity, With<Scene>>,
    token: Query<&Transform, With<ChaosToken>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rendered: Local<Option<SceneSnapshot>>,
) {
    let view = table.view();
    let discard_at = layout::investigator_discard(0);
    let bag_at = layout::test_bag();
    let key = (view.clone(), table.context.commitments.clone());
    if rendered.as_ref() == Some(&key) {
        return;
    }
    let token_scale = if table.reduced_motion {
        Vec3::ONE
    } else if rendered
        .as_ref()
        .is_some_and(|(last, _)| last.held_token == view.held_token)
    {
        token.single().map_or(Vec3::ONE, |t| t.scale)
    } else {
        Vec3::splat(0.1)
    };
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    let mut fixed = vec![
        (0, investigator_at(), String::new(), false),
        (
            1,
            STUDY_AT,
            format!("Study\n{} clues · Shroud 2", view.study_clues),
            false,
        ),
        (
            3,
            layout::investigator_asset(0, 0),
            format!("Bounty Contracts\n{} bounties", view.contracts_bounties),
            false,
        ),
        (
            7,
            Vec3::new(6.0, 0.05, -6.7),
            "Scenario reference".into(),
            false,
        ),
        (
            8,
            card_transform(8, Vec3::ZERO, false).translation,
            format!("Act · Trapped\n{} / 6 clues held", view.clues),
            false,
        ),
        (
            9,
            card_transform(9, Vec3::ZERO, false).translation,
            "Agenda · What's Going On?!".into(),
            false,
        ),
    ];
    if view.ghoul.alive {
        fixed.push((
            2,
            ghoul_at(&view),
            format!(
                "Ghoul Minion\nDamage {}/2 · Bounty {}\n{}·{}",
                view.ghoul.damage,
                view.ghoul.bounties,
                if view.ghoul.ready {
                    "Ready"
                } else {
                    "Exhausted"
                },
                if view.ghoul.engaged {
                    "Engaged"
                } else {
                    "At Study"
                }
            ),
            !view.ghoul.ready,
        ));
    }
    if view.injury {
        fixed.push((4, injury_at(), "Internal Injury".into(), false));
    }
    for (index, at, caption, exhausted) in fixed {
        let source = Source::Card(index);
        card_mesh(
            &mut commands,
            &mut meshes,
            &mut materials,
            &art,
            index,
            at,
            exhausted,
            (view.revision, source),
        );
        let caption_at = if index == 2 && view.ghoul.engaged {
            // This prepared position has two threats. Keep the enemy's state
            // beside its card within the band, clear of the asset region below.
            at + Vec3::new(2.8, 0.1, -0.7)
        } else if matches!(index, 8 | 9) {
            // Keep the screen-space caption outside the entire page. Raising its
            // world anchor alone still projects a downward-growing label over art.
            at + Vec3::new(
                if index == 9 { -1.0 } else { 1.0 } * (CARD_LENGTH * 0.5 + 0.20),
                0.25,
                0.0,
            )
        } else {
            at + Vec3::new(0.0, 0.1, 1.1)
        };
        if caption.is_empty() {
            continue;
        }
        let (width, font_size) = match index {
            2 => (130.0, 10.5),
            4 => (90.0, 10.5),
            0 | 3 => (120.0, 12.0),
            _ => (160.0, 13.0),
        };
        let label = world_label_sized(
            &mut commands,
            &art,
            caption,
            caption_at,
            PAPER,
            width,
            font_size,
        );
        if matches!(index, 8 | 9) {
            // Anchor the two captions outward so they cannot cover each other.
            commands
                .entity(label)
                .insert(OuterLabel { left: index == 9 });
        }
    }
    let encounter_discard = view.encounter_discard();
    for (index, card) in encounter_discard.iter().enumerate() {
        let entity = discard::spawn_card(
            &mut commands,
            &mut meshes,
            &mut materials,
            card,
            index,
            discard::Appearance {
                size: Vec2::new(CARD_WIDTH, CARD_LENGTH),
                thickness: CARD_THICKNESS,
                image: art.image(&format!("card/{}", card.code)),
                fallback: Color::srgb(0.34, 0.31, 0.23),
            },
        );
        let revision = view.revision;
        commands
            .entity(entity)
            .insert((Scene, CardSource(Source::EncounterDiscard)))
            .observe(move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
                if event.button == bevy::picking::pointer::PointerButton::Primary {
                    queue.0.push(Action::Select(
                        revision,
                        Source::EncounterDiscard,
                        discard::ENCOUNTER_DISCARD_AT,
                    ));
                }
            });
    }
    world_label(
        &mut commands,
        &art,
        discard::caption(encounter_discard),
        discard::ENCOUNTER_DISCARD_AT + Vec3::new(3.4, 0.1, 0.45),
        MUTED,
    );
    for (index, investigator) in view.investigators.iter().enumerate() {
        let code = ["06003", "06005", "60401"][index];
        let slot = view.investigators[..index]
            .iter()
            .filter(|other| other.location == investigator.location)
            .count();
        let at = layout::location_standee_position(
            STUDY_AT.with_y(0.0),
            Vec2::new(CARD_WIDTH, CARD_LENGTH),
            slot,
        )
        .expect("the prepared Study has at most four investigators");
        let image = art.image(&format!("standee/{code}"));
        let standee = crate::tabletop_standees::spawn(
            &mut commands,
            &mut meshes,
            &mut materials,
            image.as_ref(),
            at,
            layout::investigator_class(index),
            Vec2::new(CARD_WIDTH, CARD_LENGTH),
        );
        commands.entity(standee).insert(Scene);
        let left = slot < 2;
        let label_at = at
            + Vec3::new(
                if left { -0.25 } else { 0.25 },
                crate::tabletop_standees::height_for(Vec2::new(CARD_WIDTH, CARD_LENGTH)) + 0.15,
                0.0,
            );
        let label = world_label(
            &mut commands,
            &art,
            format!(
                "{}\n{}",
                investigator
                    .name
                    .split_whitespace()
                    .next()
                    .unwrap_or(&investigator.name),
                investigator.location,
            ),
            label_at,
            MUTED,
        );
        commands
            .entity(label)
            .insert((WorldLabel(label_at, 108.0), OuterLabel { left }));
        commands
            .entity(label)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.width = px(108);
            });
        area_heading(
            &mut commands,
            &art,
            investigator.name.clone(),
            layout::investigator_heading(index),
        );
        for (label, at) in layout::zone_labels(index) {
            world_label_sized(&mut commands, &art, label.into(), at, MUTED, 60.0, 11.0);
        }
        if index > 0 {
            // The fixture supplies identity/location/horror, but no personal piles or playable turn.
            if let Some(image) = image {
                commands.spawn((
                    Mesh3d(meshes.add(Plane3d::default().mesh().size(1.1, 1.5))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color_texture: Some(image),
                        unlit: true,
                        ..default()
                    })),
                    Transform::from_translation(layout::investigator_card(index)),
                    Scene,
                    Pickable::IGNORE,
                ));
            }
            world_label_sized(
                &mut commands,
                &art,
                "Study · companion\nPersonal cards unrepresented".into(),
                layout::investigator_status(index),
                MUTED,
                110.0,
                11.0,
            );
        }
    }
    for (text, at) in [
        (
            "Encounter deck\nReserved · not modeled",
            Vec3::new(4.0, 0.1, -3.7),
        ),
        (
            "Victory display\nReserved · not modeled",
            Vec3::new(7.0, 0.1, -1.7),
        ),
    ] {
        world_label(&mut commands, &art, text.into(), at, MUTED);
    }
    area_heading(
        &mut commands,
        &art,
        "Scenario".into(),
        Vec3::new(0.0, 0.1, -10.2),
    );
    area_heading(
        &mut commands,
        &art,
        "Shared test area".into(),
        layout::TEST_CENTER + Vec3::new(0.0, 0.1, -5.5),
    );
    for (label, at) in layout::test_zone_labels() {
        world_label(&mut commands, &art, label.into(), at, MUTED);
    }
    world_label_sized(
        &mut commands,
        &art,
        format!("Resources {}\nClues {}", view.resources, view.clues),
        layout::investigator_status(0),
        MUTED,
        100.0,
        11.0,
    );
    for (index, card) in view.hand.iter().enumerate() {
        let at = layout::hand_position(0, view.hand.len(), index);
        if let Some(face) = CARDS.iter().position(|c| c.code == card.code) {
            let source = Source::Hand(card.id);
            card_mesh(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                face,
                at,
                false,
                (view.revision, source),
            );
            world_label(
                &mut commands,
                &art,
                format!(
                    "{} · {}{}",
                    card.title,
                    card.id + 1,
                    if table.context.commitments.contains(&card.id) {
                        " · selected"
                    } else {
                        ""
                    }
                ),
                at + Vec3::new(0.0, 0.1, 1.02),
                PAPER,
            );
        }
    }
    for (index, card) in view.committed.iter().enumerate() {
        let at = layout::test_commitment(view.committed.len(), index);
        if let Some(face) = CARDS.iter().position(|c| c.code == card.code) {
            card_mesh(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                face,
                at,
                false,
                (view.revision, Source::Committed(card.id)),
            );
            world_label(
                &mut commands,
                &art,
                format!("Tony committed\n{} · {}", card.title, card.id + 1),
                at + Vec3::new(0.0, 0.1, 1.05),
                GOLD,
            );
        }
    }
    deck_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        &art,
        view.deck_count,
        view.revision,
    );
    world_label_sized(
        &mut commands,
        &art,
        format!("{} cards", view.deck_count),
        deck_at() + Vec3::new(0.0, 0.2, 1.05),
        MUTED,
        64.0,
        12.0,
    );
    // All discard identities remain in their recorded order; the top opens its collection browser.
    for (index, card) in view.discard.iter().enumerate() {
        if let Some(face) = CARDS.iter().position(|c| c.code == card.code) {
            card_mesh(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                face,
                discard_at + Vec3::Y * index as f32 * CARD_THICKNESS,
                false,
                (view.revision, Source::Discard),
            );
        }
    }
    world_label_sized(
        &mut commands,
        &art,
        format!("{} cards", view.discard.len()),
        discard_at + Vec3::new(0.0, 0.2, 1.05),
        MUTED,
        64.0,
        12.0,
    );
    let revision = view.revision;
    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.45, 0.7))),
            MeshMaterial3d(materials.add(Color::srgb(0.18, 0.10, 0.16))),
            Transform::from_translation(bag_at),
            Scene,
        ))
        .observe(move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue.0.push(Action::Select(revision, Source::Bag, bag_at));
            }
        });
    world_label(
        &mut commands,
        &art,
        format!("Chaos bag\n{} tokens", view.bag_count),
        bag_at + Vec3::new(0.0, 0.6, 0.8),
        GOLD,
    );
    if let Some(face) = view.held_token {
        let status = if view
            .test
            .as_ref()
            .is_some_and(|test| test.success.is_some())
        {
            crate::TokenStatus::Resolved
        } else {
            crate::TokenStatus::Pending
        };
        let (disk, label) = crate::tabletop_tokens::spawn(
            &mut commands,
            &mut meshes,
            &mut materials,
            crate::tabletop_tokens::TokenVisual {
                face,
                status,
                ordinal: None,
                image: art.image(&format!("token/{}", token_role(face))),
                transform: Transform::from_translation(token_at()).with_scale(token_scale),
                label_offset: Vec3::new(0.0, 0.0, 1.05),
            },
        );
        commands
            .entity(disk)
            .insert((Appearing, ChaosToken, Scene))
            .remove::<Pickable>()
            .observe(move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
                if event.button == bevy::picking::pointer::PointerButton::Primary {
                    queue
                        .0
                        .push(Action::Select(revision, Source::Token, token_at()));
                }
            });
        commands.entity(label).insert(Scene);
    }
    harm_scene(
        &mut commands,
        &mut meshes,
        &mut materials,
        &art,
        &view,
        table.harm(),
    );
    *rendered = Some(key);
}
