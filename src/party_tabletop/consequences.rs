//! Party's committed clue discovery and observation-driven token return.
//!
//! These lifetimes are independent: a clue starts during action draining, while
//! returning tokens only exist after scene commands are applied. Keep that update
//! boundary, and the distinction between scene-owned tokens and persistent moving
//! clues, inside this module. Neither completion path changes rules or saves.
use super::*;
use crate::tabletop_counters::{self as counters, CounterKind, CounterVisual};
use crate::{
    TokenFace, TokenStatus,
    tabletop_tokens::{self, TokenVisual},
};
use bevy::ecs::system::SystemParam;

#[derive(Resource, Default)]
pub(super) struct Lifetimes {
    clue: Option<Transfer>,
    rendered: Option<Observation>,
}

/// The shared activity and transition interface used by actions, scene and redraw.
/// Querying applied entities here deliberately preserves deferred token admission.
#[derive(SystemParam)]
pub(super) struct Presentation<'w, 's> {
    lifetimes: ResMut<'w, Lifetimes>,
    returning: Query<'w, 's, (), With<ReturningToken>>,
}

impl Presentation<'_, '_> {
    pub fn is_active(&self) -> bool {
        self.lifetimes.clue.is_some() || !self.returning.is_empty()
    }

    pub fn blocks(&self, action: &Action) -> bool {
        self.is_active()
            && !matches!(
                action,
                Action::Focus(_) | Action::Close | Action::Dismiss | Action::Motion
            )
    }

    /// Only a successful live save-before-install transition supplies this event.
    pub fn committed(
        &mut self,
        before: &Observation,
        after: &Observation,
        choice: &str,
        reduced: bool,
    ) {
        self.lifetimes.clue = if reduced || matches!(choice, "undo" | "finish-test") {
            None
        } else {
            Transfer::from_committed_discovery(before, after)
        };
    }

    /// Reopen/restart success or explicit view navigation interrupts discovery.
    /// Keep the last rendered observation: replay can itself cause token return.
    pub fn replaced_view(&mut self) {
        self.lifetimes.clue = None;
    }

    pub fn prepare_view(
        &mut self,
        mut view: Observation,
        live: bool,
        reduced: bool,
    ) -> Option<View> {
        if live
            && !reduced
            && let Some(clue) = &self.lifetimes.clue
        {
            view.investigators[clue.investigator.index()].clues = clue.previous_count;
        }
        let last = &self.lifetimes.rendered;
        if last.as_ref() == Some(&view) {
            return None;
        }
        let new_tokens = last.as_ref().map_or(0, |old| old.held_tokens().len());
        let returning = !reduced
            && view.test.as_ref().is_some_and(|test| test.result.is_some())
            && last
                .as_ref()
                .is_some_and(|old| old.revision < view.revision && !old.held_tokens().is_empty());
        self.lifetimes.rendered = Some(view.clone());
        Some(View {
            observation: view,
            new_tokens,
            returning,
            reduced,
        })
    }
}

/// The projected scene and its token presentation are derived together, once.
pub(super) struct View {
    pub observation: Observation,
    new_tokens: usize,
    returning: bool,
    reduced: bool,
}

impl View {
    pub fn spawn_tokens(
        &self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        art: &Art,
    ) {
        let view = &self.observation;
        let mut token_slots = [0, 0];
        let visible_tokens = if self.returning {
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
                if index >= self.new_tokens && !self.reduced {
                    Vec3::splat(0.1)
                } else {
                    Vec3::ONE
                },
            );
            let (entity, label) = tabletop_tokens::spawn(
                commands,
                meshes,
                materials,
                TokenVisual {
                    face: token.face,
                    status: token.status,
                    ordinal: Some(usize::from(token.id) + 1),
                    image: art.image(&format!("token/{}", token_role(token.face))),
                    transform,
                    label_offset: Vec3::new(0., 0., 0.55),
                },
            );
            if self.returning {
                commands.entity(entity).insert((
                    Scene,
                    Pickable::IGNORE,
                    ReturningToken {
                        start: at,
                        destination: if token.face == TokenFace::Curse
                            && token.status == TokenStatus::Resolved
                        {
                            scene::curse_pool_at()
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
            let revision = view.revision;
            commands
                .entity(entity)
                .insert((Scene, Appearing, Pickable::default(), CardSource(source)))
                .observe(move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
                    if event.button == bevy::picking::pointer::PointerButton::Primary {
                        queue.0.push(Action::Select(revision, source, at));
                    }
                });
            feedback::attach_card_feedback(commands, meshes, materials, entity, 1., 1., 0.08);
            commands.entity(label).insert(Scene);
        }
    }
}

#[derive(Component)]
pub(super) struct ReturningToken {
    start: Vec3,
    pub(super) destination: Vec3,
    elapsed: f32,
}

pub(super) fn advance_returning(
    mut commands: Commands,
    time: Res<PresentationTime>,
    table: Res<Table>,
    mut returning: Query<(Entity, &mut Transform, &mut ReturningToken)>,
) {
    for (entity, mut transform, mut transfer) in &mut returning {
        transfer.elapsed += time.delta_secs();
        let fraction = if table.reduced_motion {
            1.
        } else {
            (transfer.elapsed / 0.6).min(1.)
        };
        let eased = fraction * fraction * (3. - 2. * fraction);
        transform.translation = transfer.start.lerp(transfer.destination, eased)
            + Vec3::Y * (std::f32::consts::PI * fraction).sin() * 0.5;
        transform.scale = Vec3::splat(1. - fraction * 0.9);
        if fraction >= 1. {
            commands.entity(entity).despawn();
        }
    }
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

const DURATION: f32 = 0.8;

#[derive(Debug, Clone)]
pub(super) struct Transfer {
    investigator: Investigator,
    previous_count: u8,
    amount: u8,
    start: Vec3,
    destination: Vec3,
    elapsed: f32,
}

impl Transfer {
    pub fn from_committed_discovery(before: &Observation, after: &Observation) -> Option<Self> {
        let pending = before.test.as_ref()?;
        let completed = after.test.as_ref()?;
        // Every represented Party test is an investigation of the revealed Study.
        // Require that actual transition and its exact source/recipient, rather
        // than interpreting history strings or pairing arbitrary counter deltas.
        if pending.result.is_some()
            || completed.result != Some(true)
            || pending.investigator != completed.investigator
            || after.revision <= before.revision
        {
            return None;
        }
        let investigator = completed.investigator;
        let previous = before.investigator(investigator);
        let current = after.investigator(investigator);
        let source = before
            .locations
            .iter()
            .find(|location| location.code == "01111")?;
        let destination_source = after
            .locations
            .iter()
            .find(|location| location.code == "01111")?;
        let amount = current.clues.checked_sub(previous.clues)?;
        if amount == 0
            || previous.location != "Study"
            || current.location != "Study"
            || !source.revealed
            || !destination_source.revealed
            || source.clues.checked_sub(destination_source.clues) != Some(amount)
        {
            return None;
        }
        let map =
            location_map::GatheringMap::new(&after.locations, Vec2::new(CARD_WIDTH, CARD_LENGTH))
                .ok()?;
        Some(Self {
            investigator,
            previous_count: previous.clues,
            amount,
            start: layout::location_clue(map.position("01111")),
            destination: layout::investigator_clue(investigator.index()),
            elapsed: 0.,
        })
    }
}

#[derive(Component)]
pub(super) struct MovingClue {
    label: Entity,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn advance_clue(
    mut commands: Commands,
    mut table: ResMut<Table>,
    mut lifetimes: ResMut<Lifetimes>,
    time: Res<PresentationTime>,
    art: Res<Art>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut moving: Query<(Entity, &MovingClue, &mut Transform)>,
) {
    let reduced = table.reduced_motion;
    let Some(transfer) = lifetimes.clue.as_mut() else {
        for (entity, clue, _) in &moving {
            commands.entity(entity).despawn();
            commands.entity(clue.label).try_despawn();
        }
        return;
    };
    transfer.elapsed += time.delta_secs();
    let fraction = if reduced {
        1.
    } else {
        (transfer.elapsed / DURATION).min(1.)
    };
    if fraction >= 1. {
        for (entity, clue, _) in &moving {
            commands.entity(entity).despawn();
            commands.entity(clue.label).try_despawn();
        }
        lifetimes.clue = None;
        table.dirty = true;
        return;
    }
    let at = counters::clue_transfer_position(transfer.start, transfer.destination, fraction);
    let scale = 1. - 0.15 * fraction;
    if moving.is_empty() {
        let (entity, label) = counters::spawn_count(
            &mut commands,
            &mut meshes,
            &mut materials,
            CounterVisual {
                kind: CounterKind::Clue,
                amount: transfer.amount,
                image: art.image("token/clue"),
                at,
                scale,
            },
        )
        .expect("a discovery moves at least one clue");
        commands.entity(entity).insert(MovingClue { label });
    } else {
        for (_, _, mut transform) in &mut moving {
            transform.translation = at;
            transform.scale = Vec3::splat(scale);
        }
    }
}
