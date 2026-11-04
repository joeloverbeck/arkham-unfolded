use super::*;

#[test]
fn resolved_treachery_is_a_single_inspectable_encounter_discard_card() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("discard.json");
    let mut session = crate::Session::new(crate::Fixture::curse());
    for choice in ["commit-promise", "use-jacqueline", "cancel-1-2"] {
        let revision = session.observe().revision;
        session.choose(revision, choice).unwrap();
    }
    crate::continuity::save_session(&path, &session).unwrap();
    let session = crate::continuity::load_session(&path).unwrap();
    let mut app = app_for_session(session.clone(), path);
    app.init_resource::<Assets<Image>>();
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    app.world_mut().resource_mut::<Art>().handles[0] = Some(image.clone());
    app.update();
    let world = app.world_mut();
    let cards = world
        .query::<(&context::Selectable, &Transform)>()
        .iter(world)
        .filter(|(source, _)| source.0 == Source::Card(0))
        .map(|(_, transform)| transform.translation)
        .collect::<Vec<_>>();
    assert_eq!(cards, [crate::tabletop_discard::ENCOUNTER_DISCARD_AT]);
    let mut materials = world.query::<(&context::Selectable, &MeshMaterial3d<StandardMaterial>)>();
    let (_, material) = materials
        .iter(world)
        .find(|(source, _)| source.0 == Source::Card(0))
        .unwrap();
    assert_eq!(
        world
            .resource::<Assets<StandardMaterial>>()
            .get(material)
            .unwrap()
            .base_color_texture,
        Some(image)
    );
    assert!(
        world
            .query::<&Text>()
            .iter(world)
            .any(|text| text.0.starts_with("Encounter discard")
                && text.0.ends_with("1 card\nRotting Remains"))
    );
    act(
        &mut app,
        Action::Select(Source::Card(0), Vec2::new(200.0, 200.0)),
    );
    assert_eq!(app.world().resource::<Tabletop>().inspect, Some(0));
    assert_eq!(app.world().resource::<Tabletop>().session, session);
    act(&mut app, Action::CloseInspect);
    assert_eq!(app.world().resource::<Tabletop>().inspect, None);
}
