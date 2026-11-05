//! Visual beats of committed draws and resolutions. These views never execute rules.

use super::*;
use crate::gathering::{CardView, DrawBatch, DrawSource};
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
        view.choices.clear();
        view.decision_owner = None;
        for batch in batches {
            for card in &batch.cards {
                hide(&mut view, card.id);
            }
            if let DrawSource::Investigator(who) = batch.source {
                // Preserve original hand order while the rejected cards leave.
                let original = &before.investigators[who.index()].hand;
                let hand = &mut view.investigators[who.index()].hand;
                for card in &batch.returned {
                    hand.push(card.clone());
                }
                hand.sort_by_key(|card| {
                    original
                        .iter()
                        .position(|prior| prior.id == card.id)
                        .unwrap_or(usize::MAX)
                });
                sync_hand(&mut view, who);
            }
            let final_count = deck_count(target, batch.source);
            if interpolate_deck_count(before, target, batch) {
                set_deck_count(
                    &mut view,
                    batch.source,
                    final_count.saturating_sub(batch.returned.len()) + batch.cards.len(),
                );
            }
        }
        for batch in batches {
            let mut aside = Vec::new();
            if let DrawSource::Investigator(who) = batch.source {
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
            for (index, card) in batch.cards.iter().enumerate() {
                show(&mut view, target, card, batch.source);
                // Interpolate counts only when the public endpoints establish
                // an uninterrupted draw. A reset keeps its recorded count.
                let final_count = deck_count(target, batch.source);
                if interpolate_deck_count(before, target, batch) {
                    let remaining = final_count.saturating_sub(batch.returned.len())
                        + batch.cards.len()
                        - index
                        - 1;
                    set_deck_count(&mut view, batch.source, remaining);
                }
                let at = deck_position(batch.source)
                    + Vec3::Y * ((deck_count(&view, batch.source) + 1) as f32 * 0.0065 + 0.006);
                view.prompt = match batch.source {
                    DrawSource::Investigator(who) => format!("{} draws a card.", who.name()),
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
            if !aside.is_empty() {
                view.prompt = "Return the set-aside mulligan cards to their deck.".into();
                let transfers = aside
                    .iter()
                    .map(|card| Transfer {
                        id: card.id,
                        start: None,
                        source: batch.source,
                        turn: Turn::Down,
                    })
                    .collect();
                result.push(&view, &aside, batch.source, true, transfers);
            }
            set_deck_count(&mut view, batch.source, deck_count(target, batch.source));
        }
        result.resolutions(before, target);
        result
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
        if self.elapsed < DURATION || moving {
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

fn interpolate_deck_count(before: &Observation, target: &Observation, batch: &DrawBatch) -> bool {
    before.play.is_none()
        || (batch.returned.is_empty()
            && deck_count(before, batch.source).checked_sub(batch.cards.len())
                == Some(deck_count(target, batch.source)))
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
