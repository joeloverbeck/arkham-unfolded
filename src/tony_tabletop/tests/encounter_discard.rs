use super::*;

#[test]
fn defeated_ghoul_is_a_single_inspectable_encounter_discard_card() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("discard.json");
    std::fs::write(
        &path,
        include_bytes!("../../../tests/fixtures/encounter-discard/after-defeat.session.json"),
    )
    .unwrap();
    let mut app = presentation_app(path.clone());
    app.world_mut().resource_mut::<Table>().session = continuity::load_session(&path).unwrap();
    advance(&mut app, 0.0);
    let world = app.world_mut();
    let sources = world
        .query::<(&CardSource, &Transform)>()
        .iter(world)
        .map(|(source, transform)| (source.0, transform.translation))
        .collect::<Vec<_>>();
    assert!(!sources.iter().any(|(source, _)| *source == Source::Card(2)));
    assert_eq!(
        sources
            .iter()
            .filter(|(source, _)| *source == Source::EncounterDiscard)
            .count(),
        1
    );
    let (_, position) = sources
        .iter()
        .find(|(source, _)| *source == Source::EncounterDiscard)
        .unwrap();
    assert_eq!(*position, crate::tabletop_discard::ENCOUNTER_DISCARD_AT);
    assert!(visible_text(&mut app).iter().any(
        |text| text.starts_with("Encounter discard") && text.ends_with("1 card\nGhoul Minion")
    ));
    let before = app.world().resource::<Table>().session.clone();
    let revision = before.observe().revision;
    act(
        &mut app,
        Action::Select(
            revision,
            Source::EncounterDiscard,
            crate::tabletop_discard::ENCOUNTER_DISCARD_AT,
        ),
    );
    assert_eq!(app.world().resource::<Table>().inspect, Some(2));
    assert_eq!(app.world().resource::<Table>().session, before);
    act(&mut app, Action::Close);
    act(&mut app, Action::Replay);
    assert!(
        visible_text(&mut app)
            .iter()
            .any(|text| text.starts_with("Encounter discard\nEmpty") && text.ends_with("0 cards"))
    );
    while app
        .world()
        .resource::<Table>()
        .replay
        .as_ref()
        .unwrap()
        .index()
        + 1
        < app
            .world()
            .resource::<Table>()
            .replay
            .as_ref()
            .unwrap()
            .len()
    {
        act(&mut app, Action::Next);
    }
    let replay_revision = app.world().resource::<Table>().view().revision;
    act(
        &mut app,
        Action::Select(
            replay_revision,
            Source::EncounterDiscard,
            crate::tabletop_discard::ENCOUNTER_DISCARD_AT,
        ),
    );
    assert_eq!(app.world().resource::<Table>().inspect, Some(2));
    assert_eq!(app.world().resource::<Table>().session, before);
}
