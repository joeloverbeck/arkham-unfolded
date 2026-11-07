use super::*;

#[test]
fn permanent_names_stay_flat_on_their_mats_through_camera_changes() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = host(directory.path().join("printed-names.json"));
    let initial: Vec<_> = app
        .world_mut()
        .query_filtered::<(Entity, &Name, &Transform), With<printed::PrintedLabel>>()
        .iter(app.world())
        .map(|(entity, name, at)| (entity, name.to_string(), *at))
        .collect();
    assert_eq!(initial.len(), 25);
    for (entity, _, at) in &initial {
        assert_eq!(at.rotation, Quat::IDENTITY);
        assert!((at.translation.y - 0.071).abs() < 0.001);
        assert!(app.world().get::<WorldLabel>(*entity).is_none());
        assert!(app.world().get::<Node>(*entity).is_none());
    }
    for area in [
        Area::Scenario,
        Area::Test,
        Area::Investigator(2),
        Area::Overview,
    ] {
        app.world_mut()
            .resource_mut::<Orbit>()
            .focus(layout::focus(area));
        for _ in 0..3 {
            app.update();
        }
        for (entity, name, at) in &initial {
            assert_eq!(app.world().get::<Transform>(*entity), Some(at), "{name}");
        }
    }
    let world = app.world_mut();
    assert!(
        !world
            .query::<(&WorldLabel, &Text)>()
            .iter(world)
            .any(
                |(_, text)| ["Scenario", "Test area", "Assets", "Hand", "Committed cards"]
                    .contains(&text.0.as_str())
            )
    );
}

#[test]
fn capacity_is_printed_on_the_mat_and_restricted_capacity_updates_after_play() {
    use crate::gathering::{AssetFixture, HostileFixture};
    use crate::tabletop_layout::heading_checks;
    let session =
        Session::with_hostile_fixture(HostileFixture::Assets(AssetFixture::SignMagickSlots));
    let sign = session.observe().play.unwrap().players[2]
        .hand
        .iter()
        .find(|card| card.code == "05112")
        .unwrap()
        .id;
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("slots.json")), true);
    heading_checks::viewport::<MainCamera>(&mut app, Vec2::new(1600., 1000.), 1.);
    app.world_mut()
        .resource_mut::<Orbit>()
        .focus(layout::focus(Area::Investigator(2)));
    app.update();
    assert!(rendered_text(&mut app).contains("Arcane: 2/2"));
    activate_choice(&mut app, &format!("fast-asset:{sign}"));
    finish_presentation(&mut app);
    app.update();
    let text = rendered_text(&mut app);
    assert!(
        text.contains("Arcane: 2/3") && text.contains("Spell/Ritual") && text.contains("Hand: 1/2"),
        "{text}"
    );
    let world = app.world_mut();
    let planes: Vec<_> = world
        .query::<(Entity, &Name, &Mesh3d, &GlobalTransform)>()
        .iter(world)
        .filter(|(_, name, _, _)| name.as_str() == "Asset slot strip")
        .map(|(entity, _, _, at)| (entity, *at))
        .collect();
    let gap = heading_checks::asset_hand_gap(2);
    let (strip, at) = planes
        .into_iter()
        .min_by(|(_, left), (_, right)| {
            (left.translation().x - gap.center().x)
                .abs()
                .total_cmp(&(right.translation().x - gap.center().x).abs())
        })
        .expect("Jacqueline's occupied slots need a printed surface");
    let position = at.translation();
    assert!(
        (position.x - gap.center().x).abs() < 0.001,
        "Center the strip on the mat"
    );
    assert!(
        position.y.abs() < 0.15,
        "The strip belongs on the mat, not above it"
    );
    assert!(
        at.affine()
            .transform_vector3(Vec3::Y)
            .normalize()
            .abs_diff_eq(Vec3::Y, 0.001),
        "The printed surface must lie flat"
    );
    let (camera, camera_at) = world
        .query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>()
        .single(world)
        .unwrap();
    let bounds = heading_checks::mesh_bounds(world, camera, camera_at, strip);
    let mut allowed = Rect {
        min: Vec2::splat(f32::INFINITY),
        max: Vec2::splat(f32::NEG_INFINITY),
    };
    for x in [gap.min.x, gap.max.x] {
        for z in [gap.min.y, gap.max.y] {
            let point = camera
                .world_to_viewport(camera_at, Vec3::new(x, position.y, z))
                .unwrap();
            allowed.min = allowed.min.min(point);
            allowed.max = allowed.max.max(point);
        }
    }
    assert!(
        allowed.contains(bounds.min) && allowed.contains(bounds.max),
        "The complete slot strip must fit between assets and hand: {bounds:?} in {allowed:?}"
    );
}
