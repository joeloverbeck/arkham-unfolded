use super::*;
use crate::{
    TokenFace,
    tabletop_tokens::{self, TokenVisual},
};

pub(super) fn location_at(name: &str) -> Vec3 {
    match name {
        "Attic" => Vec3::new(-4., 0.05, -3.4),
        "Cellar" => Vec3::new(4., 0.05, -3.4),
        "Parlor" => Vec3::new(0., 0.05, -5.7),
        _ => Vec3::new(0., 0.05, -3.4),
    }
}
pub(super) fn discard_at(who: Investigator) -> Vec3 {
    layout::investigator_center(who.index(), 3) + Vec3::new(3.9, 0.05, 1.1)
}
pub(super) fn label(
    commands: &mut Commands,
    art: &Art,
    text: impl Into<String>,
    at: Vec3,
    heading: bool,
) {
    commands.spawn((
        art.text(text, if heading { 18. } else { 13. }, PAPER),
        Node {
            position_type: PositionType::Absolute,
            width: px(190.),
            ..default()
        },
        Pickable::IGNORE,
        WorldLabel(at, heading),
        Scene,
    ));
}
#[allow(clippy::too_many_arguments)]
fn card(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Art,
    index: usize,
    transform: Transform,
    source: Source,
    revision: u64,
) {
    let image = art.image(CARDS[index].role);
    let at = transform.translation;
    let entity = commands
        .spawn((
            Name::new(CARDS[index].title),
            Mesh3d(meshes.add(Plane3d::default().mesh().size(CARD_WIDTH, CARD_LENGTH))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: if image.is_some() {
                    Color::WHITE
                } else {
                    Color::srgb(0.34, 0.31, 0.23)
                },
                base_color_texture: image,
                unlit: true,
                ..default()
            })),
            transform,
            Scene,
            CardSource(source),
        ))
        .observe(move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue.0.push(Action::Select(revision, source, at));
            }
        })
        .id();
    feedback::attach_card_feedback(
        commands,
        meshes,
        materials,
        entity,
        CARD_WIDTH,
        CARD_LENGTH,
        0.,
    );
}
#[allow(clippy::too_many_arguments)]
fn face(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Art,
    role: &str,
    at: Vec3,
    source: Option<Source>,
    revision: u64,
) {
    if let Some(index) = cards::index(role) {
        let rotation = if CARDS[index].landscape {
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
        } else {
            Quat::IDENTITY
        };
        card(
            commands,
            meshes,
            materials,
            art,
            index,
            Transform::from_translation(at).with_rotation(rotation),
            source.unwrap_or(Source::Card(index)),
            revision,
        );
    }
}
#[allow(clippy::too_many_arguments)]
fn deck(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Art,
    count: usize,
    at: Vec3,
    source: Source,
    revision: u64,
) {
    let height = (count.max(1) as f32) * 0.0065;
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(CARD_WIDTH, height, CARD_LENGTH))),
        MeshMaterial3d(materials.add(PAPER)),
        Transform::from_translation(at + Vec3::Y * height / 2.),
        Scene,
        Pickable::IGNORE,
    ));
    let image = art.image("card-back/player");
    commands
        .spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(CARD_WIDTH, CARD_LENGTH))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: if image.is_some() {
                    Color::WHITE
                } else {
                    Color::srgb(0.12, 0.19, 0.30)
                },
                base_color_texture: image,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(at + Vec3::Y * (height + 0.002)),
            Scene,
        ))
        .observe(move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue.0.push(Action::Select(revision, source, at));
            }
        });
}
#[allow(clippy::too_many_arguments)]
pub(super) fn synchronize(
    mut commands: Commands,
    table: Res<Table>,
    art: Res<Art>,
    entities: Query<Entity, With<Scene>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut last: Local<Option<Observation>>,
) {
    let view = table.view();
    if last.as_ref() == Some(&view) {
        return;
    }
    let new_tokens = last.as_ref().map_or(0, |old| old.tokens.len());
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    let revision = view.revision;
    for location in &view.locations {
        let at = location_at(&location.name);
        let role = format!("card/{}", location.code);
        face(
            &mut commands,
            &mut meshes,
            &mut materials,
            &art,
            &role,
            at,
            None,
            revision,
        );
        label(
            &mut commands,
            &art,
            format!(
                "{}\n{}",
                location.name,
                if location.revealed {
                    format!("{} clues", location.clues)
                } else {
                    "Unrevealed".into()
                }
            ),
            at + Vec3::Z * 1.1,
            false,
        );
        for (slot, investigator) in view
            .investigators
            .iter()
            .filter(|i| i.location == location.name)
            .enumerate()
        {
            let index = investigator.investigator.index();
            let code = ["06003", "06005", "60401"][index];
            let point = layout::location_standee_position(
                at.with_y(0.),
                Vec2::new(CARD_WIDTH, CARD_LENGTH),
                slot,
            )
            .expect("three investigators fit");
            let image = art.image(&format!("standee/{code}"));
            let entity = crate::tabletop_standees::spawn(
                &mut commands,
                &mut meshes,
                &mut materials,
                image.as_ref(),
                point,
                layout::investigator_class(index),
            );
            commands.entity(entity).insert(Scene);
        }
    }
    for (role, side) in [
        ("card/01105", layout::ScenarioBookSide::Agenda),
        (
            if view.endpoint == Some(crate::party::Endpoint::Escaped) {
                "card/01109"
            } else {
                "card/01108"
            },
            layout::ScenarioBookSide::Act,
        ),
    ] {
        let index = cards::index(role).unwrap();
        let transform =
            layout::scenario_book_card_transform(side, Vec2::new(CARD_WIDTH, CARD_LENGTH));
        card(
            &mut commands,
            &mut meshes,
            &mut materials,
            &art,
            index,
            transform,
            Source::Card(index),
            revision,
        );
        let (kind, side) = if role == "card/01105" {
            ("Agenda 1", -1.)
        } else if role == "card/01108" {
            ("Act 1", 1.)
        } else {
            ("Act 2", 1.)
        };
        label(
            &mut commands,
            &art,
            format!("{kind}\n{}", CARDS[index].title),
            transform.translation + Vec3::new(side * 2.25, 0.25, 0.),
            false,
        );
    }
    let reference = Vec3::new(6., 0.05, -6.5);
    face(
        &mut commands,
        &mut meshes,
        &mut materials,
        &art,
        "card/01104",
        reference,
        None,
        revision,
    );
    label(
        &mut commands,
        &art,
        "Standard reference",
        reference + Vec3::Z * 1.1,
        false,
    );
    for investigator in &view.investigators {
        let who = investigator.investigator;
        let index = who.index();
        let center = layout::investigator_center(index, 3);
        let code = ["06003", "06005", "60401"][index];
        label(
            &mut commands,
            &art,
            who.name(),
            center + Vec3::new(0., 0.2, -4.3),
            true,
        );
        let at = center + Vec3::new(-3.5, 0.05, -0.8);
        face(
            &mut commands,
            &mut meshes,
            &mut materials,
            &art,
            &format!("card/{code}"),
            at,
            None,
            revision,
        );
        label(
            &mut commands,
            &art,
            format!(
                "{} resources · {} clues\nDamage {}/{} · Horror {}/{}\n{} actions · {}",
                investigator.resources,
                investigator.clues,
                investigator.damage,
                investigator.health,
                investigator.horror,
                investigator.sanity,
                investigator.ordinary_actions,
                if investigator.turn_finished {
                    "Turn ended"
                } else if view.actor == Some(who) {
                    "Acting"
                } else {
                    "Waiting"
                }
            ),
            at + Vec3::Z * 1.1,
            false,
        );
        if who == Investigator::Tony {
            face(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                "card/06010",
                center + Vec3::new(-0.7, 0.05, -0.8),
                None,
                revision,
            );
            label(
                &mut commands,
                &art,
                format!(
                    "Bounty Contracts\n{} bounties\nBonus action: no eligible enemy",
                    view.contracts_bounties
                ),
                center + Vec3::new(-0.7, 0.1, 0.3),
                false,
            );
        }
        if who == Investigator::Jacqueline {
            label(
                &mut commands,
                &art,
                format!(
                    "Arcane Research ×{}\nJacqueline's reaction: {}",
                    view.jacqueline_arcane_research,
                    if view.jacqueline_used {
                        "Used this round"
                    } else {
                        "Available this round"
                    }
                ),
                center + Vec3::new(0.1, 0.1, 0.3),
                false,
            );
            for copy in 0..view.jacqueline_arcane_research {
                face(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &art,
                    "card/04109",
                    center + Vec3::new(-0.7 + f32::from(copy) * 1.55, 0.05, -0.8),
                    None,
                    revision,
                );
            }
        }
        let deck_at = center + Vec3::new(3.9, 0.05, -1.8);
        deck(
            &mut commands,
            &mut meshes,
            &mut materials,
            &art,
            investigator.deck_count,
            deck_at,
            Source::Deck(who),
            revision,
        );
        label(
            &mut commands,
            &art,
            format!("Deck · {} cards", investigator.deck_count),
            deck_at + Vec3::Z * 1.1,
            false,
        );
        let discard_at = discard_at(who);
        if let Some(top) = investigator.discard.last() {
            face(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                &format!("card/{}", top.code),
                discard_at,
                Some(Source::Discard(who)),
                revision,
            );
        }
        label(
            &mut commands,
            &art,
            format!(
                "Discard · {} cards{}",
                investigator.discard.len(),
                investigator
                    .discard
                    .last()
                    .map_or(String::new(), |card| format!("\n{}", card.title))
            ),
            discard_at + Vec3::Z * 1.1,
            false,
        );
        for (slot, held) in investigator.hand.iter().enumerate() {
            let at = center
                + Vec3::new(
                    -3.8 + (slot % 5) as f32 * 1.65,
                    0.05,
                    2.4 + (slot / 5) as f32 * 2.6,
                );
            face(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                &format!("card/{}", held.code),
                at,
                Some(Source::Hand(held.id)),
                revision,
            );
            label(
                &mut commands,
                &art,
                held.title.clone(),
                at + Vec3::Z * 1.0,
                false,
            );
        }
    }
    label(
        &mut commands,
        &art,
        "Test area",
        layout::TEST_CENTER + Vec3::new(0., 0., -1.8),
        true,
    );
    if let Some(test) = &view.test {
        for (slot, committed) in test.committed.iter().enumerate() {
            let at = layout::TEST_CENTER + Vec3::new(-3.7 + slot as f32 * 1.55, 0.05, 0.);
            face(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                &format!("card/{}", committed.code),
                at,
                Some(Source::Committed(committed.id)),
                revision,
            );
            label(
                &mut commands,
                &art,
                format!(
                    "{}\n{}'s commitment",
                    committed.title,
                    committed.owner.name()
                ),
                at + Vec3::Z * 1.05,
                false,
            );
        }
    }
    for (index, token) in view.tokens.iter().enumerate() {
        let at = layout::TEST_CENTER + Vec3::new(-2. + index as f32 * 1.2, 0.15, 2.1);
        let transform = Transform::from_translation(at).with_scale(
            if index >= new_tokens && !table.reduced_motion {
                Vec3::splat(0.1)
            } else {
                Vec3::ONE
            },
        );
        let (entity, label) = tabletop_tokens::spawn(
            &mut commands,
            &mut meshes,
            &mut materials,
            TokenVisual {
                face: token.face,
                status: token.status,
                ordinal: Some(usize::from(token.id) + 1),
                image: art.image(&format!("token/{}", token_role(token.face))),
                transform,
                label_offset: Vec3::new(0., 0., 0.55),
            },
        );
        let source = Source::Token(token.id);
        commands
            .entity(entity)
            .insert((Scene, Appearing, Pickable::default(), CardSource(source)))
            .observe(move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
                if event.button == bevy::picking::pointer::PointerButton::Primary {
                    queue.0.push(Action::Select(revision, source, at));
                }
            });
        feedback::attach_card_feedback(
            &mut commands,
            &mut meshes,
            &mut materials,
            entity,
            1.,
            1.,
            0.08,
        );
        commands.entity(label).insert(Scene);
    }
    let bag_at = Vec3::new(6.2, 0.3, 0.5);
    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.75, 0.55))),
            MeshMaterial3d(materials.add(Color::srgb(0.22, 0.14, 0.12))),
            Transform::from_translation(bag_at),
            Scene,
        ))
        .observe(move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue.0.push(Action::Select(revision, Source::Bag, bag_at));
            }
        });
    label(
        &mut commands,
        &art,
        format!("Chaos bag\n{} tokens", view.bag_count),
        bag_at + Vec3::Z * 0.9,
        false,
    );
    label(
        &mut commands,
        &art,
        "Encounter discard: empty\nNo enemies or encounter draw in this attempt",
        Vec3::new(-6., 0.1, -6.),
        false,
    );
    *last = Some(view);
}
fn token_role(face: TokenFace) -> &'static str {
    match face {
        TokenFace::PlusOne => "plus_one",
        TokenFace::Zero => "zero",
        TokenFace::MinusOne => "minus_one",
        TokenFace::MinusTwo => "minus_two",
        TokenFace::MinusThree => "minus_three",
        TokenFace::MinusFour => "minus_four",
        TokenFace::Skull => "skull",
        TokenFace::Cultist => "cultist",
        TokenFace::Tablet => "tablet",
        TokenFace::AutoFail => "auto_fail",
        TokenFace::ElderSign => "elder_sign",
        TokenFace::Curse => "curse",
    }
}
