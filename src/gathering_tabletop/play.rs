//! Presentation of the shared runtime observation. No legality or hidden state lives here.

use super::controls::Source;
use super::*;
use crate::gathering::{CardView, Phase, PlayObservation};

pub(super) fn location_name(code: &str) -> &str {
    match code {
        "01111" => "Study",
        "01112" => "Hallway",
        "01113" => "Attic",
        "01114" => "Cellar",
        "01115" => "Parlor",
        _ => code,
    }
}

fn investigator_code(investigator: Investigator) -> &'static str {
    match investigator {
        Investigator::Tony => "06003",
        Investigator::Patrice => "06005",
        Investigator::Jacqueline => "60401",
    }
}

fn pile_text(title: &str, cards: &[CardView]) -> String {
    let mut lines = vec![format!("{title} · {} cards", cards.len())];
    for card in cards {
        lines.push(format!("\n{}", card.title));
        if let Some(rules) = crate::gathering::inspection::card(&card.code)
            .or_else(|| crate::gathering::inspection::scenario_card(&card.code, true))
        {
            lines.push(rules);
        }
    }
    lines.join("\n")
}

/// Only faces and piles represented by this player-safe observation are inspectable.
pub(super) fn inspect(view: &Observation, code: &str) -> Option<String> {
    let play = view.play.as_ref()?;
    if code == "chaos-bag" {
        let mut counts = std::collections::BTreeMap::new();
        for face in &view.standard_bag {
            *counts.entry(face.label()).or_insert(0usize) += 1;
        }
        return Some(format!(
            "Chaos bag · {} tokens currently in the bag\n\nInitial Standard composition:\n{}\n\nRevealed and sealed tokens are outside the bag. The draw order is hidden.",
            play.bag_count,
            counts
                .into_iter()
                .map(|(face, count)| format!("{count} × {face}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if code == "resolving-cards" {
        return Some(pile_text(
            "Cards awaiting resolution",
            &play.pending.resolving_cards,
        ));
    }
    if code == "pending-discard" {
        let discard = play.pending.discard.as_ref()?;
        return Some(format!(
            "{}\n\n{}",
            pile_text("Selected discard order", &discard.ordered),
            pile_text("Still to order", &discard.remaining)
        ));
    }
    if code == "searched-player" {
        return Some(pile_text(
            "Backpack search · revealed cards",
            &play.searched_player,
        ));
    }
    if code == "searched-encounter" {
        return Some(pile_text(
            "Encounter search · revealed cards",
            &play.searched_encounter,
        ));
    }
    if let Some(id) = code
        .strip_prefix("backpack:")
        .and_then(|id| id.parse::<u16>().ok())
    {
        let attached = play
            .backpacks
            .iter()
            .find(|attached| attached.asset == id)?;
        return Some(format!(
            "{}\n\n{}\nThese cards are attached face down, not in hand or in play. Only the controller may inspect them.",
            crate::gathering::inspection::card("04037")?,
            pile_text(
                &format!("{} · Backpack", attached.owner.name()),
                &attached.cards
            )
        ));
    }
    for player in &play.players {
        let key = player.investigator.id();
        if code == format!("discard:{key}") {
            return Some(pile_text(
                &format!("{} discard", player.investigator.name()),
                &player.discard,
            ));
        }
        if code == format!("removed:{key}") {
            return Some(pile_text(
                &format!("{} removed cards", player.investigator.name()),
                &player.removed,
            ));
        }
    }
    for (key, title, cards) in [
        (
            "encounter-discard",
            "Encounter discard",
            &play.encounter_discard,
        ),
        (
            "scenario-removed",
            "Scenario removed cards",
            &play.scenario.removed,
        ),
    ] {
        if code == key {
            return Some(pile_text(title, cards));
        }
    }
    if code == "victory" {
        let cards = play.scenario.victory_display();
        let points: u8 = cards.iter().map(|card| card.points).sum();
        let mut text = format!("Victory display · {} cards · {points} VP", cards.len());
        for card in cards {
            text.push_str(&format!(
                "\n\n{}\n{}",
                card.title,
                crate::gathering::inspection::scenario_card(card.code, true).unwrap_or_default()
            ));
        }
        return Some(text);
    }
    if code == "encounter-deck" {
        let inventory = view
            .scenario
            .as_ref()
            .map(|setup| {
                setup
                    .encounter_inventory
                    .iter()
                    .map(|card| format!("{} × {} · {}", card.quantity, card.title, card.code))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        return Some(format!(
            "Encounter deck · {} cards remaining\n\nInitial public inventory:\n{inventory}\n\nShuffled order is hidden.",
            play.encounter_deck_count
        ));
    }
    if let Some(room) = play
        .scenario
        .locations
        .iter()
        .find(|room| room.code == code)
    {
        return crate::gathering::inspection::scenario_card(code, room.revealed);
    }
    let agenda = format!("{:05}", 1104 + usize::from(play.agenda));
    let act = format!("{:05}", 1107 + usize::from(play.act));
    let scenario_visible = code == "01104"
        || code == agenda
        || code == act
        || (code == "01117" && play.scenario.lita_in_play)
        || play
            .scenario
            .locations
            .iter()
            .any(|room| room.fog.as_ref().is_some_and(|card| card.code == code))
        || play
            .scenario
            .enemies
            .iter()
            .any(|enemy| enemy.card.code == code)
        || play
            .encounter_discard
            .iter()
            .chain(&play.scenario.victory)
            .chain(&play.scenario.removed)
            .any(|card| card.code == code)
        || play.searched_encounter.iter().any(|card| card.code == code)
        || play
            .pending_revelation
            .as_ref()
            .is_some_and(|pending| pending.card.code == code);
    let player_visible = play
        .pending
        .resolving_cards
        .iter()
        .any(|card| card.code == code)
        || play.pending.discard.as_ref().is_some_and(|discard| {
            discard
                .remaining
                .iter()
                .chain(&discard.ordered)
                .any(|card| card.code == code)
        })
        || play.searched_player.iter().any(|card| card.code == code)
        || play
            .backpacks
            .iter()
            .flat_map(|attached| &attached.cards)
            .any(|card| card.code == code)
        || play
            .premonition
            .as_ref()
            .is_some_and(|sealed| sealed.card.code == code)
        || play.players.iter().any(|player| {
            code == investigator_code(player.investigator)
                || player
                    .hand
                    .iter()
                    .chain(&player.discard)
                    .chain(&player.threats)
                    .chain(&player.removed)
                    .any(|card| card.code == code)
                || player.assets.iter().any(|asset| asset.card.code == code)
        })
        || play.test.as_ref().is_some_and(|test| {
            test.source_card
                .as_ref()
                .is_some_and(|card| card.code == code)
                || test.committed.iter().any(|entry| entry.card.code == code)
        });
    (scenario_visible || player_visible)
        .then(|| {
            crate::gathering::inspection::card(code)
                .or_else(|| crate::gathering::inspection::scenario_card(code, true))
        })
        .flatten()
}

/// An inspection target retains the physical enemy identity privately so two
/// identical cards can show their different public counters and readiness.
pub(super) fn enemy_inspection(view: &Observation, key: &str) -> Option<(String, String)> {
    let id = key.strip_prefix("enemy:")?.parse::<u16>().ok()?;
    let enemy = view
        .play
        .as_ref()?
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == id)?;
    let engagement = enemy.engaged.map_or_else(
        || "Unengaged".into(),
        |who| format!("Engaged with {}", who.name()),
    );
    Some((
        enemy.card.code.clone(),
        format!(
            "{} · Damage {} · Doom {} · Bounties {}\n{engagement} · {}",
            if enemy.exhausted {
                "Exhausted"
            } else {
                "Ready"
            },
            enemy.damage,
            enemy.doom,
            enemy.bounties,
            location_name(&enemy.location),
        ),
    ))
}

pub(super) fn focus(view: &Observation) -> Area {
    if let Some(play) = &view.play {
        if play.scenario.outcome.is_some() {
            return Area::Scenario;
        }
        if play.test.is_some() {
            return Area::Test;
        }
    }
    view.decision_owner
        .or_else(|| view.play.as_ref().and_then(|play| play.actor))
        .map_or(Area::Scenario, |owner| Area::Investigator(owner.index()))
}

pub(super) fn framing(view: &Observation) -> (Vec3, f32) {
    view.play
        .as_ref()
        .and_then(trays::Layout::for_play)
        .filter(|tray| {
            tray.has_content() || view.play.as_ref().is_none_or(|play| play.test.is_none())
        })
        .map_or_else(|| layout::focus(focus(view)), |tray| tray.focus())
}

pub(super) fn status(view: &Observation) -> Option<String> {
    let play = view.play.as_ref()?;
    let phase = match play.phase {
        Phase::Investigation => "Investigation",
        Phase::Enemy => "Enemy phase",
        Phase::Upkeep => "Upkeep",
        Phase::Mythos => "Mythos",
        Phase::RoundEnd => "Round end",
    };
    let actor = play.actor.map_or("No active turn", Investigator::name);
    let owner = view
        .decision_owner
        .map_or("No pending player decision", Investigator::name);
    let mut lines = vec![format!(
        "Round {} · {phase} · Turn: {actor} · Decision: {owner}",
        play.round
    )];
    if let Some(pending) = &play.pending_revelation {
        lines.push(format!(
            "{} drew {}{}",
            pending.drawn_by.name(),
            pending.card.title,
            if pending.cancelled {
                " · Cancelled"
            } else {
                ""
            }
        ));
    }
    if let Some(harm) = &play.pending.harm {
        lines.push(format!(
            "Pending {}{} damage and {} horror on {} · {}",
            if harm.direct { "direct " } else { "" },
            harm.damage,
            harm.horror,
            harm.investigator.name(),
            harm.source
        ));
        for allocation in &harm.allocations {
            let target = allocation.asset.as_ref().map_or_else(
                || harm.investigator.name().to_string(),
                |asset| asset.title.clone(),
            );
            lines.push(format!(
                "Assigned to {target}: {} damage, {} horror (not yet applied)",
                allocation.damage, allocation.horror
            ));
        }
    }
    if let Some(discard) = &play.pending.discard {
        let selected = discard
            .ordered
            .iter()
            .map(|card| card.title.clone())
            .collect::<Vec<_>>()
            .join(" → ");
        lines.push(format!(
            "{} orders discards · {} remain\nSelected order: {}",
            discard.owner.name(),
            discard.remaining.len(),
            if selected.is_empty() {
                "None"
            } else {
                &selected
            }
        ));
    }
    if let Some(outcome) = &play.scenario.outcome {
        use crate::gathering::Resolution;
        lines.push(
            match outcome.resolution {
                Resolution::NoResolution => "No resolution reached.",
                Resolution::BurnHouse => "Resolution 1 · The house burns down.",
                Resolution::RefuseToBurn => "Resolution 2 · The house remains standing.",
                Resolution::Overrun => "Resolution 3 · The house is overrun.",
            }
            .into(),
        );
        for (index, player) in play.players.iter().enumerate() {
            lines.push(format!(
                "{} · Experience +{} · Physical trauma +{} · Mental trauma +{}{}",
                player.investigator.name(),
                outcome.experience_gained[index],
                outcome.physical_trauma_gained[index],
                outcome.mental_trauma_gained[index],
                if outcome.killed.contains(&player.investigator) {
                    " · Killed"
                } else {
                    ""
                }
            ));
        }
        if outcome.lita_recipient_pending {
            lines.push(
                "Lita's recipient awaits the campaign's successor-investigator decision.".into(),
            );
        }
    }
    Some(lines.join("\n"))
}

#[derive(Clone, Copy)]
enum ShownTokenState {
    Revealed,
    Selected,
    Ignored,
    Cancelled,
    Resolved,
}

impl ShownTokenState {
    fn label(self) -> &'static str {
        match self {
            Self::Revealed => "Revealed",
            Self::Selected => "Selected",
            Self::Ignored => "Ignored",
            Self::Cancelled => "Cancelled",
            Self::Resolved => "Resolved",
        }
    }
}

struct Painter<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    context: &'a CardRender<'a>,
}

impl Painter<'_, '_, '_> {
    fn chaos_area(&mut self, bag_count: usize, curse_pool: Option<u8>) {
        let chaos = crate::tabletop_pieces::chaos_area(
            self.commands,
            self.meshes,
            self.materials,
            bag_count,
            curse_pool,
            self.context.art.symbol_font(),
        );
        for entity in chaos.entities {
            self.commands.entity(entity).insert(ScenePiece);
        }
        self.selectable(
            chaos.bag,
            controls::Selection {
                source: Source::Bag,
                at: layout::test_bag(),
                title: "Chaos bag".into(),
                inspect_code: "chaos-bag".into(),
            },
        );
    }

    fn standee(&mut self, at: Vec3, slot: usize, index: usize, who: Investigator) {
        let size = Vec2::new(1.35, 1.89);
        let Some(position) = layout::location_standee_position(at, size, slot) else {
            return;
        };
        let entity = crate::tabletop_standees::spawn(
            self.commands,
            self.meshes,
            self.materials,
            self.context
                .art
                .image(&format!("standee/{}", investigator_code(who)))
                .as_ref(),
            position,
            layout::investigator_class(index),
            size,
        );
        self.commands.entity(entity).insert(ScenePiece);
        // Mesh hits bubble to the investigator's menu observer.
        self.commands.queue(move |world: &mut World| {
            let children = world
                .get::<Children>(entity)
                .map(|children| children.to_vec())
                .unwrap_or_default();
            for child in children {
                world.entity_mut(child).remove::<Pickable>();
            }
        });
        self.selectable(
            entity,
            controls::Selection {
                source: Source::Investigator(who),
                at: position,
                title: who.name().into(),
                inspect_code: investigator_code(who).into(),
            },
        );
    }

    fn card(&mut self, spec: CardSpec<'_>) -> Entity {
        card(
            self.commands,
            self.meshes,
            self.materials,
            self.context,
            spec,
        )
    }

    fn selectable(&mut self, entity: Entity, selection: controls::Selection) {
        super::scene::bind_source(self.commands, self.context, entity, selection);
    }

    fn count(
        &mut self,
        at: Vec3,
        kind: crate::tabletop_counters::CounterKind,
        amount: u8,
        source: Source,
    ) {
        use crate::tabletop_counters::{CounterKind, CounterVisual, spawn_count};
        let (role, title) = match kind {
            CounterKind::Clue => ("token/clue", "Clues"),
            CounterKind::Resource => ("token/resource", "Resources"),
        };
        if let Some((token, label)) = spawn_count(
            self.commands,
            self.meshes,
            self.materials,
            CounterVisual {
                kind,
                amount,
                image: self.context.art.image(role),
                at,
                scale: 1.0,
            },
        ) {
            self.commands.entity(token).insert(ScenePiece);
            self.commands.entity(label).insert(ScenePiece);
            let inspect_code = match &source {
                Source::Investigator(who) => investigator_code(*who).into(),
                Source::Location(code) => code.clone(),
                _ => String::new(),
            };
            self.selectable(
                token,
                controls::Selection {
                    source,
                    at,
                    title: format!("{title} · {amount}"),
                    inspect_code,
                },
            );
        }
    }

    fn caption(&mut self, at: Vec3, value: String) {
        super::scene::label(
            self.commands,
            self.context,
            value,
            at + Vec3::Z * 1.1,
            false,
        );
    }

    fn token(
        &mut self,
        at: Vec3,
        face: crate::TokenFace,
        state: ShownTokenState,
        ordinal: usize,
        active: bool,
    ) {
        let status = match state {
            ShownTokenState::Cancelled => crate::TokenStatus::Canceled,
            ShownTokenState::Resolved => crate::TokenStatus::Resolved,
            _ => crate::TokenStatus::Pending,
        };
        let (disk, label) = crate::tabletop_tokens::spawn(
            self.commands,
            self.meshes,
            self.materials,
            crate::tabletop_tokens::TokenVisual {
                face,
                status,
                ordinal: Some(ordinal),
                image: self
                    .context
                    .art
                    .image(crate::tabletop_tokens::image_role(face)),
                transform: Transform::from_translation(at),
                label_offset: Vec3::Z * 0.55,
            },
        );
        self.commands.entity(disk).insert(ScenePiece);
        let title = format!("Token {ordinal} · {}\n{}", face.label(), state.label());
        if active {
            self.selectable(
                disk,
                controls::Selection {
                    source: Source::Token(ordinal - 1),
                    at,
                    title: title.clone(),
                    inspect_code: "chaos-bag".into(),
                },
            );
        }
        if matches!(state, ShownTokenState::Ignored) {
            self.commands
                .entity(disk)
                .insert(MeshMaterial3d(self.materials.add(StandardMaterial {
                    base_color: MUTED,
                    metallic: 0.32,
                    perceptual_roughness: 0.5,
                    ..default()
                })));
        }
        self.commands
            .entity(label)
            .insert((ScenePiece, Text::new(crate::tabletop::readable_text(title))));
    }
}

pub(super) fn render_setup_pieces(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    context: &CardRender<'_>,
    map: &crate::tabletop_gathering_map::GatheringMap,
    study_clues: u8,
) {
    let mut paint = Painter {
        commands,
        meshes,
        materials,
        context,
    };
    paint.chaos_area(context.view.standard_bag.len(), Some(10));
    let study = map.position("01111");
    paint.count(
        layout::location_clue(study),
        crate::tabletop_counters::CounterKind::Clue,
        study_clues,
        Source::Location("01111".into()),
    );
    for (index, investigator) in context.view.investigators.iter().enumerate() {
        paint.standee(study, index, index, investigator.investigator);
    }
    paint.caption(layout::ENCOUNTER_DISCARD, "0 cards".into());
    paint.caption(layout::VICTORY_DISPLAY, "0 cards\n0 VP".into());
}

pub(super) fn render(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    context: &CardRender<'_>,
) {
    let view = context.view;
    let Some(play) = &view.play else { return };
    let mut paint = Painter {
        commands,
        meshes,
        materials,
        context,
    };
    for (index, player) in play.players.iter().enumerate() {
        for (slot, asset) in player.assets.iter().enumerate() {
            let attached = play
                .backpacks
                .iter()
                .find(|attached| attached.asset == asset.card.id);
            let inspection = attached.map_or_else(
                || asset.card.code.clone(),
                |_| format!("backpack:{}", asset.card.id),
            );
            let mut details = Vec::new();
            if asset.card.code == "06010" {
                details.push(format!("{} bounties", asset.uses));
            } else if asset.uses > 0 {
                details.push(format!("Uses {}", asset.uses));
            }
            if asset.damage > 0 {
                details.push(format!("Damage {}", asset.damage));
            }
            if asset.horror > 0 {
                details.push(format!("Horror {}", asset.horror));
            }
            if let Some(attached) = attached.filter(|attached| !attached.cards.is_empty()) {
                details.push(format!("{} attached", attached.cards.len()));
            }
            let status = details.join(" · ");
            let caption = if status.is_empty() {
                Caption::Text(&asset.card.title)
            } else {
                Caption::AssetStatus {
                    title: &asset.card.title,
                    status: &status,
                }
            };
            let pose = Transform::from_translation(layout::investigator_asset(index, slot))
                .with_rotation(if asset.ready {
                    Quat::IDENTITY
                } else {
                    Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)
                });
            paint.card(CardSpec {
                source: Some(attached.map_or(Source::Card(asset.card.id), |_| {
                    Source::Backpack(asset.card.id)
                })),
                physical_id: Some(asset.card.id),
                pose,
                caption: Some(caption),
                ..CardSpec::new(pose.translation, &asset.card.title, &inspection)
            });
        }
        for (slot, threat) in player.threats.iter().enumerate() {
            paint.card(CardSpec {
                source: Some(Source::Card(threat.id)),
                physical_id: Some(threat.id),
                ..CardSpec::new(
                    layout::investigator_threat(index, slot),
                    &threat.title,
                    &threat.code,
                )
            });
        }
        if let Some(top) = player.discard.last() {
            paint.card(CardSpec {
                source: Some(Source::Discard(player.investigator)),
                physical_id: Some(top.id),
                ..CardSpec::new(
                    layout::investigator_discard(index),
                    &format!("{} cards\n{}", player.discard.len(), top.title),
                    &format!("discard:{}", player.investigator.id()),
                )
            });
        } else {
            paint.caption(layout::investigator_discard(index), "0 cards".into());
        }
        super::scene::deck(
            paint.commands,
            paint.meshes,
            paint.materials,
            context,
            layout::investigator_deck(index),
            player.deck_count,
            Source::Deck(player.investigator),
            false,
        );
        if let Some(top) = player.removed.last() {
            paint.card(CardSpec {
                source: Some(Source::Removed(player.investigator)),
                physical_id: Some(top.id),
                ..CardSpec::new(
                    layout::investigator_discard(index) + Vec3::new(0., 0., 3.),
                    &format!("Removed · {} cards", player.removed.len()),
                    &format!("removed:{}", player.investigator.id()),
                )
            });
        }
    }
    for (side, code, label, source) in [
        (
            layout::ScenarioBookSide::Agenda,
            format!("{:05}", 1104 + usize::from(play.agenda)),
            format!("Agenda {} · {} doom", play.agenda, play.agenda_doom),
            Source::Agenda,
        ),
        (
            layout::ScenarioBookSide::Act,
            format!("{:05}", 1107 + usize::from(play.act)),
            format!("Act {}", play.act),
            Source::Act,
        ),
    ] {
        let pose = layout::scenario_book_card_transform(side, Vec2::new(1.35, 1.89));
        paint.card(CardSpec {
            source: Some(source),
            pose,
            ..CardSpec::new(pose.translation, &label, &code)
        });
    }
    paint.card(CardSpec {
        source: Some(Source::ScenarioReference),
        ..CardSpec::new(
            layout::SCENARIO_REFERENCE,
            "The Gathering · Standard",
            "01104",
        )
    });
    paint.chaos_area(play.bag_count, Some(play.curse_pool));
    super::scene::deck(
        paint.commands,
        paint.meshes,
        paint.materials,
        context,
        layout::ENCOUNTER_DECK,
        play.encounter_deck_count,
        Source::EncounterDeck,
        true,
    );
    if let Some(top) = play.encounter_discard.last() {
        paint.card(CardSpec {
            source: Some(Source::EncounterDiscard),
            physical_id: Some(top.id),
            ..CardSpec::new(
                layout::ENCOUNTER_DISCARD,
                &format!("{} cards\n{}", play.encounter_discard.len(), top.title),
                "encounter-discard",
            )
        });
    } else {
        paint.caption(layout::ENCOUNTER_DISCARD, "0 cards".into());
    }
    let victory = play.scenario.victory_display();
    let points: u8 = victory.iter().map(|card| card.points).sum();
    let victory_caption = format!("Victory display · {} cards · {points} VP", victory.len());
    if let Some(top) = victory.last() {
        // The footprint supplies the pile name. Keep its changing count and
        // points compact instead of wrapping a whole sentence to a card's width.
        paint.card(CardSpec {
            source: Some(Source::Victory),
            physical_id: top.physical_id,
            caption: None,
            ..CardSpec::new(layout::VICTORY_DISPLAY, &victory_caption, "victory")
        });
    }
    paint.caption(
        layout::VICTORY_DISPLAY,
        format!("{} cards\n{points} VP", victory.len()),
    );
    if let Some(top) = play.scenario.removed.last() {
        paint.card(CardSpec {
            source: Some(Source::ScenarioRemoved),
            physical_id: Some(top.id),
            caption: None,
            ..CardSpec::new(
                layout::SCENARIO_REMOVED,
                &format!("Scenario removed · {} cards", play.scenario.removed.len()),
                "scenario-removed",
            )
        });
        paint.caption(
            layout::SCENARIO_REMOVED,
            format!("Removed\n{} cards", play.scenario.removed.len()),
        );
    }
    let map = crate::tabletop_gathering_map::GatheringMap::new(
        play.scenario
            .locations
            .iter()
            .filter(|room| !play.scenario.location_in_victory_display(&room.code))
            .map(|room| (room.code.as_str(), room.revealed)),
        Vec2::new(1.35, 1.89),
    )
    .expect("The selected Gathering locations fit their shared scenario surface");
    for link in map.spawn(paint.commands, paint.meshes, paint.materials) {
        paint.commands.entity(link).insert(ScenePiece);
    }
    for room in play
        .scenario
        .locations
        .iter()
        .filter(|room| !play.scenario.location_in_victory_display(&room.code))
    {
        let at = map.position(&room.code);
        let title = if crate::tabletop_gathering_map::GatheringMap::entry_blocked(
            &room.code,
            room.revealed,
        ) {
            format!("{}\nEntry blocked", location_name(&room.code))
        } else {
            location_name(&room.code).to_owned()
        };
        paint.card(CardSpec {
            source: Some(Source::Location(room.code.clone())),
            ..CardSpec::new(at, &title, &room.code)
        });
        paint.count(
            layout::location_clue(at),
            crate::tabletop_counters::CounterKind::Clue,
            room.clues,
            Source::Location(room.code.clone()),
        );
        let occupants = play.players.iter().enumerate().filter(|(_, player)| {
            !player.defeated && !player.resigned && player.location == room.code
        });
        for (slot, (index, player)) in occupants.enumerate() {
            paint.standee(at, slot, index, player.investigator);
        }
        if let Some(fog) = &room.fog {
            paint.card(CardSpec {
                source: Some(Source::Card(fog.id)),
                physical_id: Some(fog.id),
                ..CardSpec::new(at + Vec3::new(0., 0., -2.5), &fog.title, &fog.code)
            });
        }
    }
    for (slot, enemy) in play.scenario.enemies.iter().enumerate() {
        if play.scenario.location_in_victory_display(&enemy.location) {
            continue;
        }
        let group: Vec<_> = play
            .scenario
            .enemies
            .iter()
            .filter(|other| other.engaged.is_none() && other.location == enemy.location)
            .collect();
        let grouped = enemy.engaged.is_none() && group.len() > 1;
        let title = format!(
            "{}\n{} · {}\nDamage {} · Doom {} · Bounties {}\n{}",
            enemy.card.title,
            location_name(&enemy.location),
            if enemy.exhausted {
                "Exhausted"
            } else {
                "Ready"
            },
            enemy.damage,
            enemy.doom,
            enemy.bounties,
            enemy.engaged.map_or("Unengaged".into(), |owner| format!(
                "Engaged with {}",
                owner.name()
            ))
        );
        let caption = if grouped {
            let top = group.last().unwrap().card.id == enemy.card.id;
            let count = group.len();
            let title = enemy.card.title.clone();
            let exhausted = enemy.exhausted;
            top.then(|| {
                format!(
                    "{title}\n{count} enemies{}",
                    if exhausted { " · Top exhausted" } else { "" }
                )
            })
        } else {
            Some(title.clone())
        };
        let pose = Transform::from_translation(enemy_position(play, slot)).with_rotation(
            if enemy.exhausted {
                Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)
            } else {
                Quat::IDENTITY
            },
        );
        paint.card(CardSpec {
            source: Some(Source::Card(enemy.card.id)),
            physical_id: Some(enemy.card.id),
            pose,
            caption: caption.as_deref().map(|text| {
                if grouped {
                    Caption::EnemyGroup(text)
                } else {
                    Caption::Text(text)
                }
            }),
            ..CardSpec::new(pose.translation, &title, &enemy.card.code)
        });
    }
    if play.scenario.lita_in_play
        && !play
            .players
            .iter()
            .any(|player| player.assets.iter().any(|asset| asset.card.code == "01117"))
    {
        paint.card(CardSpec {
            source: Some(Source::Lita),
            ..CardSpec::new(
                layout::SCENARIO_MAP_CENTER + Vec3::new(3., 0., 4.),
                "Lita Chantler · Parlor",
                "01117",
            )
        });
    }
    // Test contains only the current skill test. Other public resolutions have
    // an investigator-owned tray, including while a parent test is suspended.
    let committed_count = play.test.as_ref().map_or(0, |test| test.committed.len());
    let test_cards = test_resolving_cards(play);
    let test_count = committed_count + test_cards.len();
    if let Some(test) = &play.test {
        if let Some(source) = &test.source_card {
            paint.card(CardSpec {
                source: Some(Source::Card(source.id)),
                physical_id: test_source_is_physical(play, source.id).then_some(source.id),
                ..CardSpec::new(layout::test_source(), &source.title, &source.code)
            });
        }
        for (slot, committed) in test.committed.iter().enumerate() {
            paint.card(CardSpec {
                source: Some(Source::Card(committed.card.id)),
                physical_id: Some(committed.card.id),
                ..CardSpec::new(
                    layout::test_commitment(test_count, slot),
                    &format!(
                        "{}\n{}'s commitment",
                        committed.card.title,
                        committed.owner.name()
                    ),
                    &committed.card.code,
                )
            });
        }
    }
    for (slot, card) in test_cards.iter().enumerate() {
        paint.card(CardSpec {
            source: Some(Source::Card(card.id)),
            physical_id: Some(card.id),
            ..CardSpec::new(
                layout::test_commitment(test_count, committed_count + slot),
                &card.title,
                &card.code,
            )
        });
    }
    let tray = trays::Layout::for_play(play);
    if let Some(tray) = &tray {
        for (slot, card) in tray.candidates.iter().enumerate() {
            paint.card(CardSpec {
                source: Some(Source::Card(card.id)),
                physical_id: Some(card.id),
                ..CardSpec::new(tray.candidate(slot), &card.title, &card.code)
            });
        }
        for (slot, card) in tray.sources.iter().enumerate() {
            paint.card(CardSpec {
                source: Some(Source::Card(card.id)),
                physical_id: Some(card.id),
                ..CardSpec::new(tray.source(slot), &card.title, &card.code)
            });
        }
        paint.caption(
            tray.source(0) - Vec3::Z * 2.3,
            format!("{} · Resolving", tray.owner.name()),
        );
    }
    if let Some(tokens) = &play.tokens {
        for (slot, token) in tokens.revealed.iter().enumerate() {
            let state = if tokens.cancelled.contains(&slot) {
                ShownTokenState::Cancelled
            } else if tokens.ignored.contains(&slot) {
                ShownTokenState::Ignored
            } else if tokens.selected.contains(&slot) {
                ShownTokenState::Selected
            } else {
                ShownTokenState::Revealed
            };
            paint.token(
                if tokens.is_skill_test() {
                    layout::test_token(false, slot)
                } else {
                    tray.as_ref()
                        .expect("Non-test tokens have an owner")
                        .token(slot)
                },
                *token,
                state,
                slot + 1,
                true,
            );
        }
    }
    if !play
        .tokens
        .as_ref()
        .is_some_and(|tokens| tokens.is_skill_test())
        && let Some(test) = &play.test
    {
        for (slot, token) in test.revealed.iter().enumerate() {
            paint.token(
                layout::test_token(false, slot),
                *token,
                if test.resolved {
                    ShownTokenState::Resolved
                } else {
                    ShownTokenState::Revealed
                },
                slot + 1,
                play.tokens.is_none(),
            );
        }
        for (slot, (token, state)) in test
            .ignored_tokens
            .iter()
            .map(|token| (token, ShownTokenState::Ignored))
            .chain(
                test.cancelled_tokens
                    .iter()
                    .map(|token| (token, ShownTokenState::Cancelled)),
            )
            .enumerate()
        {
            paint.token(
                layout::test_token(true, slot),
                *token,
                state,
                slot + 1,
                false,
            );
        }
    }
    if let Some(sealed) = &play.premonition {
        let at = trays::sealed_position(play, sealed.controller);
        paint.card(CardSpec {
            source: Some(Source::Card(sealed.card.id)),
            physical_id: Some(sealed.card.id),
            ..CardSpec::new(
                at,
                &format!(
                    "Premonition\n{} · Sealed\n{}",
                    sealed.controller.name(),
                    sealed
                        .faces
                        .iter()
                        .map(|face| face.label())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                &sealed.card.code,
            )
        });
        for (slot, token) in sealed.faces.iter().enumerate() {
            paint.token(
                at + Vec3::new(slot as f32 * 0.65, 0.1, 0.),
                *token,
                ShownTokenState::Selected,
                slot + 1,
                false,
            );
        }
    }
}

pub(super) fn enemy_position(play: &PlayObservation, slot: usize) -> Vec3 {
    let enemy = &play.scenario.enemies[slot];
    if let Some(owner) = enemy.engaged {
        let prior = play.scenario.enemies[..slot]
            .iter()
            .filter(|other| other.engaged == Some(owner))
            .count();
        return layout::investigator_threat(
            owner.index(),
            play.players[owner.index()].threats.len() + prior,
        );
    }
    let map = crate::tabletop_gathering_map::GatheringMap::new(
        play.scenario
            .locations
            .iter()
            .map(|room| (room.code.as_str(), room.revealed)),
        Vec2::new(1.35, 1.89),
    )
    .expect("The selected Gathering locations fit their shared scenario surface");
    let group = play
        .scenario
        .enemies
        .iter()
        .filter(|other| other.engaged.is_none() && other.location == enemy.location)
        .collect::<Vec<_>>();
    let ordinal = group
        .iter()
        .position(|other| other.card.id == enemy.card.id)
        .unwrap();
    // Preserve the left-edge clearance from location standees when a quarter
    // turn makes the card wider. Ready enemies retain their established anchor.
    let clearance = if enemy.exhausted {
        (crate::tabletop_pieces::CARD_LENGTH - crate::tabletop_pieces::CARD_WIDTH) / 2.
    } else {
        0.
    };
    local_enemy_position(map.position(&enemy.location), ordinal, group.len()) + Vec3::X * clearance
}

fn local_enemy_position(location: Vec3, ordinal: usize, count: usize) -> Vec3 {
    let step = if count > 1 {
        (1. / (count - 1) as f32).min(0.25)
    } else {
        0.
    };
    location + Vec3::new(3.2, ordinal as f32 * 0.015, ordinal as f32 * step)
}

pub(super) fn test_resolving_cards(play: &PlayObservation) -> Vec<&crate::gathering::CardView> {
    play.pending
        .resolving_cards
        .iter()
        .filter(|card| {
            play.pending.test_resolving_cards.contains(&card.id)
                && !play.test.as_ref().is_some_and(|test| {
                    test.source_card
                        .as_ref()
                        .is_some_and(|source| source.id == card.id)
                })
        })
        .collect()
}

fn test_source_is_physical(play: &PlayObservation, id: u16) -> bool {
    // Assets, enemies, and other cards with a lasting home keep that physical
    // copy; Test shows a reference. An event or revelation held in a temporary
    // continuation has only its Test copy.
    !play.players.iter().any(|player| {
        player.assets.iter().any(|asset| asset.card.id == id)
            || player
                .hand
                .iter()
                .chain(&player.threats)
                .chain(&player.discard)
                .chain(&player.removed)
                .any(|card| card.id == id)
    }) && !play
        .scenario
        .enemies
        .iter()
        .any(|enemy| enemy.card.id == id)
}

#[cfg(test)]
mod presentation_tests {
    use super::*;

    #[test]
    fn local_enemy_fans_clear_every_room_standee_reservation() {
        let rooms = ["01112", "01113", "01114", "01115"];
        let map = crate::tabletop_gathering_map::GatheringMap::new(
            rooms.map(|code| (code, true)),
            Vec2::new(1.35, 1.89),
        )
        .unwrap();
        for room in rooms {
            for count in [1, 2, 3, 8, 16] {
                for ordinal in 0..count {
                    let at = local_enemy_position(map.position(room), ordinal, count).xz();
                    let card = Rect::from_center_size(at, Vec2::new(1.35, 1.89));
                    for other in rooms {
                        let center = map.position(other).xz();
                        // Independent fixed oracle for the shared1.35×1.89
                        // location face plus reserved standees and captions.
                        let reserved = Rect::from_corners(
                            center + Vec2::new(-2.3, -1.145),
                            center + Vec2::new(2.3, 2.945),
                        );
                        assert!(
                            card.max.x < reserved.min.x
                                || card.min.x > reserved.max.x
                                || card.max.y < reserved.min.y
                                || card.min.y > reserved.max.y,
                            "{room} enemy {ordinal}/{count} overlaps {other}"
                        );
                    }
                }
            }
        }
    }
}
