use super::*;
use bevy::camera::primitives::MeshAabb;
use std::time::Duration;

fn presentation_app(path: PathBuf) -> App {
    let (mut app, _) = scene_app();
    app.world_mut().resource_mut::<Table>().save_path = path.clone();
    app.insert_resource(Orbit::default())
        .insert_resource(Actions::default())
        .insert_resource(AssetSettings::new_tony(AssetConfig::load_at(
            path.with_extension("toml"),
            path.with_extension("assets"),
        )))
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Time>()
        .add_systems(
            Update,
            (
                handle_actions,
                advance_presentation,
                synchronize_scene,
                rebuild_panel,
                animate,
                animate_harm,
            )
                .chain(),
        );
    app.world_mut().spawn(Window::default());
    app
}

fn advance(app: &mut App, seconds: f32) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs_f32(seconds));
    app.update();
}

fn act(app: &mut App, action: Action) {
    app.world_mut().resource_mut::<Actions>().0.push(action);
    advance(app, 0.0);
}

fn choose(app: &mut App, id: &str) {
    let revision = app.world().resource::<Table>().session.observe().revision;
    act(app, Action::Choose(revision, id.into()));
}

fn visible_text(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    world
        .query::<&Text>()
        .iter(world)
        .map(|t| t.0.clone())
        .collect()
}

fn final_tablet_reveal(app: &mut App) {
    app.world_mut().resource_mut::<Table>().session =
        Session::with_draws(vec![TokenFace::Zero, TokenFace::Tablet]);
    for id in ["fight", "commit:none", "reveal", "resolve"] {
        choose(app, id);
    }
    act(app, Action::FinishAnimation);
    for _ in 0..2 {
        choose(app, "resource");
        act(app, Action::FinishAnimation);
    }
    for id in ["fight", "commit:none", "reveal"] {
        choose(app, id);
    }
}

fn chaos_token_transform(app: &mut App) -> Transform {
    let world = app.world_mut();
    *world
        .query_filtered::<&Transform, With<ChaosToken>>()
        .single(world)
        .expect("the revealed chaos token is visible exactly once")
}

#[test]
fn revealed_chaos_token_has_a_thick_body_and_a_separate_face() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = presentation_app(directory.path().join("turn.json"));
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    app.world_mut()
        .resource_mut::<Art>()
        .handles
        .insert("token/tablet".into(), image);
    final_tablet_reveal(&mut app);
    let world = app.world_mut();
    let (mesh, children) = world
        .query_filtered::<(&Mesh3d, Option<&Children>), With<ChaosToken>>()
        .single(world)
        .unwrap();
    let bounds = world
        .resource::<Assets<Mesh>>()
        .get(mesh)
        .unwrap()
        .get_aabb()
        .unwrap();
    assert!(
        bounds.half_extents.min_element() > 0.04,
        "a chaos token must have physical thickness"
    );
    assert!(
        children.is_some_and(|c| !c.is_empty()),
        "the face must sit on the disk body"
    );
}

#[test]
fn finishing_one_harm_preserves_the_following_fight_and_injury() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut app = presentation_app(path.clone());
    final_tablet_reveal(&mut app);
    choose(&mut app, "resolve");
    let committed = app.world().resource::<Table>().session.clone();
    let saved = std::fs::read(&path).unwrap();
    assert!(visible_text(&mut app).iter().any(|t| t == "Tablet token"));
    advance(&mut app, 1.0);
    act(&mut app, Action::FinishAnimation);
    let texts = visible_text(&mut app);
    assert!(texts.iter().any(|t| t == "Tony's Fight"));
    assert!(texts.iter().any(|t| t == "Finish animation"));
    assert!(!texts.iter().any(|t| t == "Start another Tony turn"));
    advance(&mut app, 0.3);
    assert!(visible_text(&mut app).iter().any(|t| t == "Tony's Fight"));
    act(&mut app, Action::FinishAnimation);
    let texts = visible_text(&mut app);
    assert!(texts.iter().any(|t| t == "Internal Injury"));
    assert!(texts.iter().any(|t| t == "Finish animation"));
    assert!(!texts.iter().any(|t| t == "Start another Tony turn"));
    act(&mut app, Action::FinishAnimation);
    let texts = visible_text(&mut app);
    assert!(texts.iter().any(|t| t == "Start another Tony turn"));
    assert!(!texts.iter().any(|t| t == "Finish animation"));
    assert_eq!(app.world().resource::<Table>().session, committed);
    assert_eq!(std::fs::read(path).unwrap(), saved);
}

#[test]
fn a_settled_chaos_token_does_not_reappear_between_harm_stages() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = presentation_app(directory.path().join("turn.json"));
    final_tablet_reveal(&mut app);
    advance(&mut app, 0.2);
    let settled = chaos_token_transform(&mut app);
    assert_eq!(settled.scale, Vec3::ONE);
    choose(&mut app, "resolve");
    assert_eq!(chaos_token_transform(&mut app), settled);
    act(&mut app, Action::FinishAnimation);
    assert_eq!(chaos_token_transform(&mut app), settled);
    act(&mut app, Action::FinishAnimation);
    // Injury follows test cleanup, when the token has returned to the bag.
    assert!(
        visible_text(&mut app)
            .iter()
            .any(|t| t == "Internal Injury")
    );
    assert_eq!(app.world().resource::<Table>().view().bag_count, 16);
    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<Entity, With<ChaosToken>>()
            .iter(world)
            .count(),
        0
    );
}

#[test]
fn reduced_motion_shows_the_revealed_chaos_token_at_its_final_size() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = presentation_app(directory.path().join("turn.json"));
    act(&mut app, Action::Motion);
    final_tablet_reveal(&mut app);
    let settled = chaos_token_transform(&mut app);
    assert_eq!(settled.scale, Vec3::ONE);
    choose(&mut app, "resolve");
    assert_eq!(chaos_token_transform(&mut app), settled);
    assert!(visible_text(&mut app).iter().any(|t| t == "+1 Damage"));
}

#[test]
fn risky_actions_show_the_attack_before_the_player_commits() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = presentation_app(directory.path().join("turn.json"));
    app.update();
    let texts = visible_text(&mut app);
    assert_eq!(
        texts
            .iter()
            .filter(|t| t.as_str() == "Attack of opportunity: Ghoul Minion\n1 damage + 1 horror")
            .count(),
        4
    );
    assert_eq!(app.world().resource::<Table>().session.observe().damage, 0);
    assert!(!directory.path().join("turn.json").exists());
}

#[test]
fn harm_symbols_reach_warnings_counts_stages_card_text_and_history() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = presentation_app(directory.path().join("turn.json"));
    app.world_mut().resource_mut::<Art>().symbol_font = Some(Handle::default());
    app.update();
    fn check(app: &mut App, label_contains: &str, damage: bool, horror: bool) {
        let world = app.world_mut();
        let (_, children) = world
            .query::<(&AccessibleLabel, &Children)>()
            .iter(world)
            .find(|(label, _)| label.0.contains(label_contains))
            .unwrap_or_else(|| panic!("missing readable label: {label_contains}"));
        let spans: Vec<_> = children
            .iter()
            .filter_map(|entity| {
                Some((
                    world.get::<TextSpan>(entity)?,
                    world.get::<TextColor>(entity)?,
                ))
            })
            .collect();
        for (expected, glyph, color) in [
            (damage, 'Q', crate::tabletop_harm_text::DAMAGE_COLOR),
            (horror, 'R', crate::tabletop_harm_text::HORROR_COLOR),
        ] {
            if expected {
                assert!(
                    spans
                        .iter()
                        .any(|(text, tint)| text.0.contains(glyph) && tint.0 == color)
                );
            }
        }
    }
    check(&mut app, "Attack of opportunity: Ghoul Minion", true, true);
    check(&mut app, "Resources 5", true, true);
    choose(&mut app, "investigate");
    check(&mut app, "+1 Damage", true, false);
    check(&mut app, "+1 Horror", false, true);
    act(&mut app, Action::Inspect(2));
    check(&mut app, "Attack: 1 damage and 1 horror", true, true);
    act(&mut app, Action::Close);
    act(&mut app, Action::FinishAnimation);
    act(&mut app, Action::History);
    check(&mut app, "1 damage and 1 horror", true, true);
}

#[test]
fn one_attack_shows_both_amounts_before_investigation_continues() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut app = presentation_app(path.clone());
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Choose(0, "investigate".into()));
    app.update();
    app.update();
    let texts = visible_text(&mut app);
    assert!(texts.iter().any(|t| t == "+1 Damage"));
    assert!(texts.iter().any(|t| t == "+1 Horror"));
    assert!(!texts.iter().any(|t| t == "Commit no cards"));
    let saved = continuity::load_session(&path).unwrap().observe();
    assert_eq!(
        (saved.damage, saved.horror, saved.stage),
        (1, 1, Stage::Commit)
    );
}

#[test]
fn timing_or_finishing_an_attack_releases_choices_without_reapplying_it() {
    for finish_manually in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("turn.json");
        let mut app = presentation_app(path.clone());
        choose(&mut app, "investigate");
        let committed = app.world().resource::<Table>().session.clone();
        let saved = std::fs::read(&path).unwrap();
        // The handler also refuses queued game input during the presentation.
        choose(&mut app, "commit:none");
        assert_eq!(app.world().resource::<Table>().session, committed);
        advance(&mut app, 0.2);
        assert!(
            !visible_text(&mut app)
                .iter()
                .any(|t| t == "Commit no cards")
        );
        if finish_manually {
            act(&mut app, Action::FinishAnimation);
        } else {
            advance(&mut app, 2.0);
        }
        let texts = visible_text(&mut app);
        assert!(texts.iter().any(|t| t == "Commit no cards"));
        assert!(!texts.iter().any(|t| t == "+1 Damage"));
        assert!(!texts.iter().any(|t| t == "+1 Horror"));
        assert_eq!(app.world().resource::<Table>().session, committed);
        assert_eq!(std::fs::read(&path).unwrap(), saved);
    }
}

#[test]
fn a_failed_save_never_displays_landed_harm() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    std::fs::write(&path, "not a valid saved turn").unwrap();
    let mut app = presentation_app(path.clone());
    let initial = app.world().resource::<Table>().session.clone();
    choose(&mut app, "investigate");
    let texts = visible_text(&mut app);
    assert!(texts.iter().any(|t| t.contains("Decision not committed")));
    assert!(!texts.iter().any(|t| t == "+1 Damage" || t == "+1 Horror"));
    assert_eq!(app.world().resource::<Table>().session, initial);
    assert!(!app.world().resource::<Table>().presenting());
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        "not a valid saved turn"
    );
}

#[test]
fn replay_steps_a_combined_attack_and_returns_to_the_unchanged_live_choice() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut app = presentation_app(path.clone());
    choose(&mut app, "investigate");
    let committed = app.world().resource::<Table>().session.clone();
    let saved = std::fs::read(&path).unwrap();
    act(&mut app, Action::Replay);
    act(&mut app, Action::Next);
    let texts = visible_text(&mut app);
    assert!(texts.iter().any(|t| t == "+1 Damage"));
    assert!(texts.iter().any(|t| t == "+1 Horror"));
    let attack_index = app
        .world()
        .resource::<Table>()
        .replay
        .as_ref()
        .unwrap()
        .index();
    advance(&mut app, 5.0);
    assert_eq!(
        app.world()
            .resource::<Table>()
            .replay
            .as_ref()
            .unwrap()
            .index(),
        attack_index
    );
    act(&mut app, Action::Next);
    assert!(app.world().resource::<Table>().harm().is_none());
    assert_eq!(app.world().resource::<Table>().view().stage, Stage::Commit);
    act(&mut app, Action::Back);
    assert!(visible_text(&mut app).iter().any(|t| t == "+1 Horror"));
    act(&mut app, Action::Back);
    assert_eq!(app.world().resource::<Table>().view().damage, 0);
    act(&mut app, Action::Live);
    assert!(
        visible_text(&mut app)
            .iter()
            .any(|t| t == "Commit no cards")
    );
    assert_eq!(app.world().resource::<Table>().session, committed);
    assert_eq!(std::fs::read(path).unwrap(), saved);
}

#[test]
fn reduced_motion_keeps_both_amounts_and_still_settles_the_attack() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = presentation_app(directory.path().join("turn.json"));
    act(&mut app, Action::Motion);
    choose(&mut app, "investigate");
    let texts = visible_text(&mut app);
    assert!(texts.iter().any(|t| t == "Motion: reduced"));
    assert!(texts.iter().any(|t| t == "+1 Damage"));
    assert!(texts.iter().any(|t| t == "+1 Horror"));
    let positions = |app: &mut App| {
        let world = app.world_mut();
        world
            .query_filtered::<(Entity, &Transform), With<harm::HarmVisual>>()
            .iter(world)
            .map(|(entity, transform)| (entity, *transform))
            .collect::<Vec<_>>()
    };
    let still = positions(&mut app);
    assert!(!still.is_empty());
    advance(&mut app, 0.3);
    assert_eq!(
        positions(&mut app),
        still,
        "reduced motion leaves the hit geometry still"
    );
    advance(&mut app, 2.0);
    assert!(
        visible_text(&mut app)
            .iter()
            .any(|t| t == "Commit no cards")
    );
    let view = app.world().resource::<Table>().session.observe();
    assert_eq!((view.damage, view.horror), (1, 1));
}

#[test]
fn the_ghouls_final_hit_is_visible_before_its_removal_and_bounty() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = presentation_app(directory.path().join("turn.json"));
    app.world_mut().resource_mut::<Table>().session =
        Session::with_draws(vec![TokenFace::Zero, TokenFace::Zero]);
    let ghoul_image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    app.world_mut()
        .resource_mut::<Art>()
        .handles
        .insert("card/01160".into(), ghoul_image.clone());
    let ghoul_cards = |app: &mut App| {
        let world = app.world_mut();
        let mut query = world.query_filtered::<&MeshMaterial3d<StandardMaterial>, With<Scene>>();
        let materials = world.resource::<Assets<StandardMaterial>>();
        query
            .iter(world)
            .filter(|handle| {
                materials.get(*handle).unwrap().base_color_texture.as_ref() == Some(&ghoul_image)
            })
            .count()
    };
    for _ in 0..2 {
        for id in ["fight", "commit:none", "reveal", "resolve"] {
            choose(&mut app, id);
        }
        if app
            .world()
            .resource::<Table>()
            .session
            .observe()
            .ghoul
            .alive
        {
            act(&mut app, Action::FinishAnimation);
        }
    }
    let committed = app.world().resource::<Table>().session.observe();
    assert!(!committed.ghoul.alive);
    assert_eq!(committed.resources, 6);
    assert_eq!(ghoul_cards(&mut app), 1);
    let texts = visible_text(&mut app);
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("Ghoul Minion\nDamage 2/2"))
    );
    assert!(texts.iter().any(|t| t == "+1 Damage"));
    assert_eq!(app.world().resource::<Table>().view().resources, 5);
    act(&mut app, Action::FinishAnimation);
    assert_eq!(ghoul_cards(&mut app), 0);
    assert!(visible_text(&mut app).iter().any(|t| t == "Ghoul defeated"));
    assert_eq!(app.world().resource::<Table>().view().resources, 6);
}

fn scene_app() -> (App, Handle<Image>) {
    let mut app = App::new();
    let mut images = Assets::<Image>::default();
    let back = images.add(Image::default());
    app.insert_resource(images);
    app.insert_resource(Table {
        session: Session::new().unwrap(),
        replay: None,
        save_path: PathBuf::new(),
        notice: String::new(),
        inspect: None,
        history: false,
        dirty: true,
        presentation: VecDeque::new(),
        stage_elapsed: 0.0,
        reduced_motion: false,
    })
    .insert_resource(Art {
        handles: [("card-back/player".into(), back.clone())].into(),
        symbol_font: None,
        status: String::new(),
    })
    .init_resource::<Assets<Mesh>>()
    .init_resource::<Assets<StandardMaterial>>();
    (app, back)
}

fn deck_bounds_and_top(app: &mut App) -> (f32, Option<Handle<Image>>) {
    let world = app.world_mut();
    let mut query = world.query_filtered::<
        (&Mesh3d, &MeshMaterial3d<StandardMaterial>, &Transform),
        With<PlayerDeck>,
    >();
    let meshes = world.resource::<Assets<Mesh>>();
    let materials = world.resource::<Assets<StandardMaterial>>();
    let mut bottom = f32::INFINITY;
    let mut top = f32::NEG_INFINITY;
    let mut top_image = None;
    for (mesh, material, transform) in query.iter(world) {
        let bounds = meshes.get(mesh).unwrap().get_aabb().unwrap();
        let center = transform.translation.y + bounds.center.y;
        bottom = bottom.min(center - bounds.half_extents.y);
        let upper = center + bounds.half_extents.y;
        top = top.max(upper);
        if let Some(image) = &materials.get(material).unwrap().base_color_texture {
            assert!(top_image.is_none(), "only the deck's top carries artwork");
            top_image = Some((upper, image.clone()));
        }
    }
    let image = top_image.map(|(surface, image)| {
        assert!(
            (surface - top).abs() < 0.00001,
            "back artwork lies on top of the stack"
        );
        image
    });
    (top - bottom, image)
}

#[test]
fn deck_height_and_back_follow_draws_and_recorded_replay() {
    let (mut app, back) = scene_app();
    app.add_systems(Update, synchronize_scene);
    for (draws, expected_height) in [(0, 0.208), (1, 0.2015), (2, 0.195), (3, 0.1885)] {
        if draws > 0 {
            let mut table = app.world_mut().resource_mut::<Table>();
            let revision = table.session.observe().revision;
            table.session.choose(revision, "draw").unwrap();
        }
        app.update();
        let (height, image) = deck_bounds_and_top(&mut app);
        assert!(
            (height - expected_height).abs() < 0.00001,
            "{draws} draws: height {height}"
        );
        assert_eq!(image.as_ref(), Some(&back), "the draw pile stays face down");
    }
    let recording = app.world().resource::<Table>().session.recording();
    app.world_mut().resource_mut::<Table>().replay = Some(Replay::new(recording).unwrap());
    app.update();
    let (height, image) = deck_bounds_and_top(&mut app);
    assert!(
        (height - 0.208).abs() < 0.00001,
        "replay starts with all 32 cards"
    );
    assert_eq!(image, Some(back));
    app.world_mut().resource_mut::<Table>().replay = None;
    app.update();
    assert!((deck_bounds_and_top(&mut app).0 - 0.1885).abs() < 0.00001);
}
