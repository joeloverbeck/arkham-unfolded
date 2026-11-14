//! Reach a named live control through only the reading stops the test expects.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReadingStop {
    Result,
    Story(story::StoryCard),
}

/// Advance the declared reading route through registered controls, then require
/// one direct choice. An empty route requires an already unobstructed decision.
pub(super) fn reach_live_choice(
    app: &mut App,
    readings: &[ReadingStop],
    expected_choice: &str,
) -> Entity {
    for expected in readings {
        settle_automatic_resolution(app);
        let table = app.world().resource::<Table>();
        assert_no_other_dialog(table);
        let current = table
            .presentation
            .result()
            .map(|_| ReadingStop::Result)
            .or_else(|| table.presentation.story().map(ReadingStop::Story));
        assert_eq!(current, Some(*expected), "Unexpected reading stop");
        let before = table.session.clone();
        let entity =
            choice_button(app, "Continue").expect("Expected reading has a Continue control");
        app.world_mut().trigger(Activate { entity });
        app.update();
        assert_eq!(
            app.world().resource::<Table>().session,
            before,
            "Acknowledging a reading must not choose a rules option"
        );
    }
    settle_automatic_resolution(app);
    let table = app.world().resource::<Table>();
    assert_no_other_dialog(table);
    assert!(
        !table.modal(),
        "Unexpected reading stop before {expected_choice}"
    );
    let world = app.world_mut();
    let controls: Vec<_> = world
        .query_filtered::<(Entity, &ChoiceControl), With<Button>>()
        .iter(world)
        .filter(|(_, choice)| choice.0 == expected_choice)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(
        controls.len(),
        1,
        "Expected one live control for {expected_choice}"
    );
    controls[0]
}

fn assert_no_other_dialog(table: &Table) {
    assert!(
        table.inspection.is_none()
            && !table.history_open
            && !table.restart_confirmation
            && !table.menu_open
            && !table.context.is_open()
            && table.prediction.is_none()
            && table.violin.is_none()
            && table.clue_payment.is_none()
            && table.presentation.encounter().current().is_none(),
        "Unexpected dialog while preparing live controls"
    );
}

fn harm_table() -> (tempfile::TempDir, App) {
    use crate::gathering::{HarmFixture, HostileFixture};
    let mut session = Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::DenyTablet));
    session
        .choose(session.revision(), "harm:damage:investigator")
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session, directory.path().join("harm.json")),
        false,
    );
    settle_automatic_resolution(&mut app);
    (directory, app)
}

fn result_table() -> (tempfile::TempDir, App) {
    let directory = tempfile::tempdir().unwrap();
    let app = host_with_table(
        Table::new(
            results::quick_thinking_session(),
            directory.path().join("result.json"),
        ),
        false,
    );
    (directory, app)
}

#[test]
fn unexpected_reading_remains_open() {
    let (_directory, mut app) = result_table();
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .result()
            .is_some()
    );
    let before = app.world().resource::<Table>().session.clone();
    let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        reach_live_choice(&mut app, &[], "turn:end");
    }));
    assert!(rejected.is_err());
    let table = app.world().resource::<Table>();
    assert!(table.presentation.result().is_some());
    assert_eq!(table.session, before);
}

#[test]
fn unexpected_inspection_is_not_dismissed_as_a_reading() {
    let (_directory, mut app) = result_table();
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.inspection = Some(("Unexpected inspection".into(), "Keep this open".into()));
        table.dirty = true;
    }
    app.update();
    let before = app.world().resource::<Table>().session.clone();
    let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        reach_live_choice(&mut app, &[ReadingStop::Result], "turn:end");
    }));
    assert!(rejected.is_err());
    let table = app.world().resource::<Table>();
    assert!(table.inspection.is_some());
    assert!(table.presentation.result().is_some());
    assert_eq!(table.session, before);
}

#[test]
fn missing_expected_control_fails_without_applying_harm() {
    let (_directory, mut app) = harm_table();
    let before = app.world().resource::<Table>().session.clone();
    let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        reach_live_choice(&mut app, &[], "unavailable:choice");
    }));
    assert!(rejected.is_err());
    let table = app.world().resource::<Table>();
    assert!(
        !table.modal(),
        "The harm fixture is already at a live decision"
    );
    assert_eq!(table.session, before);
}

#[test]
fn result_reaction_is_not_silently_declined() {
    let (_directory, mut app) = result_table();
    let before = app.world().resource::<Table>().session.clone();
    let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        reach_live_choice(&mut app, &[ReadingStop::Result], "turn:end");
    }));
    assert!(rejected.is_err());
    let table = app.world().resource::<Table>();
    assert!(
        table.presentation.result().is_none(),
        "The declared reading was acknowledged"
    );
    assert!(
        table.result_opportunities(),
        "The optional reaction still awaits a player choice"
    );
    assert_eq!(table.session, before);
    assert!(button_with_label(&mut app, "Decline Quick Thinking").is_some());
}
