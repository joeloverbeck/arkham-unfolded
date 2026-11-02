//! Presentation of the shared runtime observation. No legality or hidden state lives here.

use super::*;
use crate::gathering::{CardView, Phase, PlayObservation};

fn location_name(code: &str) -> &str {
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
        lines.push(format!("\n{} · copy {}", card.title, card.id));
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
                &format!("{} · Backpack copy {id}", attached.owner.name()),
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
        ("victory", "Victory display", &play.scenario.victory),
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

pub(super) fn player_status(play: &PlayObservation, index: usize) -> String {
    let player = &play.players[index];
    let state = if player.defeated {
        " · Defeated"
    } else if player.resigned {
        " · Resigned"
    } else {
        ""
    };
    format!(
        "{}{} · {}\nResources {} · Clues {} · Actions {} · Deck {}\nDamage {} · Horror {} · Trauma {}/{}",
        player.investigator.name(),
        state,
        location_name(&player.location),
        player.resources,
        player.clues,
        play.actions_remaining[index],
        player.deck_count,
        player.damage,
        player.horror,
        player.physical_trauma,
        player.mental_trauma
    )
}

pub(super) fn focus(view: &Observation) -> Area {
    if let Some(play) = &view.play {
        if play.scenario.outcome.is_some() {
            return Area::Scenario;
        }
        if play.test.is_some()
            || play.tokens.is_some()
            || play.pending_revelation.is_some()
            || !play.searched_encounter.is_empty()
        {
            return Area::Test;
        }
    }
    view.decision_owner
        .or_else(|| view.play.as_ref().and_then(|play| play.actor))
        .map_or(Area::Scenario, |owner| Area::Investigator(owner.index()))
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
    if let Some(test) = &play.test {
        lines.push(format!(
            "{} · {:?} test · Printed base {} · Difficulty {}",
            test.tester.name(),
            test.skill,
            test.base_skill,
            test.difficulty
        ));
        let chaos = if test.modifiers.automatic_failure {
            "automatic failure".into()
        } else {
            test.modifiers
                .chaos
                .map_or_else(|| "pending".into(), |value| format!("{value:+}"))
        };
        lines.push(format!(
            "Cards {:+} · Other bonuses {:+} · Chaos {chaos}",
            test.modifiers.cards, test.modifiers.other
        ));
        if let (Some(total), Some(success)) = (test.total, test.would_succeed) {
            lines.push(format!(
                "Total {total} · {}",
                if success {
                    "Would succeed"
                } else {
                    "Would fail"
                }
            ));
        }
    }
    if let Some(pending) = &play.pending_revelation {
        lines.push(format!(
            "{} drew {} · copy {}{}",
            pending.drawn_by.name(),
            pending.card.title,
            pending.card.id,
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
                |asset| format!("{} · copy {}", asset.title, asset.id),
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
            .map(|card| format!("{} · copy {}", card.title, card.id))
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
}

impl Painter<'_, '_, '_> {
    fn card(&mut self, at: Vec3, title: &str, code: &str, color: Color) -> Entity {
        card(
            self.commands,
            self.meshes,
            self.materials,
            at,
            title,
            code,
            false,
            color,
        )
    }

    fn label(&mut self, at: Vec3, value: String) {
        self.commands.spawn((
            ScenePiece,
            WorldLabel(at, true),
            text(value, 15., PAPER),
            Node {
                position_type: PositionType::Absolute,
                width: px(400),
                ..default()
            },
            Pickable::IGNORE,
        ));
    }

    fn token(&mut self, at: Vec3, face: crate::TokenFace, state: ShownTokenState, ordinal: usize) {
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
                image: None,
                transform: Transform::from_translation(at),
                label_offset: Vec3::Y * 0.2,
            },
        );
        self.commands.entity(disk).insert(ScenePiece);
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
            .remove::<crate::tabletop_tokens::TokenLabel>()
            .insert((
                ScenePiece,
                WorldLabel(at + Vec3::Y * 0.2, false),
                Text::new(format!(
                    "Token {ordinal} · {}\n{}",
                    face.label(),
                    state.label()
                )),
            ));
    }
}

pub(super) fn render(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    view: &Observation,
) {
    let Some(play) = &view.play else { return };
    let mut paint = Painter {
        commands,
        meshes,
        materials,
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
            let entity = paint.card(
                layout::investigator_asset(index, slot),
                &format!(
                    "{} · copy {}\n{} · Uses {}\nDamage {} · Horror {}{}",
                    asset.card.title,
                    asset.card.id,
                    if asset.ready { "Ready" } else { "Exhausted" },
                    asset.uses,
                    asset.damage,
                    asset.horror,
                    attached.map_or(String::new(), |attached| format!(
                        "\n{} face-down attachments",
                        attached.cards.len()
                    ))
                ),
                &inspection,
                PAPER,
            );
            paint
                .commands
                .entity(entity)
                .insert(motion::PhysicalCard(asset.card.id));
        }
        for (slot, threat) in player.threats.iter().enumerate() {
            let entity = paint.card(
                layout::investigator_threat(index, slot),
                &format!("{} · copy {}", threat.title, threat.id),
                &threat.code,
                PAPER,
            );
            paint
                .commands
                .entity(entity)
                .insert(motion::PhysicalCard(threat.id));
        }
        paint.card(
            layout::investigator_discard(index),
            &format!("Discard · {} cards", player.discard.len()),
            &format!("discard:{}", player.investigator.id()),
            MUTED,
        );
        paint.card(
            layout::investigator_deck(index),
            &format!("Deck · {} cards", player.deck_count),
            "hidden-player-deck",
            MUTED,
        );
        if !player.removed.is_empty() {
            paint.card(
                layout::investigator_discard(index) + Vec3::new(0., 0., 3.),
                &format!("Removed · {} cards", player.removed.len()),
                &format!("removed:{}", player.investigator.id()),
                MUTED,
            );
        }
    }
    for (side, code, label) in [
        (
            layout::ScenarioBookSide::Agenda,
            format!("{:05}", 1104 + usize::from(play.agenda)),
            format!("Agenda {} · {} doom", play.agenda, play.agenda_doom),
        ),
        (
            layout::ScenarioBookSide::Act,
            format!("{:05}", 1107 + usize::from(play.act)),
            format!("Act {}", play.act),
        ),
    ] {
        let pose = layout::scenario_book_card_transform(side, Vec2::new(1.35, 1.89));
        let entity = paint.card(pose.translation, &label, &code, PAPER);
        paint.commands.entity(entity).insert(pose);
    }
    paint.card(
        layout::SCENARIO_REFERENCE,
        "The Gathering · Standard",
        "01104",
        PAPER,
    );
    paint.card(
        layout::test_bag(),
        &format!("Chaos bag · {} tokens", play.bag_count),
        "chaos-bag",
        MUTED,
    );
    paint.card(
        layout::ENCOUNTER_DECK,
        &format!("Encounter deck · {} cards", play.encounter_deck_count),
        "encounter-deck",
        MUTED,
    );
    paint.card(
        layout::ENCOUNTER_DISCARD,
        &format!("Encounter discard · {} cards", play.encounter_discard.len()),
        "encounter-discard",
        MUTED,
    );
    paint.card(
        layout::VICTORY_DISPLAY,
        &format!("Victory display · {} cards", play.scenario.victory.len()),
        "victory",
        MUTED,
    );
    let map = crate::tabletop_gathering_map::GatheringMap::new(
        play.scenario
            .locations
            .iter()
            .map(|room| (room.code.as_str(), room.revealed)),
        Vec2::new(1.35, 1.89),
    )
    .expect("The selected Gathering locations fit their shared scenario surface");
    for link in map.spawn(paint.commands, paint.meshes, paint.materials) {
        paint.commands.entity(link).insert(ScenePiece);
    }
    for room in &play.scenario.locations {
        let at = map.position(&room.code);
        let title = format!(
            "{} · {}\n{} clues",
            location_name(&room.code),
            if crate::tabletop_gathering_map::GatheringMap::entry_blocked(&room.code, room.revealed)
            {
                "Entry blocked"
            } else if room.revealed {
                "Revealed"
            } else {
                "Unrevealed"
            },
            room.clues
        );
        paint.card(at, &title, &room.code, PAPER);
        let occupants = play.players.iter().enumerate().filter(|(_, player)| {
            !player.defeated && !player.resigned && player.location == room.code
        });
        for (slot, (index, _)) in occupants.enumerate() {
            if let Some(position) =
                layout::location_standee_position(at, Vec2::new(1.35, 1.89), slot)
            {
                let entity = crate::tabletop_standees::spawn(
                    paint.commands,
                    paint.meshes,
                    paint.materials,
                    None,
                    position,
                    layout::investigator_class(index),
                    Vec2::new(1.35, 1.89),
                );
                paint.commands.entity(entity).insert(ScenePiece);
            }
        }
        if let Some(fog) = &room.fog {
            paint.card(
                at + Vec3::new(0., 0., -2.5),
                &format!("{} · copy {}", fog.title, fog.id),
                &fog.code,
                PAPER,
            );
        }
    }
    for (slot, enemy) in play.scenario.enemies.iter().enumerate() {
        let entity = paint.card(
            enemy_position(slot),
            &format!(
                "{} · copy {}\n{} · {}\nDamage {} · Doom {} · Bounties {}\n{}",
                enemy.card.title,
                enemy.card.id,
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
            ),
            &enemy.card.code,
            PAPER,
        );
        paint
            .commands
            .entity(entity)
            .insert(motion::PhysicalCard(enemy.card.id));
    }
    if play.scenario.lita_in_play
        && !play
            .players
            .iter()
            .any(|player| player.assets.iter().any(|asset| asset.card.code == "01117"))
    {
        paint.card(
            layout::SCENARIO_MAP_CENTER + Vec3::new(3., 0., 4.),
            "Lita Chantler · Parlor",
            "01117",
            PAPER,
        );
    }
    if let Some(test) = &play.test {
        paint.label(
            layout::test_heading(),
            format!(
                "{} · {:?}\nPrinted base {} · Difficulty {}",
                test.tester.name(),
                test.skill,
                test.base_skill,
                test.difficulty
            ),
        );
        if let Some(source) = &test.source_card {
            paint.card(
                layout::test_source(),
                &format!("{} · copy {}", source.title, source.id),
                &source.code,
                PAPER,
            );
        }
        for (slot, committed) in test.committed.iter().enumerate() {
            let entity = paint.card(
                layout::test_commitment(test.committed.len(), slot),
                &format!(
                    "{} · copy {}\n{}",
                    committed.card.title,
                    committed.card.id,
                    committed.owner.name()
                ),
                &committed.card.code,
                PAPER,
            );
            paint
                .commands
                .entity(entity)
                .insert(motion::PhysicalCard(committed.card.id));
        }
    }
    if let Some(pending) = &play.pending_revelation {
        paint.card(
            layout::test_source(),
            &format!(
                "{} · copy {}\nDrawn by {}",
                pending.card.title,
                pending.card.id,
                pending.drawn_by.name()
            ),
            &pending.card.code,
            PAPER,
        );
    }
    for (slot, searched) in play.searched_encounter.iter().enumerate() {
        paint.card(
            layout::test_commitment(play.searched_encounter.len(), slot),
            &format!("Search · {} · copy {}", searched.title, searched.id),
            &searched.code,
            PAPER,
        );
    }
    if !play.searched_player.is_empty() {
        paint.card(
            layout::test_source(),
            &format!(
                "Backpack search · {} cards\nInspect revealed cards",
                play.searched_player.len()
            ),
            "searched-player",
            PAPER,
        );
    }
    if !play.pending.resolving_cards.is_empty() {
        paint.card(
            resolving_position(),
            &format!("Resolving cards · {}", play.pending.resolving_cards.len()),
            "resolving-cards",
            PAPER,
        );
    }
    if play.pending.discard.is_some() {
        paint.card(
            layout::test_source() + Vec3::new(5., 0., 0.),
            "Pending discard order",
            "pending-discard",
            PAPER,
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
            paint.token(layout::test_token(false, slot), *token, state, slot + 1);
        }
    } else if let Some(test) = &play.test {
        for (slot, token) in test.revealed.iter().enumerate() {
            paint.token(
                layout::test_token(false, slot),
                *token,
                ShownTokenState::Resolved,
                slot + 1,
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
            paint.token(layout::test_token(true, slot), *token, state, slot + 1);
        }
    }
    if let Some(sealed) = &play.premonition {
        paint.card(
            layout::test_bag() + Vec3::new(0., 0., 2.5),
            &format!(
                "Premonition · copy {}\n{} · Sealed\n{}",
                sealed.card.id,
                sealed.controller.name(),
                sealed
                    .faces
                    .iter()
                    .map(|face| face.label())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            &sealed.card.code,
            PAPER,
        );
    }
}

pub(super) fn enemy_position(slot: usize) -> Vec3 {
    layout::SCENARIO_MAP_CENTER
        + Vec3::new(
            -6.5 + (slot % 6) as f32 * 2.6,
            0.,
            7.5 + (slot / 6) as f32 * 3.,
        )
}

pub(super) fn resolving_position() -> Vec3 {
    layout::test_source() + Vec3::new(2.5, 0., 0.)
}
