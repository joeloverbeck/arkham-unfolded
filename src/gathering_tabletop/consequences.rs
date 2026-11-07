//! Party's physical clue and token paths, admitted from committed public facts.
use super::*;
use crate::{gathering::TestView, tabletop_counters as counters, tabletop_tokens as tokens};

#[derive(Clone)]
struct Clue {
    investigator: usize,
    previous: u8,
    amount: u8,
    start: Vec3,
    end: Vec3,
    elapsed: f32,
}

#[derive(Clone)]
struct Departure {
    face: crate::TokenFace,
    status: crate::TokenStatus,
    start: Transform,
    end: Vec3,
}

#[derive(Resource, Default)]
pub(super) struct State {
    clue: Option<Clue>,
    pending_test: Option<(TestView, usize)>,
    departures: Vec<Departure>,
    returning: usize,
    skip_next: bool,
}

impl State {
    pub(super) fn active(&self) -> bool {
        self.clue.is_some() || !self.departures.is_empty() || self.returning > 0
    }

    pub(super) fn clear(&mut self) {
        self.clue = None;
        self.pending_test = None;
        self.departures.clear();
        self.returning = 0;
        self.skip_next = true;
    }

    pub(super) fn shown_clues(&self, investigator: usize, current: u8) -> u8 {
        self.clue
            .as_ref()
            .filter(|clue| clue.investigator == investigator)
            .map_or(current, |clue| clue.previous)
    }

    pub(super) fn prepare(
        &mut self,
        before: Option<&Observation>,
        after: &Observation,
        live: bool,
        reduced: bool,
    ) {
        if !live || reduced {
            self.clear();
            return;
        }
        if std::mem::take(&mut self.skip_next) {
            return;
        }
        let Some(after_play) = after.play.as_ref() else {
            return;
        };
        let Some(before) = before else {
            if let Some(test) = after_play.test.as_ref().filter(|test| test.resolved) {
                self.pending_test = Some((test.clone(), after_play.completed_tests.len()));
            }
            return;
        };
        let Some(before_play) = before.play.as_ref() else {
            return;
        };
        if let Some(test) = before_play.test.as_ref().filter(|test| test.resolved) {
            self.pending_test = Some((test.clone(), before_play.completed_tests.len()));
        }
        if self.clue.is_none() {
            self.clue = discovery(before, after);
        }
        if after_play.completed_tests.len() == before_play.completed_tests.len() + 1
            && let Some((test, count)) = self.pending_test.take()
            && count == before_play.completed_tests.len()
            && let Some(result) = after_play.completed_tests.last()
            && result.tester == test.tester
            && Some(result.total) == test.total
            && result.difficulty == test.difficulty
            && Some(result.success) == test.would_succeed
        {
            let token_count =
                test.revealed.len() + test.ignored_tokens.len() + test.cancelled_tokens.len();
            for (slot, face) in test.revealed.iter().enumerate() {
                self.departures.push(Departure {
                    face: *face,
                    status: crate::TokenStatus::Resolved,
                    start: layout::gathering_test_token(slot, token_count),
                    end: if *face == crate::TokenFace::Curse {
                        layout::test_curse_pool()
                    } else {
                        layout::test_bag()
                    },
                });
            }
            for (slot, (face, status)) in test
                .ignored_tokens
                .iter()
                .map(|face| (face, crate::TokenStatus::Pending))
                .chain(
                    test.cancelled_tokens
                        .iter()
                        .map(|face| (face, crate::TokenStatus::Canceled)),
                )
                .enumerate()
            {
                self.departures.push(Departure {
                    face: *face,
                    status,
                    start: layout::gathering_test_token(test.revealed.len() + slot, token_count),
                    end: layout::test_bag(),
                });
            }
        }
        if let Some(test) = after_play.test.as_ref().filter(|test| test.resolved) {
            self.pending_test = Some((test.clone(), after_play.completed_tests.len()));
        }
    }
}

fn discovery(before: &Observation, after: &Observation) -> Option<Clue> {
    let (before, after) = (before.play.as_ref()?, after.play.as_ref()?);
    let prior = before.test.as_ref()?;
    let location = prior.investigation_location.as_deref()?;
    if !prior.resolved || prior.would_succeed != Some(true) {
        return None;
    }
    if let Some(next) = &after.test {
        if next.investigation_location.as_deref() != Some(location)
            || prior.tester != next.tester
            || !next.resolved
            || next.would_succeed != Some(true)
            || prior.revealed != next.revealed
            || prior.committed != next.committed
        {
            return None;
        }
    } else {
        // An automatic ST7 operation can lead directly to the captured ST8
        // cleanup. Match that one completed test, not an arbitrary later view.
        let result = after.completed_tests.last()?;
        if after.completed_tests.len() != before.completed_tests.len() + 1
            || result.tester != prior.tester
            || result.source != "Investigate"
            || Some(result.total) != prior.total
            || result.difficulty != prior.difficulty
            || !result.success
        {
            return None;
        }
    }
    let index = prior.tester.index();
    let previous = before.players[index].clues;
    let amount = after.players[index].clues.checked_sub(previous)?;
    let source = before
        .scenario
        .locations
        .iter()
        .find(|room| room.code == location)?;
    let end_source = after
        .scenario
        .locations
        .iter()
        .find(|room| room.code == location)?;
    if amount == 0
        || !source.revealed
        || !end_source.revealed
        || source.clues.checked_sub(end_source.clues) != Some(amount)
    {
        return None;
    }
    let map = crate::tabletop_gathering_map::GatheringMap::new(
        after
            .scenario
            .locations
            .iter()
            .map(|room| (room.code.as_str(), room.revealed)),
        Vec2::new(1.35, 1.89),
    )
    .ok()?;
    Some(Clue {
        investigator: index,
        previous,
        amount,
        start: layout::location_clue(map.position(location)),
        end: layout::investigator_clue(index),
        elapsed: 0.,
    })
}

#[derive(Component)]
pub(super) struct Owned;
#[derive(Component)]
pub(super) struct MovingClue {
    label: Entity,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn advance(
    mut commands: Commands,
    mut state: ResMut<State>,
    mut table: ResMut<Table>,
    health: Res<RenderHealth>,
    time: Res<PresentationTime>,
    art: Res<art::Art>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut returning: Query<(Entity, &mut Transform, &mut tokens::ReturningToken), With<Owned>>,
    mut moving: Query<(Entity, &MovingClue, &mut Transform), Without<tokens::ReturningToken>>,
) {
    if table.reduced_motion || false {
        state.clear();
    }
    let paused = !health.allows_play() || table.motion_blocked();
    for departure in std::mem::take(&mut state.departures) {
        let (root, label) = tokens::spawn(
            &mut commands,
            &mut meshes,
            &mut materials,
            tokens::TokenVisual {
                face: departure.face,
                status: departure.status,
                ordinal: None,
                image: art.image(tokens::image_role(departure.face)),
                transform: departure.start,
                label_offset: Vec3::ZERO,
            },
        );
        commands.entity(label).despawn();
        commands.entity(root).insert((
            Owned,
            tokens::ReturningToken::new(departure.start.translation, departure.end)
                .with_scale(departure.start.scale),
        ));
        state.returning += 1;
    }
    for (entity, mut transform, mut token) in &mut returning {
        if state.returning == 0
            || (!paused && token.advance(&mut transform, time.delta_secs(), table.reduced_motion))
        {
            commands.entity(entity).despawn();
            state.returning = state.returning.saturating_sub(1);
            table.dirty = true;
        }
    }
    let Some(clue) = state.clue.as_mut() else {
        for (entity, clue, _) in &moving {
            commands.entity(entity).despawn();
            commands.entity(clue.label).try_despawn();
        }
        return;
    };
    if !paused {
        clue.elapsed += time.delta_secs();
    }
    let fraction = (clue.elapsed / 0.8).clamp(0., 1.);
    if fraction >= 1. {
        state.clue = None;
        for (entity, clue, _) in &moving {
            commands.entity(entity).despawn();
            commands.entity(clue.label).try_despawn();
        }
        table.dirty = true;
        return;
    }
    let at = counters::clue_transfer_position(clue.start, clue.end, fraction);
    let scale = 1. - 0.15 * fraction;
    if moving.is_empty() {
        let (root, label) = counters::spawn_count(
            &mut commands,
            &mut meshes,
            &mut materials,
            counters::CounterVisual {
                kind: counters::CounterKind::Clue,
                amount: clue.amount,
                image: art.image("token/clue"),
                at,
                scale,
            },
        )
        .expect("discovery moves a positive count");
        commands.entity(root).insert((Owned, MovingClue { label }));
    } else {
        for (_, _, mut transform) in &mut moving {
            transform.translation = at;
            transform.scale = Vec3::splat(scale);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gathering::{HostileFixture, TestCardFixture, TokenFixture};

    fn step(session: &mut Session, views: &mut Vec<Observation>, id: &str) {
        let revision = session.observe().revision;
        session
            .choose(revision, id)
            .unwrap_or_else(|error| panic!("{id}: {error:?}"));
        views.extend(
            session
                .presentation_after(revision)
                .iter()
                .map(|moment| session.observe_presentation(moment)),
        );
        views.push(session.observe());
    }

    fn investigation() -> (Session, Vec<Observation>) {
        let mut session =
            Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::EyesOlive));
        let mut views = vec![session.observe()];
        let assets = &views[0].play.as_ref().unwrap().players[2].assets;
        let eyes = assets
            .iter()
            .find(|asset| asset.card.code == "11067")
            .unwrap()
            .card
            .id;
        let olive = assets
            .iter()
            .find(|asset| asset.card.code == "04197")
            .unwrap()
            .card
            .id;
        for id in [
            format!("action:ordinary:investigate:spell:{eyes}"),
            "window:pass".into(),
            "commit:confirm".into(),
            "window:pass".into(),
            "tokens:eyes:1".into(),
            format!("tokens:olive:0:{olive}"),
            "test:reveal".into(),
            "tokens:position:1".into(),
            "tokens:confirm".into(),
        ] {
            step(&mut session, &mut views, &id);
        }
        (session, views)
    }

    #[test]
    fn real_investigation_moves_matching_source_clues_and_holds_recipient_until_arrival() {
        let (_, views) = investigation();
        let pair = views
            .windows(2)
            .find(|pair| discovery(&pair[0], &pair[1]).is_some())
            .expect("actual successful investigation has a public discovery boundary");
        let mut state = State::default();
        state.prepare(Some(&pair[0]), &pair[1], true, false);
        let transfer = state.clue.as_ref().unwrap();
        let current = pair[1].play.as_ref().unwrap().players[2].clues;
        assert_eq!(transfer.amount, 2);
        assert_eq!(state.shown_clues(2, current), current - 2);
        assert_eq!(transfer.end, layout::investigator_clue(2));
        let mut unknown = pair[0].clone();
        unknown
            .play
            .as_mut()
            .unwrap()
            .test
            .as_mut()
            .unwrap()
            .investigation_location = None;
        assert!(discovery(&unknown, &pair[1]).is_none());
        let mut unrelated = pair[0].clone();
        unrelated
            .play
            .as_mut()
            .unwrap()
            .test
            .as_mut()
            .unwrap()
            .investigation_location = Some("01113".into());
        assert!(discovery(&unrelated, &pair[1]).is_none());
        state.clear();
        assert_eq!(state.shown_clues(2, current), current);
        for (live, reduced) in [(false, false), (true, true)] {
            let mut state = State::default();
            state.prepare(Some(&pair[0]), &pair[1], live, reduced);
            assert!(!state.active());
        }
    }

    #[test]
    fn installed_clue_motion_preserves_saved_state_and_defers_recipient_counter_until_arrival() {
        let (session, views) = investigation();
        let pair = views
            .windows(2)
            .find(|pair| discovery(&pair[0], &pair[1]).is_some())
            .unwrap();
        let mut state = State::default();
        state.prepare(Some(&pair[0]), &pair[1], true, false);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("clue-motion.json");
        let mut app = super::super::tests::host(path.clone());
        app.insert_resource(Table::new(session.clone(), path.clone()))
            .insert_resource(state);
        app.update();
        let world = app.world_mut();
        assert_eq!(world.query::<&MovingClue>().iter(world).count(), 1);
        assert!(
            !world
                .query::<(&counters::Counter, &Transform)>()
                .iter(world)
                .any(
                    |(counter, transform)| counter.kind == counters::CounterKind::Clue
                        && transform.translation == layout::investigator_clue(2)
                )
        );
        let before = world.resource::<Table>().session.clone();
        world.insert_resource(RenderHealth::failed_for_test());
        world
            .resource_mut::<PresentationTime>()
            .advance_by(std::time::Duration::from_millis(400));
        app.update();
        assert_eq!(
            app.world()
                .resource::<State>()
                .clue
                .as_ref()
                .unwrap()
                .elapsed,
            0.
        );
        app.insert_resource(RenderHealth::ready_for_test());
        app.world_mut()
            .resource_mut::<PresentationTime>()
            .advance_by(std::time::Duration::ZERO);
        let epoch = app.world().resource::<Table>().epoch;
        app.world_mut()
            .resource_mut::<super::super::Actions>()
            .0
            .push(super::super::Action::ContinueResult(epoch));
        app.update();
        app.world_mut()
            .resource_mut::<PresentationTime>()
            .advance_by(std::time::Duration::from_millis(400));
        app.update();
        assert!(app.world().resource::<State>().active());
        app.world_mut()
            .resource_mut::<PresentationTime>()
            .advance_by(std::time::Duration::from_millis(400));
        app.update();
        let world = app.world_mut();
        assert!(!world.resource::<State>().active());
        assert_eq!(world.query::<&MovingClue>().iter(world).count(), 0);
        assert!(
            world
                .query::<(&counters::Counter, &Transform)>()
                .iter(world)
                .any(
                    |(counter, transform)| counter.kind == counters::CounterKind::Clue
                        && counter.amount == 2
                        && transform.translation == layout::investigator_clue(2)
                )
        );
        assert_eq!(world.resource::<Table>().session.clone(), before);
        assert!(
            !path.exists(),
            "visual arrival does not save a rules transition"
        );
    }

    #[test]
    fn actual_test_cleanup_returns_tokens_only_after_recorded_completion() {
        let mut session =
            Session::with_hostile_fixture(HostileFixture::TestCards(TestCardFixture::PromiseCurse));
        let mut views = vec![session.observe()];
        let promise = views[0].play.as_ref().unwrap().players[1]
            .hand
            .iter()
            .find(|card| card.code == "07032")
            .unwrap()
            .id;
        for id in [
            "window:pass".into(),
            format!("commit:{promise}"),
            "commit:confirm".into(),
        ] {
            step(&mut session, &mut views, &id);
        }
        for _ in 0..8 {
            let view = session.observe();
            if view.choices.iter().any(|choice| choice.id == "test:reveal") {
                break;
            }
            let id = ["window:pass", "commit:confirm"]
                .into_iter()
                .find(|id| view.choices.iter().any(|choice| choice.id == *id))
                .unwrap();
            step(&mut session, &mut views, id);
        }
        step(&mut session, &mut views, "test:reveal");
        step(&mut session, &mut views, "test:reveal");
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("token-return.json");
        let mut app = super::super::tests::host(path.clone());
        app.insert_resource(Table::new(session, path));
        app.update();
        let world = app.world_mut();
        let pool = world
            .query::<(&Name, &Transform)>()
            .iter(world)
            .find_map(|(name, transform)| {
                (name.as_str() == "Curse pool").then_some(transform.translation)
            })
            .expect("the production scene renders the curse pool");
        let mut state = State::default();
        let mut departures = 0;
        let mut curses = 0;
        for pair in views.windows(2) {
            state.prepare(Some(&pair[0]), &pair[1], true, false);
            if !state.departures.is_empty() {
                assert_eq!(
                    pair[1].play.as_ref().unwrap().completed_tests.len(),
                    pair[0].play.as_ref().unwrap().completed_tests.len() + 1
                );
                departures += state.departures.len();
                for token in &state.departures {
                    if token.face == crate::TokenFace::Curse {
                        curses += 1;
                        assert_eq!(
                            token.end, pool,
                            "resolved curse must return to the rendered pool"
                        );
                    } else {
                        assert_eq!(token.end, layout::test_bag());
                    }
                }
                state.departures.clear();
            }
        }
        assert_eq!(departures, 2);
        assert_eq!(curses, 1);
    }
}
