use super::*;

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
