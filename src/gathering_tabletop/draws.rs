//! Visual beats of committed draws and resolutions. These views never execute rules.

use super::*;
use crate::gathering::{CardView, DeckStep, DrawBatch, DrawSource};
use std::collections::VecDeque;

pub(super) const DURATION: f32 = 0.7;

#[derive(Clone, Copy)]
pub(super) struct Transfer {
    pub id: u16,
    pub start: Option<Vec3>,
    pub source: DrawSource,
    pub turn: Turn,
}

#[derive(Clone, Copy)]
pub(super) enum Turn {
    Up,
    Down,
    FaceUp,
}

struct Aside {
    card: CardView,
    at: Vec3,
    mulligan: bool,
}

struct Beat {
    view: Observation,
    aside: Vec<Aside>,
    transfers: Vec<Transfer>,
    rendered: bool,
    deck: Option<shuffle::Operation>,
}

#[derive(Default)]
pub(super) struct Playback {
    beats: VecDeque<Beat>,
    elapsed: f32,
}

impl Playback {
    pub(super) fn new(before: &Observation, target: &Observation, batches: &[DrawBatch]) -> Self {
        let mut result = Self::default();
        let mut view = target.clone();
        if let Some(play) = &mut view.play {
            play.pending.held_draws = before
                .play
                .as_ref()
                .map_or_else(Vec::new, |prior| prior.pending.held_draws.clone());
        }
        view.choices.clear();
        view.decision_owner = None;
        let mut initialized = Vec::new();
        for batch in batches {
            for card in &batch.cards {
                hide(&mut view, card.id);
            }
            for card in &batch.returned {
                hide(&mut view, card.id);
            }
            if let DrawSource::Investigator(who) = batch.source {
                // Preserve original hand order while the rejected cards leave.
                let original = &before.investigators[who.index()].hand;
                let hand = &mut view.investigators[who.index()].hand;
                for card in &batch.returned {
                    if original.iter().any(|held| held.id == card.id) {
                        hand.push(card.clone());
                    }
                }
                hand.sort_by_key(|card| {
                    original
                        .iter()
                        .position(|prior| prior.id == card.id)
                        .unwrap_or(usize::MAX)
                });
                sync_hand(&mut view, who);
            }
            if !initialized.contains(&batch.source) {
                let count = match batch.steps.first() {
                    Some(DeckStep::Shuffle { count }) => *count,
                    _ => deck_count(before, batch.source),
                };
                set_deck_count(&mut view, batch.source, count);
                if batch
                    .steps
                    .iter()
                    .any(|step| matches!(step, DeckStep::Recycle { .. }))
                {
                    copy_discard(&mut view, before, batch.source);
                }
                initialized.push(batch.source);
            }
        }
        // Returning search cards must reach the deck before its shuffle starts.
        result.resolutions(before, &view);
        for batch in batches {
            let mut aside = Vec::new();
            if let DrawSource::Investigator(who) = batch.source
                && !batch.cards.is_empty()
            {
                for card in &batch.returned {
                    hide(&mut view, card.id);
                    aside.push(card.clone());
                    view.prompt = format!("{} sets aside a mulligan card.", who.name());
                    result.push(
                        &view,
                        &aside,
                        batch.source,
                        false,
                        vec![Transfer {
                            id: card.id,
                            start: None,
                            source: batch.source,
                            turn: Turn::FaceUp,
                        }],
                    );
                }
            }
            for step in &batch.steps {
                match *step {
                    DeckStep::Draw { card, .. } | DeckStep::RevealHeld { card } => {
                        let at = match *step {
                            DeckStep::Draw { remaining, .. } => {
                                set_deck_count(&mut view, batch.source, remaining);
                                deck_position(batch.source)
                                    + Vec3::Y * ((remaining + 1) as f32 * 0.0065 + 0.006)
                            }
                            _ => {
                                let remaining = held_count(&view, batch.source).saturating_sub(1);
                                set_held_count(&mut view, batch.source, remaining);
                                deck_position(batch.source)
                                    + Vec3::Z * 2.4
                                    + Vec3::Y * (remaining as f32 * 0.0065)
                            }
                        };
                        let card = &batch.cards[card];
                        show(&mut view, target, card, batch.source);
                        view.prompt = match batch.source {
                            DrawSource::Investigator(who) => {
                                format!("{} draws a card.", who.name())
                            }
                            DrawSource::Encounter => "Draw and reveal an encounter card.".into(),
                        };
                        result.push(
                            &view,
                            &aside,
                            batch.source,
                            false,
                            vec![Transfer {
                                id: card.id,
                                start: Some(at),
                                source: batch.source,
                                turn: Turn::Up,
                            }],
                        );
                    }
                    DeckStep::Return => {
                        view.prompt = "Return the cards to their deck.".into();
                        let count = deck_count(&view, batch.source) + batch.returned.len();
                        set_deck_count(&mut view, batch.source, count);
                        let transfers = batch
                            .returned
                            .iter()
                            .map(|card| Transfer {
                                id: card.id,
                                start: None,
                                source: batch.source,
                                turn: Turn::Down,
                            })
                            .collect();
                        result.push(&view, &batch.returned, batch.source, true, transfers);
                        aside.clear();
                    }
                    DeckStep::Shuffle { count } => {
                        set_deck_count(&mut view, batch.source, count);
                        view.prompt = match batch.source {
                            DrawSource::Investigator(who) => {
                                format!("Shuffle {}'s deck.", who.name())
                            }
                            DrawSource::Encounter => "Shuffle the encounter deck.".into(),
                        };
                        result.deck_beat(
                            &view,
                            shuffle::Operation::Shuffle {
                                source: batch.source,
                                count,
                            },
                        );
                    }
                    DeckStep::Recycle { count } => {
                        copy_discard(&mut view, target, batch.source);
                        view.prompt = "Return the discard pile to the deck.".into();
                        result.deck_beat(
                            &view,
                            shuffle::Operation::Recycle {
                                source: batch.source,
                                count,
                                base: deck_count(&view, batch.source),
                            },
                        );
                        let total = deck_count(&view, batch.source) + count;
                        set_deck_count(&mut view, batch.source, total);
                    }
                    DeckStep::TakeHidden { remaining, held } => {
                        set_deck_count(&mut view, batch.source, remaining);
                        set_held_count(&mut view, batch.source, held.saturating_sub(1));
                        view.prompt =
                            "Draw a card face down; the simultaneous draw is not yet complete."
                                .into();
                        result.deck_beat(
                            &view,
                            shuffle::Operation::Take {
                                source: batch.source,
                                remaining,
                                held,
                            },
                        );
                        set_held_count(&mut view, batch.source, held);
                    }
                }
            }
        }
        result
    }

    fn deck_beat(&mut self, view: &Observation, operation: shuffle::Operation) {
        self.beats.push_back(Beat {
            view: view.clone(),
            aside: vec![],
            transfers: vec![],
            rendered: false,
            deck: Some(operation),
        });
    }

    pub(super) fn deck_motion(&self) -> Option<(shuffle::Operation, f32)> {
        Some((self.beats.front()?.deck?, self.elapsed))
    }

    /// Searches expose only the cards already disclosed by the rules. They stay
    /// in their rules zones; these transfers are temporary presentation copies.
    fn resolutions(&mut self, before: &Observation, target: &Observation) {
        let searched = |view: &Observation| -> Vec<(CardView, DrawSource)> {
            let Some(play) = &view.play else {
                return vec![];
            };
            let mut cards = play
                .searched_encounter
                .iter()
                .cloned()
                .map(|card| (card, DrawSource::Encounter))
                .collect::<Vec<_>>();
            if let Some(owner) = play.pending.resolution_owner {
                cards.extend(
                    play.searched_player
                        .iter()
                        .cloned()
                        .map(|card| (card, DrawSource::Investigator(owner))),
                );
            }
            cards
        };
        let prior = searched(before);
        let next = searched(target);
        let mut transfers = Vec::new();
        let mut aside = Vec::new();
        for (card, source) in &next {
            if !prior.iter().any(|(prior, _)| prior.id == card.id) {
                transfers.push(Transfer {
                    id: card.id,
                    start: Some(
                        deck_position(*source)
                            + Vec3::Y * (deck_count(target, *source) as f32 * 0.0065 + 0.012),
                    ),
                    source: *source,
                    turn: Turn::Up,
                });
            }
        }
        let destinations = motion::Origins::from_view(Some(target));
        for (card, source) in prior {
            if destinations.contains(card.id) {
                continue;
            }
            // Backpack attachments have a lasting face-down home under the
            // asset. Their existing aggregate menu remains the inspection route.
            let attachment = target.play.as_ref().and_then(|play| {
                play.backpacks
                    .iter()
                    .find(|pack| pack.cards.iter().any(|attached| attached.id == card.id))
                    .and_then(|pack| {
                        play.players[pack.owner.index()]
                            .assets
                            .iter()
                            .position(|asset| asset.card.id == pack.asset)
                            .map(|slot| {
                                layout::investigator_asset(pack.owner.index(), slot)
                                    + Vec3::new(0.25, 0.025, 0.)
                            })
                    })
            });
            let at = attachment.unwrap_or_else(|| {
                deck_position(source)
                    + Vec3::Y * (deck_count(target, source) as f32 * 0.0065 + 0.012)
            });
            transfers.push(Transfer {
                id: card.id,
                start: None,
                source,
                turn: Turn::Down,
            });
            aside.push(Aside {
                card,
                at,
                mulligan: false,
            });
        }
        // The lasting discard representation shows only its top card. Keep the
        // other ordered cards visible for their trip to the same pile as well.
        if let Some(discard) = before
            .play
            .as_ref()
            .and_then(|play| play.pending.discard.as_ref())
            && let Some(play) = &target.play
        {
            let pile = &play.players[discard.owner.index()].discard;
            for card in discard.ordered.iter().chain(&discard.remaining) {
                if pile.iter().any(|placed| placed.id == card.id)
                    && pile.last().is_some_and(|top| top.id != card.id)
                {
                    transfers.push(Transfer {
                        id: card.id,
                        start: None,
                        source: DrawSource::Investigator(discard.owner),
                        turn: Turn::FaceUp,
                    });
                    aside.push(Aside {
                        card: card.clone(),
                        at: layout::investigator_discard(discard.owner.index()) - Vec3::Y * 0.005,
                        mulligan: false,
                    });
                }
            }
        }
        if !transfers.is_empty() {
            let mut view = target.clone();
            view.choices.clear();
            view.decision_owner = None;
            self.beats.push_back(Beat {
                view,
                aside,
                transfers,
                rendered: false,
                deck: None,
            });
        }
    }

    fn push(
        &mut self,
        view: &Observation,
        aside: &[CardView],
        source: DrawSource,
        returning: bool,
        transfers: Vec<Transfer>,
    ) {
        let at = deck_position(source)
            + if returning {
                Vec3::Y * (deck_count(view, source) as f32 * 0.0065 + 0.012)
            } else {
                Vec3::Z * 2.4
            };
        self.beats.push_back(Beat {
            view: view.clone(),
            aside: aside
                .iter()
                .enumerate()
                .map(|(index, card)| Aside {
                    card: card.clone(),
                    at: at + Vec3::new(index as f32 * 0.035, index as f32 * 0.008, 0.),
                    mulligan: true,
                })
                .collect(),
            transfers,
            rendered: false,
            deck: None,
        });
    }

    pub(super) fn view(&self) -> Option<&Observation> {
        self.beats.front().map(|beat| &beat.view)
    }

    pub(super) fn active(&self) -> bool {
        !self.beats.is_empty()
    }

    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(super) fn returning(&self, id: u16) -> Option<DrawSource> {
        self.beats
            .front()?
            .transfers
            .iter()
            .find(|transfer| transfer.id == id && matches!(transfer.turn, Turn::Down))
            .map(|transfer| transfer.source)
    }

    pub(super) fn take_transfers(&mut self) -> Vec<Transfer> {
        let Some(beat) = self.beats.front_mut() else {
            return vec![];
        };
        if beat.rendered {
            return vec![];
        }
        beat.rendered = true;
        beat.transfers.clone()
    }

    /// A long frame may finish one visible beat, never skip unseen cards.
    pub(super) fn advance(&mut self, delta: f32, moving: bool) -> bool {
        if !self.beats.front().is_some_and(|beat| beat.rendered) {
            return false;
        }
        self.elapsed += delta;
        let duration = self
            .beats
            .front()
            .and_then(|beat| beat.deck)
            .map_or(DURATION, shuffle::Operation::duration);
        if self.elapsed < duration || moving {
            return false;
        }
        self.beats.pop_front();
        self.elapsed = 0.;
        true
    }

    pub(super) fn render(
        &self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        context: &CardRender<'_>,
    ) {
        let Some(beat) = self.beats.front() else {
            return;
        };
        if let Some(operation) = beat.deck {
            shuffle::spawn(
                commands,
                meshes,
                materials,
                context,
                operation,
                self.elapsed,
            );
        }
        for (index, aside) in beat.aside.iter().enumerate() {
            // The small stack has one caption. Returning cards need none next
            // to the existing deck count; the prompt already describes the move.
            let caption = (aside.mulligan
                && index + 1 == beat.aside.len()
                && self.returning(aside.card.id).is_none())
            .then(|| format!("Mulligan cards · {}", beat.aside.len()));
            // Leave the card's direct inspection observer in place: this public
            // face is outside the observed hand and has no gameplay choices.
            let entity = card(
                commands,
                meshes,
                materials,
                context,
                CardSpec {
                    physical_id: Some(aside.card.id),
                    caption: caption.as_deref().map(Caption::Text),
                    ..CardSpec::new(aside.at, &aside.card.title, &aside.card.code)
                },
            );
            if !aside.mulligan {
                // These cards are already represented by their committed pile
                // or attachment; only the lasting menu may inspect them.
                commands.entity(entity).insert(Pickable::IGNORE);
            }
        }
    }
}

pub(super) fn deck_position(source: DrawSource) -> Vec3 {
    match source {
        DrawSource::Investigator(who) => layout::investigator_deck(who.index()),
        DrawSource::Encounter => layout::ENCOUNTER_DECK,
    }
}

fn deck_count(view: &Observation, source: DrawSource) -> usize {
    match source {
        DrawSource::Investigator(who) => view.investigators[who.index()].deck_count,
        DrawSource::Encounter => view
            .play
            .as_ref()
            .map_or(0, |play| play.encounter_deck_count),
    }
}

fn set_deck_count(view: &mut Observation, source: DrawSource, count: usize) {
    match source {
        DrawSource::Investigator(who) => {
            view.investigators[who.index()].deck_count = count;
            if let Some(play) = &mut view.play {
                play.players[who.index()].deck_count = count;
            }
        }
        DrawSource::Encounter => {
            if let Some(play) = &mut view.play {
                play.encounter_deck_count = count;
            }
        }
    }
}

fn held_count(view: &Observation, source: DrawSource) -> usize {
    let DrawSource::Investigator(who) = source else {
        return 0;
    };
    view.play
        .as_ref()
        .and_then(|play| {
            play.pending
                .held_draws
                .iter()
                .find(|held| held.investigator == who)
        })
        .map_or(0, |held| held.count)
}

fn set_held_count(view: &mut Observation, source: DrawSource, count: usize) {
    let (Some(play), DrawSource::Investigator(who)) = (&mut view.play, source) else {
        return;
    };
    play.pending
        .held_draws
        .retain(|held| held.investigator != who);
    if count > 0 {
        play.pending
            .held_draws
            .push(crate::gathering::HeldDrawView {
                investigator: who,
                count,
            });
    }
}

fn copy_discard(view: &mut Observation, from: &Observation, source: DrawSource) {
    let (Some(play), Some(from)) = (&mut view.play, &from.play) else {
        return;
    };
    match source {
        DrawSource::Investigator(who) => {
            play.players[who.index()].discard = from.players[who.index()].discard.clone()
        }
        DrawSource::Encounter => play.encounter_discard = from.encounter_discard.clone(),
    }
}

fn sync_hand(view: &mut Observation, who: Investigator) {
    if let Some(play) = &mut view.play {
        play.players[who.index()].hand = view.investigators[who.index()].hand.clone();
    }
}

fn hide(view: &mut Observation, id: u16) {
    for player in &mut view.investigators {
        player.hand.retain(|card| card.id != id);
    }
    if let Some(play) = &mut view.play {
        if let Some(test) = &mut play.test
            && test.source_card.as_ref().is_some_and(|card| card.id == id)
        {
            test.source_card = None;
        }
        for player in &mut play.players {
            player.hand.retain(|card| card.id != id);
        }
        play.pending.resolving_cards.retain(|card| card.id != id);
        if play
            .pending_revelation
            .as_ref()
            .is_some_and(|pending| pending.card.id == id)
        {
            play.pending_revelation = None;
        }
    }
}

fn show(view: &mut Observation, target: &Observation, card: &CardView, source: DrawSource) {
    if let DrawSource::Investigator(who) = source
        && target.investigators[who.index()]
            .hand
            .iter()
            .any(|held| held.id == card.id)
    {
        view.investigators[who.index()].hand.push(card.clone());
        sync_hand(view, who);
    } else if let (Some(play), Some(target)) = (&mut view.play, &target.play) {
        if target
            .pending_revelation
            .as_ref()
            .is_some_and(|pending| pending.card.id == card.id)
        {
            play.pending_revelation = target.pending_revelation.clone();
        } else if target
            .pending
            .resolving_cards
            .iter()
            .any(|held| held.id == card.id)
        {
            play.pending.resolving_cards.push(card.clone());
        }
    }
}
