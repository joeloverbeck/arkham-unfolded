//! Physical locations for the existing prepared position, not additional game state.
use super::*;

pub(super) const INVESTIGATOR_AT: Vec3 = Vec3::new(-15.8, 0.05, 10.1);
pub(super) const INJURY_AT: Vec3 = Vec3::new(-13.1, 0.05, 7.2);
pub(super) const DECK_AT: Vec3 = Vec3::new(-8.0, 0.05, 9.5);
pub(super) const TOKEN_AT: Vec3 = Vec3::new(2.1, 0.2, 0.8);
const DISCARD_AT: Vec3 = Vec3::new(-8.0, 0.05, 12.0);
const BAG_AT: Vec3 = Vec3::new(5.5, 0.35, 0.0);
const STUDY_AT: Vec3 = Vec3::new(-2.5, 0.05, -3.8);

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
        Vec3::new(-10.4, 0.05, 7.2)
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
        (
            0,
            INVESTIGATOR_AT,
            format!("Tony\nDamage {}/9 · Horror {}/5", view.damage, view.horror),
            false,
        ),
        (
            1,
            STUDY_AT,
            format!("Study\n{} clues · Shroud 2", view.study_clues),
            false,
        ),
        (
            3,
            Vec3::new(-12.4, 0.05, 10.1),
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
                "Ghoul Minion\nDamage {}/2\n{} / {}\nBounty {}",
                view.ghoul.damage,
                if view.ghoul.ready {
                    "Ready"
                } else {
                    "Exhausted"
                },
                if view.ghoul.engaged {
                    "Engaged"
                } else {
                    "At Study"
                },
                view.ghoul.bounties
            ),
            !view.ghoul.ready,
        ));
    } else {
        world_label(
            &mut commands,
            &art,
            "Ghoul defeated".into(),
            ghoul_at(&view),
            MUTED,
        );
    }
    if view.injury {
        fixed.push((
            4,
            INJURY_AT,
            "Internal Injury\nTony's threat area".into(),
            false,
        ));
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
        let caption_at = if matches!(index, 8 | 9) {
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
        let label = world_label(&mut commands, &art, caption, caption_at, PAPER);
        if matches!(index, 8 | 9) {
            // Anchor the two captions outward so they cannot cover each other.
            commands
                .entity(label)
                .insert(OuterLabel { left: index == 9 });
        }
    }
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
        );
        commands.entity(standee).insert(Scene);
        let left = slot < 2;
        let label_at = at
            + Vec3::new(
                if left { -0.25 } else { 0.25 },
                crate::tabletop_standees::HEIGHT + 0.15,
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
        let center = layout::investigator_center(index, 3);
        area_heading(
            &mut commands,
            &art,
            investigator.name.clone(),
            center + Vec3::new(0.0, 0.05, -4.3),
        );
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
                    Transform::from_translation(center + Vec3::new(-3.5, 0.05, 0.0)),
                    Scene,
                    Pickable::IGNORE,
                ));
            }
            world_label(
                &mut commands,
                &art,
                format!(
                    "Study · Horror {}/{}\nCompanion · no turn here\nPersonal cards not modeled",
                    investigator.horror, investigator.sanity
                ),
                center,
                MUTED,
            );
        }
    }
    for (text, at) in [
        ("Tony's threats", Vec3::new(-15.8, 0.1, 6.0)),
        ("Hand", Vec3::new(-10.4, 0.1, 11.65)),
        // Reserve the space above Tony for harm amounts, counters and totals.
        ("Assets in play", Vec3::new(-15.8, 0.1, 6.9)),
        (
            "Encounter deck\nReserved · not modeled",
            Vec3::new(4.0, 0.1, -3.7),
        ),
        (
            "Encounter discard\nReserved · not modeled",
            Vec3::new(7.0, 0.1, -3.7),
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
        layout::TEST_CENTER + Vec3::new(-3.5, 0.1, -1.5),
    );
    for (index, card) in view.hand.iter().enumerate() {
        let at = Vec3::new(
            -15.5 + (index % 4) as f32 * 1.8,
            0.05,
            13.0 + (index / 4) as f32 * 2.4,
        );
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
        let at = layout::TEST_CENTER + Vec3::new(-3.3 + index as f32 * 1.8, 0.05, -0.2);
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
    world_label(
        &mut commands,
        &art,
        format!("Tony's deck\n{} cards", view.deck_count),
        DECK_AT + Vec3::new(0.0, 0.2, 1.05),
        MUTED,
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
                DISCARD_AT + Vec3::Y * index as f32 * CARD_THICKNESS,
                false,
                (view.revision, Source::Discard),
            );
        }
    }
    world_label(
        &mut commands,
        &art,
        format!("Tony's discard\n{} cards", view.discard.len()),
        DISCARD_AT + Vec3::new(0.0, 0.2, 1.05),
        MUTED,
    );
    let revision = view.revision;
    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.45, 0.7))),
            MeshMaterial3d(materials.add(Color::srgb(0.18, 0.10, 0.16))),
            Transform::from_translation(BAG_AT),
            Scene,
        ))
        .observe(move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue.0.push(Action::Select(revision, Source::Bag, BAG_AT));
            }
        });
    world_label(
        &mut commands,
        &art,
        format!("Chaos bag\n{} tokens", view.bag_count),
        BAG_AT + Vec3::new(0.0, 0.6, 0.8),
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
                transform: Transform::from_translation(TOKEN_AT).with_scale(token_scale),
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
                        .push(Action::Select(revision, Source::Token, TOKEN_AT));
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
