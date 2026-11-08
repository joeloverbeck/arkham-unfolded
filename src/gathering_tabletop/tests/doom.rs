use super::*;
use crate::tabletop_counters::{Counter, CounterKind};

#[test]
fn altar_keeps_agenda_doom_separate_from_quarry_and_automatic_mythos_clears_both() {
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringDoomThreshold);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("doom.json");
    let mut app = host_with_table(Table::new(session.clone(), path.clone()), false);
    let world = app.world_mut();
    assert!(
        world
            .query::<&Name>()
            .iter(world)
            .any(|name| name.as_str() == "Agenda doom altar")
    );
    let counts: Vec<_> = world
        .query::<(&Counter, &Transform)>()
        .iter(world)
        .filter(|(counter, _)| counter.kind == CounterKind::Doom)
        .map(|(counter, at)| (counter.amount, at.translation))
        .collect();
    assert_eq!(counts.len(), 2);
    assert!(counts.iter().all(|(amount, _)| *amount == 1));
    assert!(
        counts
            .iter()
            .any(|(_, at)| at.x.abs() < 0.1 && (at.z + 18.4).abs() < 0.1)
    );
    assert!(rendered_text(&mut app).contains("Total doom: 2/3"));
    let mut seen_three = false;
    let mut seen_local_clear = false;
    let quarry_at = play::enemy_position(&session.observe().play.unwrap(), 0);
    for _ in 0..100 {
        draws::tick(&mut app, 0.1);
        seen_three |= rendered_text(&mut app).contains("Total doom: 3/3");
        let world = app.world_mut();
        let moving: Vec<_> = world
            .query_filtered::<&Transform, With<super::super::doom::Moving>>()
            .iter(world)
            .map(|at| at.translation)
            .collect();
        if moving.len() == 2 {
            assert!(
                moving
                    .iter()
                    .any(|at| at.x.abs() < 0.1 && (at.z + 18.4).abs() < 0.1)
            );
            assert!(
                moving
                    .iter()
                    .any(|at| (at.x - quarry_at.x - 0.67).abs() < 0.1
                        && (at.z - quarry_at.z).abs() < 0.1)
            );
            assert!(
                app.world()
                    .resource::<Table>()
                    .presentation
                    .story()
                    .is_none(),
                "Physical clearing precedes the reverse-side reading"
            );
            seen_local_clear = true;
        }
        if app
            .world()
            .resource::<Table>()
            .presentation
            .story()
            .is_some()
        {
            break;
        }
    }
    assert!(
        seen_three,
        "The added agenda doom arrives before the threshold clears it"
    );
    assert!(seen_local_clear, "Each bearer clears its own doom");
    let table = app.world().resource::<Table>();
    let committed = table.session.clone();
    let play = committed.observe().play.unwrap();
    assert_eq!(play.agenda_doom, 0);
    assert!(play.scenario.enemies.iter().all(|enemy| enemy.doom == 0));
    assert_eq!(
        play.encounter_deck_count,
        session.observe().play.unwrap().encounter_deck_count
    );
    assert!(
        committed
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "agenda:lead-horror")
    );
    assert_eq!(continuity::load_session(path).unwrap(), committed);
    let world = app.world_mut();
    assert!(
        !world
            .query::<&Counter>()
            .iter(world)
            .any(|counter| counter.kind == CounterKind::Doom)
    );
    assert!(rendered_text(&mut app).contains("Total doom: 0/3"));
}

#[test]
fn zero_doom_keeps_the_altar_and_reduced_motion_uses_the_same_committed_result() {
    let mut session = Session::with_seed(89);
    for id in [
        "lead:tony",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
    ] {
        session.choose(session.observe().revision, id).unwrap();
    }
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session, directory.path().join("zero.json")),
        false,
    );
    let world = app.world_mut();
    assert!(
        world
            .query::<&Name>()
            .iter(world)
            .any(|name| name.as_str() == "Agenda doom altar")
    );
    assert!(
        !world
            .query::<&Counter>()
            .iter(world)
            .any(|counter| counter.kind == CounterKind::Doom)
    );
    assert!(rendered_text(&mut app).contains("Agenda doom: 0\nTotal doom: 0/3"));
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringDoomThreshold);
    let mut expected = session.clone();
    expected
        .choose(expected.observe().revision, "mythos:begin")
        .unwrap();
    let mut table = Table::new(session, directory.path().join("reduced.json"));
    table.reduced_motion = true;
    let mut app = host_with_table(table, false);
    for _ in 0..30 {
        draws::tick(&mut app, 0.1);
    }
    assert_eq!(app.world().resource::<Table>().session, expected);
    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<Entity, With<super::super::doom::Moving>>()
            .iter(world)
            .count(),
        0
    );
    assert!(rendered_text(&mut app).contains("Total doom: 0/3"));
}
