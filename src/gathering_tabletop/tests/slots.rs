use super::*;

#[test]
fn capacity_is_printed_on_the_mat_and_restricted_capacity_updates_after_play() {
    use crate::gathering::{AssetFixture, HostileFixture};
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
    let planes = world
        .query::<(&Name, &Mesh3d, &Transform)>()
        .iter(world)
        .filter(|(name, _, _)| name.as_str() == "Asset slot strip")
        .count();
    assert!(
        planes > 0,
        "Slot text must be printed on a physical surface"
    );
}
