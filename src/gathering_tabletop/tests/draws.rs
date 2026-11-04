use super::*;
use crate::gathering_tabletop::draws::DURATION;

fn tick(app: &mut App, seconds: f32) {
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_secs_f32(seconds));
    app.update();
}

fn physical(app: &mut App, id: u16) -> Transform {
    let world = app.world_mut();
    let positions = world
        .query::<(&motion::PhysicalCard, &Transform)>()
        .iter(world)
        .filter(|(card, _)| card.0 == id)
        .map(|(_, transform)| *transform)
        .collect::<Vec<_>>();
    assert_eq!(
        positions.len(),
        1,
        "One physical piece represents card {id}"
    );
    positions[0]
}

fn back_rotation(app: &mut App, id: u16) -> Quat {
    let face = physical(app, id);
    let world = app.world_mut();
    let back = world
        .query::<(&Name, &Transform, &ChildOf)>()
        .iter(world)
        .find_map(|(name, transform, parent)| {
            (name.as_str() == "Drawn card back"
                && world
                    .get::<motion::PhysicalCard>(parent.parent())
                    .is_some_and(|card| card.0 == id))
            .then_some(transform.rotation)
        })
        .expect("The moving card has its physical back attached");
    face.rotation * back
}

fn assert_right_to_left_draw(app: &mut App, id: u16) {
    // card_shape maps the top of either image to -Z and its right edge to +X.
    let back = back_rotation(app, id);
    assert!((back * Vec3::Y).y > 0.99, "The back starts face-up");
    assert!((back * Vec3::NEG_Z).dot(Vec3::NEG_Z) > 0.99);
    assert!((back * Vec3::X).x > 0.99);

    tick(app, DURATION * 0.5);
    let face = physical(app, id).rotation;
    let top = face * Vec3::NEG_Z;
    assert!(
        top.dot(Vec3::NEG_Z) > 0.98,
        "The card turns sideways while its top keeps its heading: {top:?}"
    );
    let back_right = back_rotation(app, id) * Vec3::X;
    assert!(
        back_right.y > 0.9,
        "The back's right edge lifts over toward the left: {back_right:?}"
    );
    assert!((face * Vec3::Y).x > 0.9);

    tick(app, DURATION * 0.5);
    let face = physical(app, id).rotation;
    assert!((face * Vec3::Y).y > 0.99, "The face lands upward");
    assert!((face * Vec3::NEG_Z).dot(Vec3::NEG_Z) > 0.99);
    assert!((face * Vec3::X).x > 0.99);
}

#[test]
fn investigator_draw_flips_right_to_left_with_both_faces_upright() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = host(dir.path().join("sideways-draw.json"));
    activate_choice(&mut app, "lead:tony");
    let id = app.world().resource::<Table>().view().investigators[0].hand[0].id;
    assert_right_to_left_draw(&mut app, id);
}

#[test]
fn encounter_draw_flips_right_to_left_with_both_faces_upright() {
    use crate::gathering::{HostileFixture, SearchFixture};
    let dir = tempfile::tempdir().unwrap();
    let session = Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HuntTargets));
    let mut app = host_with_table(
        Table::new(session, dir.path().join("sideways-encounter.json")),
        false,
    );
    activate_choice(&mut app, "mythos:draw");
    activate_choice(&mut app, "search:decline-hunt");
    let view = app.world().resource::<Table>().view();
    let id = view.play.unwrap().pending_revelation.unwrap().card.id;
    assert_right_to_left_draw(&mut app, id);
}

#[test]
fn opening_deal_is_serial_from_actual_decks_and_pauses_without_restarting() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("opening.json");
    let mut app = host(path.clone());
    let camera = app.world().resource::<Orbit>().pose();
    activate_choice(&mut app, "lead:jacqueline");
    let committed = continuity::load_session(&path).unwrap();
    let first = committed.observe().investigators[2].hand[0].clone();
    let view = app.world().resource::<Table>().view();
    assert_eq!(
        view.investigators
            .iter()
            .map(|p| p.hand.len())
            .collect::<Vec<_>>(),
        [0, 0, 1]
    );
    assert!(view.choices.is_empty());
    let start = physical(&mut app, first.id);
    assert!((start.translation.x - layout::investigator_deck(2).x).abs() < 0.001);
    assert!(
        (start.rotation * Vec3::Y).y < -0.99,
        "The actual card back starts upward"
    );
    tick(&mut app, 0.25);
    let lifted = physical(&mut app, first.id);
    assert!(lifted.translation.y > start.translation.y);
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Inspect(first.title, first.code));
    app.update();
    let paused = physical(&mut app, first.id);
    for _ in 0..5 {
        tick(&mut app, 0.2);
    }
    assert_eq!(physical(&mut app, first.id), paused);
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Close);
    tick(&mut app, 0.05);
    assert_ne!(physical(&mut app, first.id), start);
    let mut previous = 1;
    let mut counts = vec![0, 0, 1];
    let mut arrived = vec![2];
    for _ in 0..160 {
        tick(&mut app, 0.1);
        let table = app.world().resource::<Table>();
        let view = table.view();
        let total: usize = view.investigators.iter().map(|p| p.hand.len()).sum();
        assert!(total == previous || total == previous + 1);
        if total > previous {
            let owner = view
                .investigators
                .iter()
                .enumerate()
                .position(|(index, player)| player.hand.len() > counts[index])
                .unwrap();
            arrived.push(owner);
        }
        counts = view
            .investigators
            .iter()
            .map(|player| player.hand.len())
            .collect();
        previous = total;
        if !table.is_presenting() {
            break;
        }
    }
    assert_eq!(arrived, [vec![2; 5], vec![0; 5], vec![1; 5]].concat());
    assert_eq!(
        app.world().resource::<Table>().view(),
        committed.observe_for_play()
    );
    assert_eq!(app.world().resource::<Orbit>().pose(), camera);
    assert_eq!(continuity::load_session(path).unwrap(), committed);
}

#[test]
fn mulligan_moves_rejected_cards_out_deals_replacements_then_returns_them() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = host(dir.path().join("mulligan.json"));
    activate_choice(&mut app, "lead:tony");
    finish_presentation(&mut app);
    let before = app.world().resource::<Table>().view();
    let rejected = before.investigators[0].hand[..2].to_vec();
    for card in &rejected {
        activate_choice(&mut app, &format!("mulligan:{}", card.id));
    }
    let old_at = physical(&mut app, rejected[0].id);
    activate_choice(&mut app, "confirm-mulligan");
    assert_eq!(
        physical(&mut app, rejected[0].id).translation,
        old_at.translation
    );
    tick(&mut app, 0.25);
    let aside_entity = {
        let world = app.world_mut();
        world
            .query::<(Entity, &motion::PhysicalCard)>()
            .iter(world)
            .find(|(_, card)| card.0 == rejected[0].id)
            .unwrap()
            .0
    };
    click_mesh_observer(&mut app, aside_entity);
    app.update();
    assert_eq!(
        app.world()
            .resource::<Table>()
            .inspection
            .as_ref()
            .map(|(_, code)| code),
        Some(&rejected[0].code),
        "A set-aside card remains inspectable through its actual pointer observer"
    );
    let paused = physical(&mut app, rejected[0].id);
    tick(&mut app, 0.3);
    assert_eq!(physical(&mut app, rejected[0].id), paused);
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Close);
    app.update();
    let target = app.world().resource::<Table>().session.observe_for_play();
    let replacement_ids: Vec<_> = target.investigators[0]
        .hand
        .iter()
        .filter(|c| {
            !before.investigators[0]
                .hand
                .iter()
                .any(|old| old.id == c.id)
        })
        .map(|c| c.id)
        .collect();
    assert_eq!(replacement_ids.len(), 2);
    let mut saw_replacement = false;
    let mut saw_return = false;
    for _ in 0..80 {
        tick(&mut app, 0.1);
        let table = app.world().resource::<Table>();
        let view = table.view();
        if view.investigators[0]
            .hand
            .iter()
            .any(|c| replacement_ids.contains(&c.id))
        {
            saw_replacement = true;
            assert!(
                !view.investigators[0]
                    .hand
                    .iter()
                    .any(|c| rejected.iter().any(|old| old.id == c.id))
            );
        }
        let returning = table.draws.returning(rejected[0].id).is_some();
        if returning {
            saw_return = true;
            assert!(
                replacement_ids
                    .iter()
                    .all(|id| view.investigators[0].hand.iter().any(|c| c.id == *id))
            );
        }
        if !table.is_presenting() {
            break;
        }
        if returning {
            let face = physical(&mut app, rejected[0].id).rotation;
            let back = back_rotation(&mut app, rejected[0].id);
            assert!((face * Vec3::NEG_Z).dot(Vec3::NEG_Z) > 0.98);
            assert!((back * Vec3::NEG_Z).dot(Vec3::NEG_Z) > 0.98);
        }
        let world = app.world_mut();
        let aside_captions = world
            .query::<&motion::CardLabel>()
            .iter(world)
            .filter_map(|label| world.get::<motion::PhysicalCard>(label.0))
            .filter(|physical| {
                rejected.iter().any(|card| card.id == physical.0)
                    && !view.investigators[0]
                        .hand
                        .iter()
                        .any(|card| card.id == physical.0)
            })
            .count();
        assert!(
            aside_captions <= usize::from(!returning),
            "The aside pile has one caption, and returning cards do not overlap the deck caption"
        );
    }
    assert!(saw_replacement && saw_return);
    assert_eq!(app.world().resource::<Table>().view(), target);
}

#[test]
fn an_asset_test_preview_never_shares_the_physical_cards_motion_identity() {
    let mut session = Session::with_hostile_fixture(
        crate::gathering::HostileFixture::SelectedWitness(crate::gathering::SelectedWitness::Mists),
    );
    let play = session.observe().play.unwrap();
    let asset = play.players[1]
        .assets
        .iter()
        .find(|asset| asset.card.code == "04029")
        .unwrap();
    let id = asset.card.id;
    let enemy = play
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01160")
        .unwrap()
        .card
        .id;
    session
        .choose(
            session.observe().revision,
            &format!("action:ordinary:evade:{enemy}:spell:{id}"),
        )
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("preview.json")), false);
    finish_presentation(&mut app);
    let original = physical(&mut app, id);
    let positions = |app: &mut App| {
        let world = app.world_mut();
        let mut positions = world
            .query::<(&ContextTarget, &Transform)>()
            .iter(world)
            .filter(|(target, _)| target.source == controls::Source::Card(id))
            .map(|(_, at)| at.translation)
            .collect::<Vec<_>>();
        positions.sort_by(|a, b| a.x.total_cmp(&b.x));
        positions
    };
    let settled = positions(&mut app);
    assert_eq!(
        settled.len(),
        2,
        "The asset and its Test-area reference are both visible"
    );
    assert!(settled.contains(&layout::test_source()));
    app.world_mut().resource_mut::<Table>().dirty = true;
    tick(&mut app, 0.1);
    assert_eq!(physical(&mut app, id), original);
    assert_eq!(
        positions(&mut app),
        settled,
        "Rebuilding preserves both settled positions"
    );
}

#[test]
fn reduced_motion_and_replay_keep_committed_setup_without_redrawing_rules() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replay.json");
    let mut app = host(path.clone());
    activate_choice(&mut app, "lead:tony");
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Motion);
    app.update();
    assert!(!app.world().resource::<Table>().draws.active());
    let saved = std::fs::read(&path).unwrap();
    let target = app.world().resource::<Table>().view();
    assert_eq!(
        target
            .investigators
            .iter()
            .map(|p| p.hand.len())
            .sum::<usize>(),
        15
    );
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::Motion,
        Action::Replay,
        Action::Next,
    ]);
    tick(&mut app, 0.);
    assert!(app.world().resource::<Table>().draws.active());
    for _ in 0..160 {
        tick(&mut app, 0.1);
    }
    assert!(!app.world().resource::<Table>().draws.active());
    assert_eq!(
        app.world().resource::<Table>().view().investigators,
        target.investigators
    );
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .extend([Action::Back, Action::Live]);
    tick(&mut app, 0.);
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    let fresh = Table::open(path).unwrap();
    assert!(!fresh.draws.active());
    assert_eq!(fresh.view(), target);
}

#[test]
fn encounter_card_leaves_its_deck_reveals_in_test_and_moves_to_its_actual_discard() {
    use crate::gathering::{HostileFixture, SearchFixture};
    let dir = tempfile::tempdir().unwrap();
    let session = Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HuntTargets));
    let mut app = host_with_table(
        Table::new(session, dir.path().join("encounter.json")),
        false,
    );
    activate_choice(&mut app, "mythos:draw");
    activate_choice(&mut app, "search:decline-hunt");
    let view = app.world().resource::<Table>().view();
    let card = view.play.unwrap().pending_revelation.unwrap().card;
    assert_eq!(
        card.code, "01118",
        "Flesh-Eater has no Attic to spawn at in this fixture"
    );
    let from = physical(&mut app, card.id);
    assert!((from.translation.x - layout::ENCOUNTER_DECK.x).abs() < 0.001);
    assert!((from.rotation * Vec3::Y).y < -0.99);
    let mut saw_reveal = false;
    let mut saw_departure = false;
    for _ in 0..80 {
        tick(&mut app, 0.1);
        let at = physical(&mut app, card.id);
        if at.translation.abs_diff_eq(layout::test_source(), 0.001) {
            saw_reveal = true;
            assert!((at.rotation * Vec3::Y).y > 0.99);
        }
        if app
            .world()
            .resource::<Table>()
            .view()
            .play
            .unwrap()
            .encounter_discard
            .iter()
            .any(|c| c.id == card.id)
            && !at.translation.abs_diff_eq(layout::ENCOUNTER_DISCARD, 0.001)
        {
            saw_departure = true;
        }
        if !app.world().resource::<Table>().is_presenting() {
            break;
        }
    }
    assert!(saw_reveal && saw_departure);
    assert!(
        physical(&mut app, card.id)
            .translation
            .abs_diff_eq(layout::ENCOUNTER_DISCARD, 0.001)
    );
}
