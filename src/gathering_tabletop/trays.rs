//! Temporary resolution surfaces. ADR 0002 reserves Test for skill tests.
use super::*;
use crate::gathering::{CardView, PlayObservation};

pub(super) struct Layout<'a> {
    pub owner: Investigator,
    pub candidates: Vec<&'a CardView>,
    pub sources: Vec<&'a CardView>,
    token_count: usize,
}

impl<'a> Layout<'a> {
    pub(super) fn for_play(play: &'a PlayObservation) -> Option<Self> {
        let owner = play.pending.resolution_owner?;
        let mut candidates: Vec<_> = play
            .searched_encounter
            .iter()
            .chain(&play.searched_player)
            .collect();
        if let Some(discard) = &play.pending.discard {
            candidates.extend(discard.ordered.iter().chain(&discard.remaining));
        }
        let mut sources = Vec::new();
        for card in play.pending.resolving_cards.iter().chain(
            play.pending_revelation
                .iter()
                .map(|revelation| &revelation.card),
        ) {
            if !play.test.as_ref().is_some_and(|test| {
                test.source_card
                    .as_ref()
                    .is_some_and(|source| source.id == card.id)
            }) && !play.pending.test_resolving_cards.contains(&card.id)
                && !play
                    .premonition
                    .as_ref()
                    .is_some_and(|sealed| sealed.card.id == card.id)
                && !candidates.iter().any(|candidate| candidate.id == card.id)
                && !sources
                    .iter()
                    .any(|source: &&CardView| source.id == card.id)
            {
                sources.push(card);
            }
        }
        let tokens = play
            .tokens
            .as_ref()
            .filter(|tokens| !tokens.is_skill_test());
        Some(Self {
            owner,
            candidates,
            sources,
            token_count: tokens.map_or(0, |tokens| {
                tokens
                    .reveal_groups()
                    .iter()
                    .map(|group| group.members.len().div_ceil(3) * 3)
                    .sum()
            }),
        })
    }

    pub(super) fn candidate(&self, slot: usize) -> Vec3 {
        anchor(self.owner)
            + Vec3::new(
                -1.5 + (slot % 3) as f32 * 2.5,
                0.05,
                1.8 + (slot / 3) as f32 * 2.8,
            )
    }

    pub(super) fn source(&self, slot: usize) -> Vec3 {
        anchor(self.owner) + Vec3::new(-4.8, 0.05, 4.6 + slot as f32 * 2.8)
    }

    pub(super) fn token(&self, slot: usize) -> Vec3 {
        anchor(self.owner)
            + Vec3::new(
                -1.5 + (slot % 3) as f32 * 3.75,
                0.12,
                2.4 + self.candidates.len().div_ceil(3) as f32 * 2.8 + (slot / 3) as f32 * 3.0,
            )
    }

    pub(super) fn positions(&self) -> impl Iterator<Item = (u16, Vec3)> + '_ {
        self.candidates
            .iter()
            .enumerate()
            .map(|(slot, card)| (card.id, self.candidate(slot)))
            .chain(
                self.sources
                    .iter()
                    .enumerate()
                    .map(|(slot, card)| (card.id, self.source(slot))),
            )
    }

    fn depth(&self) -> f32 {
        let rows = self
            .candidates
            .len()
            .div_ceil(3)
            .max(self.sources.len() + 1);
        let card_depth = (3.5 + rows as f32 * 2.8).max(10.);
        // Sources sit beside the tokens, not before them. Adding both column
        // depths made Resolve zoom out enough to hide their captions.
        let token_depth = self
            .token_count
            .checked_sub(1)
            .map_or(0., |last| self.token(last).z - anchor(self.owner).z + 2.4);
        card_depth.max(token_depth)
    }

    pub(super) fn focus(&self) -> (Vec3, f32) {
        (
            anchor(self.owner) + Vec3::Z * (self.depth() / 2. + 0.5),
            22.0_f32.max(self.depth() * 1.8),
        )
    }

    pub(super) fn has_content(&self) -> bool {
        !self.candidates.is_empty() || !self.sources.is_empty() || self.token_count > 0
    }
}

fn anchor(owner: Investigator) -> Vec3 {
    Vec3::new(layout::investigator_center(owner.index(), 3).x, 0., 30.6)
}

pub(super) fn sealed_position(_play: &PlayObservation, owner: Investigator) -> Vec3 {
    anchor(owner) + Vec3::new(-4.8, 0.05, 1.8)
}

pub(super) fn sealed_token(
    play: &PlayObservation,
    owner: Investigator,
    slot: usize,
    count: usize,
) -> Vec3 {
    sealed_position(play, owner)
        + Vec3::new(
            0.,
            0.16,
            (slot as f32 - (count.saturating_sub(1)) as f32 * 0.5) * 0.95,
        )
}

#[derive(Clone, Copy, Default)]
struct Drawer {
    open: f32,
    wanted: bool,
    depth: f32,
}

#[derive(Default)]
pub(super) struct State([Drawer; 3]);

impl State {
    pub(super) fn prepare(&mut self, view: &Observation, immediate: bool) {
        let layout = view.play.as_ref().and_then(Layout::for_play);
        for (index, drawer) in self.0.iter_mut().enumerate() {
            drawer.wanted = layout
                .as_ref()
                .is_some_and(|layout| layout.owner.index() == index);
            drawer.wanted |= view
                .play
                .as_ref()
                .and_then(|play| play.premonition.as_ref())
                .is_some_and(|sealed| sealed.controller.index() == index);
            if drawer.wanted {
                drawer.depth = drawer.depth.max(10.);
            }
            if let Some(layout) = layout
                .as_ref()
                .filter(|layout| layout.owner.index() == index)
            {
                drawer.depth = layout.depth();
            }
            if immediate {
                drawer.open = if drawer.wanted { 1. } else { 0. };
            }
        }
    }

    pub(super) fn active(&self) -> bool {
        self.0
            .iter()
            .any(|drawer| drawer.open != if drawer.wanted { 1. } else { 0. })
    }

    pub(super) fn render(
        &self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        context: &CardRender<'_>,
    ) {
        for (index, drawer) in self
            .0
            .iter()
            .enumerate()
            .filter(|(_, drawer)| drawer.wanted || drawer.open > 0.)
        {
            for (layer, (width, depth, y, height, tint)) in [
                (
                    14.,
                    drawer.depth,
                    -0.24,
                    0.30,
                    layout::investigator_color(index),
                ),
                (
                    13.84,
                    drawer.depth - 0.16,
                    -0.075,
                    0.03,
                    layout::investigator_color(index).mix(&Color::BLACK, 0.48),
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let offset = Vec3::new(0., y, drawer.depth / 2.);
                let entity = commands
                    .spawn((
                        Name::new("Resolution tray"),
                        ScenePiece,
                        Surface { index, offset },
                        Mesh3d(meshes.add(Cuboid::new(width, height, depth))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: tint,
                            perceptual_roughness: 0.95,
                            ..default()
                        })),
                        Transform::from_translation(surface_position(index, drawer, offset)),
                        Pickable::IGNORE,
                    ))
                    .id();
                if layer == 1
                    && drawer.wanted
                    && let Some(play) = &context.view.play
                {
                    let group = if !play.searched_encounter.is_empty() {
                        Some((
                            controls::Source::SearchEncounter,
                            "Encounter search",
                            "searched-encounter",
                        ))
                    } else if !play.searched_player.is_empty() {
                        Some((
                            controls::Source::SearchPlayer,
                            "Backpack search",
                            "searched-player",
                        ))
                    } else if play.pending.discard.is_some() {
                        Some((
                            controls::Source::PendingDiscard,
                            "Discard order",
                            "pending-discard",
                        ))
                    } else if !play.pending.resolving_cards.is_empty()
                        || play.pending_revelation.is_some()
                    {
                        Some((
                            controls::Source::Resolving,
                            "Resolving cards",
                            "resolving-cards",
                        ))
                    } else {
                        None
                    };
                    if let Some((source, title, code)) = group {
                        scene::bind_source(
                            commands,
                            context,
                            entity,
                            controls::Selection {
                                source,
                                title: title.into(),
                                inspect_code: code.into(),
                                at: anchor(Investigator::ALL[index]) + offset,
                            },
                        );
                    }
                }
            }
        }
    }
}

#[derive(Component)]
pub(super) struct Surface {
    index: usize,
    offset: Vec3,
}

fn surface_position(index: usize, drawer: &Drawer, offset: Vec3) -> Vec3 {
    let t = drawer.open * drawer.open * (3. - 2. * drawer.open);
    anchor(Investigator::ALL[index]) + offset + Vec3::new(0., -0.5, -drawer.depth - 0.6) * (1. - t)
}

pub(super) fn animate(
    mut table: ResMut<Table>,
    time: Res<PresentationTime>,
    moving: Query<(), With<motion::CardMotion>>,
    mut surfaces: Query<(&Surface, &mut Transform, &mut Visibility)>,
) {
    if table.motion_blocked() {
        return;
    }
    let hold = !moving.is_empty() || table.is_presenting() || table.sealed.active();
    let immediate = table.reduced_motion;
    for drawer in &mut table.trays.0 {
        if immediate {
            drawer.open = if drawer.wanted { 1. } else { 0. };
        } else if drawer.wanted {
            drawer.open = (drawer.open + time.delta_secs() / 0.35).min(1.);
        } else if !hold {
            drawer.open = (drawer.open - time.delta_secs() / 0.35).max(0.);
        }
    }
    for (surface, mut transform, mut visibility) in &mut surfaces {
        let drawer = &table.trays.0[surface.index];
        transform.translation = surface_position(surface.index, drawer, surface.offset);
        *visibility = if drawer.open == 0. && !drawer.wanted {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}
