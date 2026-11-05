use super::*;

pub(super) fn at_public_moment(
    session: Session,
    play: crate::gathering::PlayObservation,
    path: PathBuf,
    symbols: bool,
) -> App {
    let mut table = Table::new(session, path);
    table.presentation = presentation::Presentation::at_moment(
        &table.session,
        PresentationMoment {
            revealed_story: vec![],
            revision: 1,
            sequence: 0,
            caption: "Card presentation witness".into(),
            play,
        },
    );
    let mut app = host_with_table(table, false);
    if symbols {
        app.insert_resource(art::Art::with_symbol_font_for_test(Handle::default()));
        app.world_mut().resource_mut::<Table>().dirty = true;
        app.update();
    }
    app
}

fn by_source(app: &mut App, source: controls::Source) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<(Entity, &ContextTarget), With<CardFeedback>>()
        .iter(world)
        .find_map(|(entity, target)| (target.source == source).then_some(entity))
        .expect("the public card has a selectable face")
}

fn captions(app: &mut App, card: Entity) -> Vec<String> {
    let world = app.world_mut();
    world
        .query::<(&Text, &motion::CardLabel)>()
        .iter(world)
        .filter(|(_, label)| label.0 == card)
        .map(|(text, _)| text.0.clone())
        .collect()
}

#[test]
fn asset_status_and_discard_keep_their_captions_and_inspection_keys_after_redraw() {
    for symbols in [false, true] {
        asset_status_and_discard(symbols);
    }
}

fn asset_status_and_discard(symbols: bool) {
    let dir = tempfile::tempdir().unwrap();
    let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Backpack(
        crate::gathering::BackpackFixture::Defeat,
    ));
    let mut play = session.observe().play.unwrap();
    // A public-observation witness isolates construction from rules execution.
    // One previously attached card is now the discard top; the pack is exhausted.
    let discarded = play.backpacks[0].cards.remove(0);
    let discarded_id = discarded.id;
    play.players[0].discard = vec![discarded];
    let pack = play.players[0]
        .assets
        .iter_mut()
        .find(|asset| asset.card.code == "04037")
        .unwrap();
    pack.ready = false;
    pack.uses = 2;
    pack.damage = 1;
    pack.horror = 1;
    let pack_id = pack.card.id;
    let mut app = at_public_moment(session, play, dir.path().join("assets.json"), symbols);
    for _ in 0..2 {
        let pack = by_source(&mut app, controls::Source::Backpack(pack_id));
        assert_eq!(
            captions(&mut app, pack),
            ["Backpack\nUses 2 · Damage 1 · Horror 1 · 2 attached"]
        );
        let label = caption_entity(&mut app, pack);
        assert_eq!(
            app.world().get::<AccessibleLabel>(label).unwrap().0,
            "Backpack"
        );
        assert!(caption_spans(&app, label).is_empty());
        assert_eq!(
            app.world().get::<motion::PhysicalCard>(pack).unwrap().0,
            pack_id
        );
        assert!(
            (app.world().get::<Transform>(pack).unwrap().rotation * Vec3::NEG_Z)
                .abs_diff_eq(Vec3::X, 0.001),
            "exhaustion moves the printed top edge to the right"
        );
        let discard = by_source(&mut app, controls::Source::Discard(Investigator::Tony));
        assert_eq!(
            captions(&mut app, discard),
            ["1 cards\nLucky Cigarette Case"]
        );
        assert_eq!(
            app.world().get::<motion::PhysicalCard>(discard).unwrap().0,
            discarded_id
        );
        // Exercise the production pointer observers: display status must not
        // replace the title or pile key supplied to selection/inspection.
        for (entity, title, key) in [
            (pack, "Backpack", format!("backpack:{pack_id}")),
            (
                discard,
                "1 cards\nLucky Cigarette Case",
                "discard:tony".into(),
            ),
        ] {
            queue_mesh_click(&mut app, entity);
            let Action::Select(_, _, selection) =
                app.world_mut().resource_mut::<Actions>().0.pop().unwrap()
            else {
                panic!("card pointer must select its own inspection target");
            };
            assert_eq!(selection.title, title);
            assert_eq!(selection.inspect_code, key);
        }
        app.world_mut().resource_mut::<Table>().dirty = true;
        app.update();
    }
}

#[test]
fn grouped_enemies_and_scenario_piles_keep_only_their_intended_captions_after_redraw() {
    for symbols in [false, true] {
        grouped_enemies_and_scenario_piles(symbols);
    }
}

fn grouped_enemies_and_scenario_piles(symbols: bool) {
    let dir = tempfile::tempdir().unwrap();
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringPriestDefeat);
    let mut play = session.observe().play.unwrap();
    let mut minion = play.scenario.enemies[0].clone();
    minion.card = crate::gathering::CardView {
        id: 900,
        code: "01160".into(),
        title: "Ghoul Minion".into(),
        weakness: false,
    };
    minion.location = "01112".into();
    minion.engaged = None;
    minion.exhausted = false;
    let mut top = minion.clone();
    top.card.id = 901;
    top.exhausted = true;
    play.scenario.enemies = vec![minion, top];
    play.scenario.victory = vec![crate::gathering::CardView {
        id: 902,
        code: "01118".into(),
        title: "Flesh-Eater".into(),
        weakness: false,
    }];
    play.scenario.removed = vec![crate::gathering::CardView {
        id: 903,
        code: "01162".into(),
        title: "Rotting Remains".into(),
        weakness: false,
    }];
    let mut app = at_public_moment(session, play, dir.path().join("piles.json"), symbols);
    for _ in 0..2 {
        let lower = by_source(&mut app, controls::Source::Card(900));
        let top = by_source(&mut app, controls::Source::Card(901));
        assert!(
            (app.world().get::<Transform>(top).unwrap().rotation * Vec3::NEG_Z)
                .abs_diff_eq(Vec3::X, 0.001),
            "An exhausted unengaged enemy turns clockwise too"
        );
        let room = by_source(&mut app, controls::Source::Location("01112".into()));
        let room_x = app.world().get::<Transform>(room).unwrap().translation.x;
        let enemy_x = app.world().get::<Transform>(top).unwrap().translation.x;
        assert!(
            enemy_x + 1.89 / 2. < room_x - 2.3,
            "The rotated enemy must clear the room's standee/caption reservation"
        );
        assert_eq!(captions(&mut app, lower), ["Ghoul Minion"]);
        assert_eq!(captions(&mut app, top), ["Ghoul Minion\nExhausted"]);
        let label = caption_entity(&mut app, top);
        assert_eq!(
            app.world().get::<AccessibleLabel>(label).unwrap().0,
            "Ghoul Minion\nHallway · Exhausted\nUnengaged"
        );
        let spans = caption_spans(&app, label);
        assert!(
            spans.is_empty(),
            "Individual card captions have no stale group spans"
        );
        for (source, id) in [
            (controls::Source::Victory, 902),
            (controls::Source::ScenarioRemoved, 903),
        ] {
            let pile = by_source(&mut app, source);
            assert!(captions(&mut app, pile).is_empty());
            assert_eq!(app.world().get::<motion::PhysicalCard>(pile).unwrap().0, id);
        }
        let text = rendered_text(&mut app);
        assert!(text.contains("1 cards\n1 VP"));
        assert!(text.contains("Removed\n1 cards"));
        for (entity, id) in [(lower, 900), (top, 901)] {
            assert_eq!(
                app.world().get::<motion::PhysicalCard>(entity).unwrap().0,
                id
            );
            queue_mesh_click(&mut app, entity);
            let Action::Select(_, _, selection) =
                app.world_mut().resource_mut::<Actions>().0.pop().unwrap()
            else {
                panic!("each enemy must keep its own pointer observer");
            };
            assert_eq!(selection.source, controls::Source::Card(id));
            assert_eq!(selection.inspect_code, "01160");
            assert!(!selection.title.contains("Damage "));
        }
        app.world_mut().resource_mut::<Table>().dirty = true;
        app.update();
    }
}

fn caption_entity(app: &mut App, card: Entity) -> Entity {
    let world = app.world_mut();
    world
        .query::<(Entity, &motion::CardLabel)>()
        .iter(world)
        .find_map(|(entity, label)| (label.0 == card).then_some(entity))
        .unwrap()
}

fn caption_spans(app: &App, label: Entity) -> String {
    app.world()
        .get::<Children>(label)
        .into_iter()
        .flatten()
        .filter_map(|child| app.world().get::<TextSpan>(*child))
        .map(|span| span.0.as_str())
        .collect()
}
