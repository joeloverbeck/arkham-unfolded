use super::super::location_map::ConnectionVisual;
use super::*;

fn connection_entities(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    let mut entities: Vec<_> = world
        .query_filtered::<Entity, With<ConnectionVisual>>()
        .iter(world)
        .collect();
    entities.sort();
    entities
}

fn rendered_connections(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    let mut names: Vec<_> = world
        .query::<&Name>()
        .iter(world)
        .filter(|name| name.as_str().starts_with("Location connection "))
        .map(|name| name.as_str().to_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn production_scene_draws_only_the_three_reciprocal_hallway_connections() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("connections.json");
    let (mut app, _) =
        selection_lifecycle::production_app(path.clone(), RenderHealth::ready_for_test());
    assert!(rendered_connections(&mut app).is_empty());
    app.insert_resource(Table::new(completed_attempt(Some(true)), path));
    app.update();
    assert_eq!(
        rendered_connections(&mut app),
        [
            "Location connection Hallway ↔ Attic",
            "Location connection Hallway ↔ Cellar",
            "Location connection Hallway ↔ Parlor",
        ]
    );
    assert!(
        visible_text(&mut app)
            .iter()
            .any(|text| text.contains("Entry blocked"))
    );
    let entities = connection_entities(&mut app);
    assert_eq!(entities.len(), 3);
    for entity in &entities {
        let world = app.world();
        let root = world.entity(*entity);
        let pickable = root.get::<Pickable>().unwrap();
        assert!(!pickable.should_block_lower && !pickable.is_hoverable);
        let children = root.get::<Children>().unwrap();
        assert_eq!(children.len(), 2, "each path has a dark rim and brass fill");
        for child in children {
            let child = world.entity(*child);
            let pickable = child.get::<Pickable>().unwrap();
            assert!(!pickable.should_block_lower && !pickable.is_hoverable);
            assert!(child.get::<Mesh3d>().is_some());
        }
    }
    act(&mut app, Action::Focus(Area::Scenario));
    assert_eq!(
        connection_entities(&mut app),
        entities,
        "camera movement does not rebuild the graph"
    );
    act(&mut app, Action::Save);
    act(&mut app, Action::Reopen);
    assert_eq!(
        rendered_connections(&mut app).len(),
        3,
        "reopen rebuilds without duplicate paths"
    );
    let live = app.world().resource::<Table>().session.clone();
    act(&mut app, Action::Replay);
    assert!(
        rendered_connections(&mut app).is_empty(),
        "initial replay Study does not expose future rooms"
    );
    loop {
        let table = app.world().resource::<Table>();
        let replay = table.replay.as_ref().unwrap();
        if replay.index() + 1 == replay.len() {
            break;
        }
        act(&mut app, Action::Next);
        let escaped = app.world().resource::<Table>().view().endpoint
            == Some(crate::party::Endpoint::Escaped);
        assert_eq!(
            rendered_connections(&mut app).len(),
            if escaped { 3 } else { 0 }
        );
    }
    assert_eq!(rendered_connections(&mut app).len(), 3);
    act(&mut app, Action::Back);
    assert!(
        rendered_connections(&mut app).is_empty(),
        "Act reverse acknowledgement precedes the new map"
    );
    act(&mut app, Action::Live);
    assert_eq!(rendered_connections(&mut app).len(), 3);
    assert_eq!(app.world().resource::<Table>().session, live);
}
