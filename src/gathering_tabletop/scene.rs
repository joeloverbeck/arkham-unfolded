//! Gathering's public observations rendered with the settled Party pieces.
use super::*;
use crate::tabletop_pieces as pieces;

pub(super) fn label(
    commands: &mut Commands,
    context: &CardRender<'_>,
    value: impl Into<String>,
    at: Vec3,
    heading: bool,
) -> Entity {
    let entity = pieces::label(commands, value, at, heading, context.art.symbol_font());
    commands.entity(entity).insert(ScenePiece);
    entity
}

pub(super) fn bind_source(
    commands: &mut Commands,
    context: &CardRender<'_>,
    entity: Entity,
    selection: controls::Selection,
) {
    context.bind(commands, entity, selection.source.clone());
    commands.entity(entity).remove::<Pickable>().observe(
        move |event: On<PointerClick>,
              targets: Query<&ContextTarget>,
              mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary
                && let Ok(target) = targets.get(entity)
            {
                queue.0.push(Action::Select(
                    target.epoch,
                    target.revision,
                    selection.clone(),
                ));
            }
        },
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn deck(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    context: &CardRender<'_>,
    at: Vec3,
    count: usize,
    source: controls::Source,
    encounter: bool,
) {
    let image = context.art.image(if encounter {
        "card-back/encounter"
    } else {
        "card-back/player"
    });
    let deck = pieces::deck(commands, meshes, materials, image, count, at);
    let deck_source = match source {
        controls::Source::Deck(who) => crate::gathering::DrawSource::Investigator(who),
        _ => crate::gathering::DrawSource::Encounter,
    };
    commands
        .entity(deck.body)
        .insert((ScenePiece, shuffle::DeckSurface(deck_source)));
    commands
        .entity(deck.top)
        .insert((ScenePiece, shuffle::DeckSurface(deck_source)));
    feedback::attach_card_feedback(
        commands,
        meshes,
        materials,
        deck.top,
        pieces::CARD_WIDTH,
        pieces::CARD_LENGTH,
        0.0,
    );
    bind_source(
        commands,
        context,
        deck.top,
        controls::Selection {
            source,
            at,
            title: if encounter {
                "Encounter deck"
            } else {
                "Player deck"
            }
            .into(),
            inspect_code: if encounter {
                "encounter-deck"
            } else {
                "hidden-player-deck"
            }
            .into(),
        },
    );
    label(
        commands,
        context,
        format!("{count} cards"),
        at + Vec3::Z * 1.1 - Vec3::X * if encounter { 0.12 } else { 0. },
        false,
    );
}

pub(super) fn render(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    context: &CardRender<'_>,
) {
    let view = context.view;
    for (index, investigator) in view.investigators.iter().enumerate() {
        let who = investigator.investigator;
        let code = match who {
            Investigator::Tony => "06003",
            Investigator::Patrice => "06005",
            Investigator::Jacqueline => "60401",
        };
        card(
            commands,
            meshes,
            materials,
            context,
            CardSpec {
                source: Some(controls::Source::Investigator(who)),
                ..CardSpec::new(layout::investigator_card(index), who.name(), code)
            },
        );
        let player = view.play.as_ref().map(|play| &play.players[index]);
        let projection = player.map(|player| &player.presentation);
        if let Some(spent) = projection.and_then(|status| status.round_ability_spent) {
            let class = layout::investigator_class(index);
            let token = crate::tabletop_counters::spawn_ability_marker(
                commands,
                meshes,
                materials,
                crate::tabletop_counters::ActionRow {
                    investigator: index,
                    turn_ended: false,
                    additional: None,
                    additional_eligible: false,
                    class,
                    icon: context.art.image("token/investigator-ability"),
                    background: context
                        .art
                        .image(&format!("class/{}/background", class.asset_name())),
                },
                spent,
            );
            commands.entity(token).insert(ScenePiece);
        }
        let panel = pieces::InvestigatorCounters {
            index,
            resources: player.map_or(investigator.resources, |player| player.resources),
            clues: context
                .consequences
                .shown_clues(index, player.map_or(0, |player| player.clues)),
            damage: player.map_or(investigator.damage, |player| player.damage),
            health: projection.map_or_else(|| investigator.health(), |status| status.health),
            horror: player.map_or(investigator.horror, |player| player.horror),
            sanity: projection.map_or_else(|| investigator.sanity(), |status| status.sanity),
            actions: projection.map(|status| pieces::ActionState {
                allowance: status.ordinary_action_allowance,
                spent: status.ordinary_actions_spent,
                turn_ended: status.turn_finished,
                additional: status.tony_bonus.as_ref().map(|bonus| !bonus.used),
                additional_eligible: status
                    .tony_bonus
                    .as_ref()
                    .is_some_and(|bonus| bonus.available && bonus.eligible),
            }),
        };
        let counters = pieces::investigator_counters(commands, meshes, materials, panel, |role| {
            context.art.image(role)
        });
        for entity in counters {
            commands.entity(entity).insert(ScenePiece);
        }
        for (slot, held) in investigator.hand.iter().enumerate() {
            // Upkeep ordering keeps cards in their rules zone until the whole
            // discard resolves, but their one physical face is in the drawer.
            if view
                .play
                .as_ref()
                .and_then(|play| play.pending.discard.as_ref())
                .is_some_and(|discard| {
                    discard
                        .ordered
                        .iter()
                        .chain(&discard.remaining)
                        .any(|card| card.id == held.id)
                })
            {
                continue;
            }
            card(
                commands,
                meshes,
                materials,
                context,
                CardSpec {
                    physical_id: Some(held.id),
                    source: Some(controls::Source::Card(held.id)),
                    selected: investigator.selected_mulligan.contains(&held.id)
                        || view
                            .play
                            .as_ref()
                            .and_then(|play| play.test.as_ref())
                            .is_some_and(|test| test.selected.contains(&held.id)),
                    ..CardSpec::new(
                        layout::hand_position(index, investigator.hand.len(), slot),
                        &held.title,
                        &held.code,
                    )
                },
            );
        }
        if view.play.is_none() {
            for (slot, permanent) in investigator.permanents.iter().enumerate() {
                card(
                    commands,
                    meshes,
                    materials,
                    context,
                    CardSpec {
                        physical_id: Some(permanent.id),
                        source: Some(controls::Source::Card(permanent.id)),
                        ..CardSpec::new(
                            layout::investigator_asset(index, slot),
                            &permanent.title,
                            &permanent.code,
                        )
                    },
                );
            }
            deck(
                commands,
                meshes,
                materials,
                context,
                layout::investigator_deck(index),
                investigator.deck_count,
                controls::Source::Deck(who),
                false,
            );
            label(
                commands,
                context,
                "0 cards",
                layout::investigator_discard(index) + Vec3::Z * 1.1,
                false,
            );
        }
    }
    if view.play.is_some() {
        play::render(commands, meshes, materials, context);
    } else {
        let scenario = view.scenario_for_display();
        for (side, codes, title, source) in [
            (
                layout::ScenarioBookSide::Agenda,
                &scenario.agenda,
                "Agenda 1",
                controls::Source::Agenda,
            ),
            (
                layout::ScenarioBookSide::Act,
                &scenario.act,
                "Act 1",
                controls::Source::Act,
            ),
        ] {
            if let Some(code) = codes.first() {
                let pose = layout::scenario_book_card_transform(
                    side,
                    Vec2::new(pieces::CARD_WIDTH, pieces::CARD_LENGTH),
                );
                card(
                    commands,
                    meshes,
                    materials,
                    context,
                    CardSpec {
                        pose,
                        source: Some(source),
                        ..CardSpec::new(pose.translation, title, code)
                    },
                );
            }
        }
        card(
            commands,
            meshes,
            materials,
            context,
            CardSpec {
                source: Some(controls::Source::ScenarioReference),
                ..CardSpec::new(
                    layout::SCENARIO_REFERENCE,
                    "Standard reference",
                    &scenario.reference,
                )
            },
        );
        let map = crate::tabletop_gathering_map::GatheringMap::new(
            scenario
                .locations
                .iter()
                .map(|code| (code.as_str(), code == "01111")),
            Vec2::new(pieces::CARD_WIDTH, pieces::CARD_LENGTH),
        )
        .expect("The selected Gathering locations fit their shared scenario surface");
        for entity in map.spawn(commands, meshes, materials) {
            commands.entity(entity).insert(ScenePiece);
        }
        for code in &scenario.locations {
            let at = map.position(code);
            card(
                commands,
                meshes,
                materials,
                context,
                CardSpec {
                    source: Some(controls::Source::Location(code.clone())),
                    ..CardSpec::new(at, "Study", code)
                },
            );
        }
        deck(
            commands,
            meshes,
            materials,
            context,
            layout::ENCOUNTER_DECK,
            scenario.encounter_deck_count,
            controls::Source::EncounterDeck,
            true,
        );
        play::render_setup_pieces(
            commands,
            meshes,
            materials,
            context,
            &map,
            scenario.study_clues,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tabletop_counters::{ActionMarker, Counter, CounterKind};
    use bevy::camera::primitives::MeshAabb;

    #[test]
    fn production_setup_uses_party_card_counter_caption_and_counted_deck_geometry() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = super::super::tests::host(directory.path().join("scene.json"));
        let view = app.world().resource::<Table>().view();
        let world = app.world_mut();
        let pieces = world
            .query::<(Entity, &ContextTarget, Option<&Name>, &Transform)>()
            .iter(world)
            .map(|(entity, target, name, transform)| {
                (
                    entity,
                    target.source.clone(),
                    name.map(|name| name.as_str().to_owned()),
                    *transform,
                )
            })
            .collect::<Vec<_>>();
        for (index, investigator) in view.investigators.iter().enumerate() {
            let (entity, _, _, transform) = pieces
                .iter()
                .find(|(_, source, name, _)| {
                    *source == controls::Source::Investigator(investigator.investigator)
                        && name.as_deref() == Some(investigator.investigator.name())
                })
                .unwrap();
            assert_eq!(transform.translation, layout::investigator_card(index));
            let bounds = world
                .resource::<Assets<Mesh>>()
                .get(&world.get::<Mesh3d>(*entity).unwrap().0)
                .unwrap()
                .get_aabb()
                .unwrap();
            assert!((bounds.half_extents.x * 2. - pieces::CARD_WIDTH).abs() < 0.0001);
            assert!((bounds.half_extents.z * 2. - pieces::CARD_LENGTH).abs() < 0.0001);
            assert!((transform.rotation * Vec3::X - Vec3::NEG_Z).length() < 0.0001);
            let (_, _, _, deck) = pieces
                .iter()
                .find(|(_, source, _, _)| {
                    *source == controls::Source::Deck(investigator.investigator)
                })
                .unwrap();
            let expected_y = layout::investigator_deck(index).y
                + investigator.deck_count.max(1) as f32 * 0.0065
                + 0.002;
            assert!((deck.translation.y - expected_y).abs() < 0.0001);
            let resource = world
                .query::<(&Counter, &Transform)>()
                .iter(world)
                .find(|(counter, transform)| {
                    counter.kind == CounterKind::Resource
                        && transform.translation == layout::investigator_resource(index)
                })
                .map(|(counter, _)| counter.amount);
            assert_eq!(
                resource,
                (investigator.resources > 0).then_some(investigator.resources)
            );
            for (horror, capacity) in [
                (false, investigator.health()),
                (true, investigator.sanity()),
            ] {
                let expected = format!(
                    "{}/{capacity}",
                    if horror {
                        investigator.horror
                    } else {
                        investigator.damage
                    }
                );
                assert!(
                    world
                        .query::<(&WorldLabel, &Text)>()
                        .iter(world)
                        .any(|(anchor, text)| text.0 == expected
                            && anchor.0 == layout::investigator_harm_caption(index, horror))
                );
            }
        }
        let labels = world
            .query::<(&WorldLabel, &Text, &Node, Option<&BackgroundColor>)>()
            .iter(world)
            .collect::<Vec<_>>();
        assert!(
            !labels
                .iter()
                .any(|(_, text, _, _)| text.0 == "Scenario" || text.0 == "Test area")
        );
        for source in [
            controls::Source::Location("01111".into()),
            controls::Source::Agenda,
            controls::Source::Act,
            controls::Source::EncounterDeck,
            controls::Source::ScenarioReference,
            controls::Source::Bag,
        ] {
            assert!(
                pieces.iter().any(|(_, target, _, _)| *target == source),
                "{source:?}"
            );
        }
        // The public preview must not move setup or reveal a shuffled order.
        assert!(view.scenario.is_none());
        assert!(view.play.is_none());
        assert_eq!(view.stage, crate::gathering::Stage::ChooseLead);
        assert!(
            !labels
                .iter()
                .any(|(_, text, _, _)| text.0.contains(" · copy "))
        );
        assert!(
            labels
                .iter()
                .filter(|(_, _, node, _)| node.width == px(190.))
                .all(|(_, _, _, background)| background.is_none_or(|color| color.0.alpha() == 0.))
        );
        assert_eq!(
            world
                .query::<&Name>()
                .iter(world)
                .filter(|name| name.as_str() == "Investigator standee")
                .count(),
            3
        );
        assert!(
            world
                .query::<&Counter>()
                .iter(world)
                .any(|counter| counter.kind == CounterKind::Clue && counter.amount == 6)
        );
    }

    #[test]
    fn production_play_projects_all_ordinary_actions_and_the_unspent_restricted_action() {
        let directory = tempfile::tempdir().unwrap();
        let mut session = Session::with_seed(19);
        for id in [
            "lead:tony",
            "confirm-mulligan",
            "confirm-mulligan",
            "confirm-mulligan",
        ] {
            session.choose(session.observe().revision, id).unwrap();
        }
        let mut app = super::super::tests::host(directory.path().join("actions.json"));
        app.insert_resource(Table::new(session, directory.path().join("actions.json")));
        app.update();
        let world = app.world_mut();
        let markers = world
            .query::<&ActionMarker>()
            .iter(world)
            .collect::<Vec<_>>();
        assert_eq!(
            markers.iter().filter(|marker| !marker.additional).count(),
            9
        );
        assert_eq!(markers.iter().filter(|marker| marker.additional).count(), 1);
        assert!(markers.iter().all(|marker| !marker.spent));
        assert!(
            markers
                .iter()
                .find(|marker| marker.additional)
                .unwrap()
                .subdued
        );
    }

    #[test]
    fn arcane_research_labels_each_physical_copy_in_setup_and_play() {
        let directory = tempfile::tempdir().unwrap();
        for complete in [false, true] {
            let mut session = Session::with_seed(19);
            session
                .choose(session.observe().revision, "lead:tony")
                .unwrap();
            if complete {
                for _ in 0..3 {
                    session
                        .choose(session.observe().revision, "confirm-mulligan")
                        .unwrap();
                }
            }
            let path = directory.path().join(format!("research-{complete}.json"));
            let mut app = super::super::tests::host(path.clone());
            app.insert_resource(Table::new(session, path));
            app.update();
            let world = app.world_mut();
            let captions = world
                .query::<(&WorldLabel, &Text)>()
                .iter(world)
                .filter_map(|(_, text)| {
                    text.0
                        .starts_with("Arcane Research")
                        .then_some(text.0.clone())
                })
                .collect::<Vec<_>>();
            assert_eq!(captions, ["Arcane Research", "Arcane Research"]);
            let targets = world
                .query::<(&Name, &motion::PhysicalCard, &ContextTarget)>()
                .iter(world)
                .filter(|(name, _, _)| name.as_str() == "Arcane Research")
                .collect::<Vec<_>>();
            assert_eq!(targets.len(), 2);
            assert_ne!(targets[0].1.0, targets[1].1.0);
            assert!(
                targets.iter().all(
                    |(_, physical, target)| target.source == controls::Source::Card(physical.0)
                )
            );
        }
    }
}
