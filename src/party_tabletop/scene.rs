use super::*;
use crate::{
    TokenFace, TokenStatus,
    tabletop_counters::{self as counters, ActionRow, CounterKind, CounterVisual},
    tabletop_tokens::{self, TokenVisual},
};

pub(super) fn discard_at(who: Investigator) -> Vec3 {
    layout::investigator_discard(who.index())
}
pub(super) fn label(
    commands: &mut Commands,
    art: &Art,
    text: impl Into<String>,
    at: Vec3,
    heading: bool,
) -> Entity {
    commands
        .spawn((
            art.text(text, if heading { 18. } else { 13. }, PAPER),
            TextLayout::justify(Justify::Center),
            Node {
                position_type: PositionType::Absolute,
                width: px(190.),
                ..default()
            },
            Pickable::IGNORE,
            WorldLabel(at, heading),
            Scene,
        ))
        .id()
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
    let mut view = table.view();
    for (index, investigator) in view.investigators.iter_mut().enumerate() {
        investigator.clues = table.displayed_clues(index);
    }
    if last.as_ref() == Some(&view) {
        return;
    }
    let new_tokens = last.as_ref().map_or(0, |old| old.held_tokens().len());
    let returning = !table.reduced_motion
        && view.test.as_ref().is_some_and(|test| test.result.is_some())
        && last
            .as_ref()
            .is_some_and(|old| old.revision < view.revision && !old.held_tokens().is_empty());
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    let revision = view.revision;
    label(
        &mut commands,
        &art,
        "Scenario",
        layout::scenario_heading(),
        true,
    );
    let map = location_map::GatheringMap::new(&view.locations, Vec2::new(CARD_WIDTH, CARD_LENGTH))
        .expect("the prepared Gathering map must fit its scenario surface");
    map.spawn(&mut commands, &mut meshes, &mut materials);
    for location in &view.locations {
        let at = map.position(&location.code);
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
                if location_map::GatheringMap::entry_blocked(location) {
                    "Entry blocked".into()
                } else if location.revealed {
                    String::new()
                } else {
                    "Unrevealed".into()
                }
            ),
            at + Vec3::Z * 1.1,
            false,
        );
        if location.revealed
            && let Some((token, caption)) = counters::spawn_count(
                &mut commands,
                &mut meshes,
                &mut materials,
                CounterVisual {
                    kind: CounterKind::Clue,
                    amount: location.clues,
                    image: art.image("token/clue"),
                    at: layout::location_clue(at),
                    scale: 1.0,
                },
            )
        {
            commands.entity(token).insert(Scene);
            commands.entity(caption).insert(Scene);
        }
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
                Vec2::new(CARD_WIDTH, CARD_LENGTH),
            );
            commands.entity(entity).insert(Scene);
        }
    }
    for (role, side) in [
        ("card/01105", layout::ScenarioBookSide::Agenda),
        (
            if view.stage == crate::party::Stage::ActReveal {
                "card/01108/reverse"
            } else if view.endpoint == Some(crate::party::Endpoint::Escaped) {
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
        } else if role == "card/01108" || role == "card/01108/reverse" {
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
    let reference = layout::SCENARIO_REFERENCE;
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
        let code = ["06003", "06005", "60401"][index];
        label(
            &mut commands,
            &art,
            who.name(),
            layout::investigator_heading(index),
            true,
        );
        for (name, at) in layout::zone_labels(index) {
            label(&mut commands, &art, name, at, false);
        }
        let at = layout::investigator_card(index);
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
        for (kind, amount, role, at) in [
            (
                CounterKind::Resource,
                investigator.resources,
                "token/resource",
                layout::investigator_resource(index),
            ),
            (
                CounterKind::Clue,
                investigator.clues,
                "token/clue",
                layout::investigator_clue(index),
            ),
        ] {
            if let Some((token, caption)) = counters::spawn_count(
                &mut commands,
                &mut meshes,
                &mut materials,
                CounterVisual {
                    kind,
                    amount,
                    image: art.image(role),
                    at,
                    scale: 0.85,
                },
            ) {
                commands.entity(token).insert(Scene);
                commands.entity(caption).insert(Scene);
            }
        }
        let class = layout::investigator_class(index);
        let class_name = class.asset_name();
        for entity in counters::spawn_actions(
            &mut commands,
            &mut meshes,
            &mut materials,
            ActionRow {
                investigator: index,
                remaining: investigator.ordinary_actions,
                turn_ended: investigator.turn_finished,
                additional: (who == Investigator::Tony).then_some(view.tony_bonus_action),
                additional_eligible: false,
                class,
                icon: art.image(&format!("class/{class_name}/icon")),
                background: art.image(&format!("class/{class_name}/background")),
            },
        ) {
            commands.entity(entity).insert(Scene);
        }
        for (amount, capacity, horror, role, title) in [
            (
                investigator.damage,
                investigator.health,
                false,
                "token/damage",
                "Damage",
            ),
            (
                investigator.horror,
                investigator.sanity,
                true,
                "token/horror",
                "Horror",
            ),
        ] {
            let at = layout::investigator_harm(index, horror);
            let image = art.image(role);
            let caption = layout::investigator_harm_caption(index, horror, image.is_none());
            let text = if image.is_some() {
                format!("{amount}/{capacity}")
            } else {
                format!("{title} {amount}/{capacity}")
            };
            let entity = crate::tabletop_harm_tokens::spawn(
                &mut commands,
                &mut meshes,
                &mut materials,
                image,
                at,
                horror,
            );
            commands.entity(entity).insert(Scene);
            commands.spawn((
                Text::new(text),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(PAPER),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(80.0),
                    ..default()
                },
                WorldLabel(caption, false),
                Pickable::IGNORE,
                Scene,
            ));
        }
        if who == Investigator::Tony {
            face(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                "card/06010",
                layout::investigator_asset(index, 0),
                None,
                revision,
            );
            label(
                &mut commands,
                &art,
                format!("Bounty Contracts\n{} bounties", view.contracts_bounties),
                layout::investigator_asset(index, 0) + Vec3::Z * 1.1,
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
                layout::investigator_asset(index, 0) + Vec3::Z * 1.1,
                false,
            );
            for copy in 0..view.jacqueline_arcane_research {
                face(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &art,
                    "card/04109",
                    layout::investigator_asset(index, usize::from(copy)),
                    None,
                    revision,
                );
            }
        }
        let deck_at = layout::investigator_deck(index);
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
            format!("{} cards", investigator.deck_count),
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
                "{} cards{}",
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
            let at = layout::hand_position(index, investigator.hand.len(), slot);
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
    let test_heading = label(
        &mut commands,
        &art,
        "Test area",
        layout::test_heading(),
        true,
    );
    commands
        .entity(test_heading)
        .insert(layout::TestAreaHeading);
    for (name, at) in layout::test_zone_labels() {
        label(&mut commands, &art, name, at, false);
    }
    if let Some(test) = &view.test {
        for (slot, committed) in test.committed.iter().enumerate() {
            let at = layout::test_commitment(test.committed.len(), slot);
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
    let mut token_slots = [0, 0];
    let visible_tokens = if returning {
        &view.tokens
    } else {
        view.held_tokens()
    };
    for (index, token) in visible_tokens.iter().enumerate() {
        let canceled = token.status == TokenStatus::Canceled;
        let section = usize::from(canceled);
        let at = layout::test_token(canceled, token_slots[section]);
        token_slots[section] += 1;
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
        if returning {
            commands.entity(entity).insert((
                Scene,
                Pickable::IGNORE,
                ReturningToken {
                    start: at,
                    destination: if token.face == TokenFace::Curse
                        && token.status == TokenStatus::Resolved
                    {
                        curse_pool_at()
                    } else {
                        layout::test_bag()
                    },
                    elapsed: 0.,
                },
            ));
            commands.entity(label).despawn();
            continue;
        }
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
    let bag_at = layout::test_bag();
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
    let pool_at = curse_pool_at();
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.6, 0.12))),
        MeshMaterial3d(materials.add(Color::srgb(0.25, 0.16, 0.32))),
        Transform::from_translation(pool_at),
        Pickable::IGNORE,
        Scene,
    ));
    label(
        &mut commands,
        &art,
        format!("Curse pool\n{} tokens", view.curses_in_pool),
        pool_at + Vec3::Z * 0.9,
        false,
    );
    label(
        &mut commands,
        &art,
        "Encounter discard: empty\nNo enemies or encounter draw in this attempt",
        layout::ENCOUNTER_DISCARD + Vec3::Z * 2.0,
        false,
    );
    *last = Some(view);
}
pub(super) fn curse_pool_at() -> Vec3 {
    layout::test_bag() - Vec3::X * 2.
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
