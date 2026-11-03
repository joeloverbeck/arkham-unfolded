use super::*;
use crate::tabletop_counters::{self as counters, CounterKind, CounterVisual};
use crate::tabletop_pieces as pieces;

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
    let entity = pieces::label(commands, text, at, heading, &art.symbol_font);
    commands.entity(entity).insert(Scene);
    entity
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
    let at = transform.translation;
    let entity = pieces::card(
        commands,
        meshes,
        materials,
        art.image(CARDS[index].role),
        transform,
        CARDS[index].title,
    );
    commands
        .entity(entity)
        .insert((Scene, CardSource(source)))
        .observe(move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue.0.push(Action::Select(revision, source, at));
            }
        });
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
    let deck = pieces::deck(
        commands,
        meshes,
        materials,
        art.image("card-back/player"),
        count,
        at,
    );
    commands.entity(deck.body).insert(Scene);
    commands.entity(deck.top).insert(Scene).observe(
        move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue.0.push(Action::Select(revision, source, at));
            }
        },
    );
}
#[allow(clippy::too_many_arguments)]
pub(super) fn synchronize(
    mut commands: Commands,
    table: Res<Table>,
    art: Res<Art>,
    entities: Query<Entity, With<Scene>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut consequences: consequences::Presentation,
) {
    let Some(presentation) =
        consequences.prepare_view(table.view(), table.replay.is_none(), table.reduced_motion)
    else {
        return;
    };
    let view = &presentation.observation;
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
    let map = location_map::GatheringMap::new(
        view.locations
            .iter()
            .map(|location| (location.code.as_str(), location.revealed)),
        Vec2::new(CARD_WIDTH, CARD_LENGTH),
    )
    .expect("the prepared Gathering map must fit its scenario surface");
    for entity in map.spawn(&mut commands, &mut meshes, &mut materials) {
        commands.entity(entity).insert(Scene);
    }
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
                if location_map::GatheringMap::entry_blocked(&location.code, location.revealed) {
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
        let counters = pieces::investigator_counters(
            &mut commands,
            &mut meshes,
            &mut materials,
            pieces::InvestigatorCounters {
                index,
                resources: investigator.resources,
                clues: investigator.clues,
                damage: investigator.damage,
                health: investigator.health,
                horror: investigator.horror,
                sanity: investigator.sanity,
                actions: Some(pieces::ActionState {
                    remaining: investigator.ordinary_actions,
                    allowance: 3,
                    spent: 3_u8.saturating_sub(investigator.ordinary_actions),
                    turn_ended: investigator.turn_finished,
                    additional: (who == Investigator::Tony).then_some(view.tony_bonus_action),
                    additional_eligible: false,
                }),
            },
            |role| art.image(role),
        );
        for entity in counters.entities {
            commands.entity(entity).insert(Scene);
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
            let caption = label(
                &mut commands,
                &art,
                held.title.clone(),
                at + Vec3::Z * 1.0,
                false,
            );
            commands.entity(caption).insert(pieces::CardCaption);
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
    presentation.spawn_tokens(&mut commands, &mut meshes, &mut materials, &art);
    let bag_at = layout::test_bag();
    let chaos = pieces::chaos_area(
        &mut commands,
        &mut meshes,
        &mut materials,
        view.bag_count,
        Some(view.curses_in_pool),
        &art.symbol_font,
    );
    for entity in chaos.entities {
        commands.entity(entity).insert(Scene);
    }
    commands.entity(chaos.bag).observe(
        move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue.0.push(Action::Select(revision, Source::Bag, bag_at));
            }
        },
    );
    label(
        &mut commands,
        &art,
        "Encounter discard: empty\nNo enemies or encounter draw in this attempt",
        layout::ENCOUNTER_DISCARD + Vec3::Z * 2.0,
        false,
    );
}
