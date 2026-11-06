use super::*;

#[test]
fn nested_token_selection_has_visible_source_instruction_and_direct_controls() {
    use crate::gathering::{HostileFixture, TokenFixture};
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::NestedProphecyCleanup));
    let prophecy = session.observe().play.unwrap().players[2].hand[0].id;
    for id in [
        "tokens:jacqueline".to_owned(),
        format!("tokens:prophecy:0:{prophecy}"),
        "test:reveal".to_owned(),
    ] {
        session.choose(session.observe().revision, &id).unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let table = Table::new(session, dir.path().join("tokens.json"));
    let mut app = host_with_table(table, true);
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
        &mut app,
        Vec2::new(1600., 1000.),
        1.,
    );
    app.world_mut().resource_mut::<Table>().dirty = true;
    for _ in 0..4 {
        app.update();
    }
    let text = rendered_text(&mut app);
    assert!(
        text.contains("Dark Prophecy") && text.contains("Choose 1"),
        "{text}"
    );
    let choices = app.world().resource::<Table>().view().choices;
    for choice in choices
        .iter()
        .filter(|choice| choice.id.starts_with("tokens:keep:"))
    {
        assert_unclipped_button(
            &mut app,
            ui::choice_label(choice),
            Vec2::new(1600., 1000.),
            1.,
        );
    }
    assert!(!app.world().resource::<Table>().context.is_open());
}

#[test]
#[ignore = "Read-only startup check for a supplied ordinary pending-token save"]
fn supplied_token_save_opens_on_visible_guidance() {
    let path =
        std::env::var("ARKHAM_GATHERING_REVIEW_SAVE").expect("Supply a pending-token save path");
    let before = std::fs::read(&path).unwrap();
    let table = Table::open(path.clone().into()).unwrap();
    assert!(!table.modal());
    let mut app = host_with_table(table, true);
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
        &mut app,
        Vec2::new(1600., 1000.),
        1.,
    );
    app.world_mut().resource_mut::<Table>().dirty = true;
    for _ in 0..4 {
        app.update();
    }
    let view = app.world().resource::<Table>().view();
    let tokens = view
        .play
        .as_ref()
        .unwrap()
        .tokens
        .as_ref()
        .expect("Pending physical tokens");
    assert!(!tokens.revealed.is_empty());
    assert!(rendered_text(&mut app).contains("Jacqueline"));
    for choice in view
        .choices
        .iter()
        .filter(|choice| !tokens.choice_members(&choice.id).is_empty())
    {
        assert_unclipped_button(
            &mut app,
            ui::choice_label(choice),
            Vec2::new(1600., 1000.),
            1.,
        );
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
