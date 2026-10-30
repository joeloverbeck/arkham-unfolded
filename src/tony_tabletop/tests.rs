use super::*;
// Drive the same presentation clock as the native adapter, independently of rendering.
use crate::tabletop_pacing::PresentationTime as Time;
use bevy::camera::primitives::MeshAabb;
use std::time::Duration;

use crate::tabletop_standee_checks as standee_checks;

mod harm_lifetime;

#[test]
fn diagnostic_initial_view_parses_without_changing_default_framing_or_files() {
    let directory = tempfile::tempdir().unwrap();
    let parse = |args: &[&str]| {
        Options::parse_from(
            ["--experiment", "tony"]
                .into_iter()
                .chain(args.iter().copied())
                .map(str::to_owned),
            AssetConfig::load_at(
                directory.path().join("settings.toml"),
                directory.path().join("assets"),
            ),
        )
    };
    assert_eq!(parse(&[]).unwrap().unwrap().initial_view, None);
    for (name, area) in [
        ("overview", Area::Overview),
        ("scenario", Area::Scenario),
        ("test", Area::Test),
    ] {
        assert_eq!(
            parse(&["--initial-view", name])
                .unwrap()
                .unwrap()
                .initial_view,
            Some(area)
        );
    }
    assert!(parse(&["--initial-view"]).is_err());
    assert!(parse(&["--initial-view", "missing"]).is_err());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn launch_uses_configured_root_without_creating_a_preference() {
    let dir = tempfile::tempdir().unwrap();
    let settings = dir.path().join("tony.toml");
    let default_root = dir.path().join("prepared-tony");
    let options = Options::parse_from(
        ["--experiment", "tony"].map(str::to_owned),
        AssetConfig::load_at(settings.clone(), default_root.clone()),
    )
    .unwrap()
    .unwrap();
    assert_eq!(options.art_root, Some(default_root));
    assert!(!settings.exists());
}

#[test]
fn launch_overrides_leave_saved_tony_preference_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let settings = dir.path().join("tony.toml");
    let default_root = dir.path().join("default-tony");
    let chosen_root = dir.path().join("chosen-tony");
    let mut config = AssetConfig::load_at(settings.clone(), default_root.clone());
    config.save_root(&chosen_root).unwrap();
    let saved_bytes = std::fs::read(&settings).unwrap();

    for (args, expected) in [
        (vec!["--experiment", "tony"], Some(chosen_root.clone())),
        (
            vec!["--experiment", "tony", "--art-root", "temporary-art"],
            Some(PathBuf::from("temporary-art")),
        ),
        (vec!["--experiment", "tony", "--no-art"], None),
    ] {
        let options = Options::parse_from(
            args.into_iter().map(str::to_owned),
            AssetConfig::load_at(settings.clone(), default_root.clone()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(options.art_root, expected);
        assert_eq!(options.config.root(), chosen_root);
        assert_eq!(std::fs::read(&settings).unwrap(), saved_bytes);
    }
}

fn presentation_app(path: PathBuf) -> App {
    presentation_app_in(App::new(), path)
}

fn presentation_app_in(app: App, path: PathBuf) -> App {
    let (mut app, _) = scene_app_in(app);
    app.world_mut().resource_mut::<Table>().save_path = path.clone();
    app.insert_resource(Orbit::new(CameraMode::Tony))
        .insert_resource(Actions::default())
        .insert_resource(AssetSettings::new_tony(AssetConfig::load_at(
            path.with_extension("toml"),
            path.with_extension("assets"),
        )))
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Time>()
        .init_resource::<FeedbackSettings>()
        .add_systems(
            Update,
            (
                handle_actions,
                advance_presentation,
                synchronize_scene,
                synchronize_feedback,
                rebuild_panel,
                controls::draw_menu,
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
    for source in [
        Source::Card(0),
        Source::Card(1),
        Source::Card(4),
        Source::Deck,
    ] {
        act(&mut app, Action::Select(0, source, Vec3::ZERO));
        let texts = visible_text(&mut app);
        assert_eq!(texts.iter().filter(|t| t.as_str() == "Attack of opportunity: Ghoul Minion\n1 damage + 1 horror").count(), 1,
            "the selected component exposes its own warning before activation");
    }
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
    act(&mut app, Action::Select(0, Source::Card(1), Vec3::ZERO));
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
    assert!(app.world().resource::<Table>().reduced_motion);
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
    scene_app_in(App::new())
}

#[test]
fn book_cards_keep_their_pose_direct_inspection_and_reopened_state() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = presentation_app(directory.path().join("turn.json"));
    advance(&mut app, 0.0);
    let before = app.world().resource::<Table>().session.clone();
    let pose = |app: &mut App, name: &str| {
        let world = app.world_mut();
        *world
            .query::<(&Name, &Transform)>()
            .iter(world)
            .find(|(n, _)| n.as_str() == name)
            .unwrap()
            .1
    };
    let agenda = pose(&mut app, "Card 01105");
    let act_card = pose(&mut app, "Card 01108");
    assert!(agenda.translation.x < 0.0 && act_card.translation.x > 0.0);
    assert!((agenda.translation.x + act_card.translation.x).abs() < 0.001);
    for (index, card) in [(9, agenda), (8, act_card)] {
        let normal = card.rotation * Vec3::Y;
        assert!(normal.z > 0.9 && normal.y > 0.1 && normal.y < 0.4);
        act(
            &mut app,
            Action::Select(0, Source::Card(index), card.translation),
        );
        assert_eq!(app.world().resource::<Table>().inspect, Some(index));
        assert!(!app.world().resource::<Table>().context.open);
        act(&mut app, Action::Close);
        assert_eq!(app.world().resource::<Table>().session, before);
    }
    act(&mut app, Action::Save);
    act(&mut app, Action::Reopen);
    assert_eq!(pose(&mut app, "Card 01105"), agenda);
    assert_eq!(pose(&mut app, "Card 01108"), act_card);
    assert_eq!(app.world().resource::<Table>().session, before);
    act(&mut app, Action::Replay);
    assert_eq!(pose(&mut app, "Card 01105"), agenda);
    assert_eq!(pose(&mut app, "Card 01108"), act_card);
    act(&mut app, Action::Live);
    assert_eq!(app.world().resource::<Table>().session, before);
}

#[test]
fn inspect_only_cards_open_directly_without_changing_live_or_replayed_choices() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = presentation_app(directory.path().join("turn.json"));
    advance(&mut app, 0.0);
    let before = app.world().resource::<Table>().session.clone();
    act(&mut app, Action::Select(0, Source::Card(3), Vec3::ZERO));
    assert_eq!(app.world().resource::<Table>().inspect, Some(3));
    assert!(!app.world().resource::<Table>().context.open);
    act(&mut app, Action::Close);
    assert_eq!(app.world().resource::<Table>().session, before);
    act(&mut app, Action::Select(0, Source::Card(2), Vec3::ZERO));
    assert!(app.world().resource::<Table>().context.open);
    assert_eq!(app.world().resource::<Table>().inspect, None);
    act(&mut app, Action::Replay);
    act(&mut app, Action::Select(0, Source::Card(2), Vec3::ZERO));
    assert_eq!(app.world().resource::<Table>().inspect, Some(2));
    assert!(!app.world().resource::<Table>().context.open);
    assert_eq!(app.world().resource::<Table>().session, before);
}

#[test]
fn source_marks_follow_open_menus_without_rebuilding_cards_or_erasing_commitments() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = presentation_app(directory.path().join("turn.json"));
    advance(&mut app, 0.0);
    let card = |app: &mut App, source| {
        let world = app.world_mut();
        world
            .query::<(Entity, &CardSource)>()
            .iter(world)
            .find(|(_, candidate)| candidate.0 == source)
            .unwrap()
            .0
    };
    let ghoul = card(&mut app, Source::Card(2));
    act(&mut app, Action::Select(0, Source::Card(2), Vec3::ZERO));
    assert!(app.world().get::<CardFeedback>(ghoul).unwrap().menu_source);
    act(&mut app, Action::DismissMenu);
    assert!(!app.world().get::<CardFeedback>(ghoul).unwrap().menu_source);
    assert_eq!(ghoul, card(&mut app, Source::Card(2)));
    choose(&mut app, "fight");
    let revision = app.world().resource::<Table>().view().revision;
    act(
        &mut app,
        Action::Select(revision, Source::Hand(0), Vec3::ZERO),
    );
    let epoch = app.world().resource::<Table>().context.epoch;
    act(&mut app, Action::ToggleCommit(epoch, revision, 0));
    let hand = card(&mut app, Source::Hand(0));
    assert!(
        app.world()
            .get::<CardFeedback>(hand)
            .unwrap()
            .pending_choice
    );
    act(
        &mut app,
        Action::Select(revision, Source::Card(3), Vec3::ZERO),
    );
    assert_eq!(app.world().resource::<Table>().inspect, Some(3));
    assert!(app.world().resource::<FeedbackSettings>().blocked);
    act(&mut app, Action::Close);
    assert!(
        app.world()
            .get::<CardFeedback>(hand)
            .unwrap()
            .pending_choice
    );
    assert!(!app.world().resource::<FeedbackSettings>().blocked);
    act(&mut app, Action::Motion);
    assert!(app.world().resource::<FeedbackSettings>().reduced_motion);
}

fn scene_app_in(mut app: App) -> (App, Handle<Image>) {
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
        presentation: HarmPresentation::default(),
        reduced_motion: false,
        context: Context::default(),
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

#[test]
fn production_scene_keeps_physical_standees_and_readable_missing_portraits() {
    use crate::tabletop_layout::InvestigatorClass::{Mystic, Rogue, Survivor};

    for missing in [None, Some(1)] {
        let (mut app, _) = scene_app();
        let mut expected = Vec::new();
        for (index, (code, class)) in [("06003", Rogue), ("06005", Survivor), ("60401", Mystic)]
            .into_iter()
            .enumerate()
        {
            let image = (missing != Some(index)).then(|| {
                app.world_mut()
                    .resource_mut::<Assets<Image>>()
                    .add(Image::default())
            });
            if let Some(image) = &image {
                app.world_mut()
                    .resource_mut::<Art>()
                    .handles
                    .insert(format!("standee/{code}"), image.clone());
            }
            expected.push((class, image));
        }
        app.add_systems(Update, synchronize_scene);
        app.update();
        standee_checks::assert_standees(app.world_mut(), &expected);
        for name in ["Tony", "Patrice", "Jacqueline"] {
            assert!(
                app.world_mut()
                    .query_filtered::<&Text, With<WorldLabel>>()
                    .iter(app.world())
                    .any(|label| label.0.contains(name)),
                "{name} remains readable with or without a portrait"
            );
        }
    }
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

#[test]
fn selecting_and_inspecting_two_copies_preserves_the_complete_commitment() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("turn.json");
    let mut app = presentation_app(path.clone());
    for _ in 0..3 {
        choose(&mut app, "draw");
        act(&mut app, Action::FinishAnimation);
    }
    choose(&mut app, "fight");
    let before = app.world().resource::<Table>().session.clone();
    let revision = before.observe().revision;
    let saved = std::fs::read(&path).unwrap();
    for id in [0, 3] {
        act(
            &mut app,
            Action::Select(revision, Source::Hand(id), Vec3::ZERO),
        );
        let epoch = app.world().resource::<Table>().context.epoch;
        act(&mut app, Action::ToggleCommit(epoch, revision, id));
    }
    act(&mut app, Action::Inspect(5));
    act(&mut app, Action::Close);
    assert_eq!(
        app.world().resource::<Table>().context.commitments,
        [0, 3].into()
    );
    assert_eq!(app.world().resource::<Table>().session, before);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        saved,
        "inspection and staging do not save a choice"
    );
    act(&mut app, Action::ConfirmCommit(revision));
    let view = app.world().resource::<Table>().session.observe();
    assert_eq!(view.stage, Stage::Reveal);
    assert_eq!(
        view.committed
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        vec![0, 3]
    );
    assert_eq!(view.test.unwrap().skill_value, 7);
    assert!(
        app.world()
            .resource::<Table>()
            .context
            .commitments
            .is_empty()
    );
}

fn keyboard_menu_app(path: PathBuf) -> (App, Entity) {
    use bevy::{
        camera::{ComputedCameraValues, RenderTarget, RenderTargetInfo},
        input_focus::{InputDispatchPlugin, InputFocusPlugin},
        window::{PrimaryWindow, WindowRef},
    };

    let mut app = App::new();
    app.add_plugins((
        bevy::app::TaskPoolPlugin::default(),
        bevy::asset::AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        bevy::text::TextPlugin,
        bevy::transform::TransformPlugin,
        bevy::camera::visibility::VisibilityPlugin,
        bevy::input::InputPlugin,
        InputFocusPlugin,
        InputDispatchPlugin,
        feedback::FeedbackPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        bevy::ui::UiPlugin,
        bevy::ui_widgets::ButtonPlugin,
    ))
    .init_resource::<Assets<TextureAtlasLayout>>()
    .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
    .init_resource::<bevy::prelude::Time<Real>>();
    let mut app = presentation_app_in(app, path);
    app.add_message::<bevy::window::CursorMoved>().add_systems(
        Update,
        camera_controls.after(handle_actions).before(rebuild_panel),
    );
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    {
        let mut window = app.world_mut().get_mut::<Window>(window).unwrap();
        window.resolution = (1600, 1000).into();
        window.focused = true;
    }
    // TabNavigationPlugin attaches to the primary window during startup.
    app.world_mut().entity_mut(window).insert(PrimaryWindow);
    app.world_mut().spawn((
        MainCamera,
        Transform::default(),
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(1600, 1000),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
        RenderTarget::Window(WindowRef::Entity(window)),
    ));
    app.update();
    (app, window)
}

fn keyboard_press(app: &mut App, window: Entity, key_code: KeyCode) {
    keyboard_chord(app, window, key_code, false);
}

fn keyboard_chord(app: &mut App, window: Entity, key_code: KeyCode, shift: bool) {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    let logical_key = match key_code {
        KeyCode::Tab => Key::Tab,
        KeyCode::Enter => Key::Enter,
        KeyCode::Space => Key::Space,
        KeyCode::Escape => Key::Escape,
        KeyCode::KeyW => Key::Character("w".into()),
        KeyCode::ArrowLeft => Key::ArrowLeft,
        _ => unreachable!(),
    };
    let mut events = Vec::new();
    if shift {
        events.push((KeyCode::ShiftLeft, Key::Shift, ButtonState::Pressed));
    }
    events.extend([
        (key_code, logical_key.clone(), ButtonState::Pressed),
        (key_code, logical_key, ButtonState::Released),
    ]);
    if shift {
        events.push((KeyCode::ShiftLeft, Key::Shift, ButtonState::Released));
    }
    for (key_code, logical_key, state) in events {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key,
            state,
            text: None,
            repeat: false,
            window,
        });
        advance(app, 0.1);
    }
}

fn focused_button_label(app: &App) -> Option<&str> {
    let entity = app.world().resource::<InputFocus>().get()?;
    app.world()
        .get::<Children>(entity)?
        .iter()
        .find_map(|child| app.world().get::<Text>(child).map(|text| text.0.as_str()))
}

#[test]
fn keyboard_context_navigation_wraps_and_cancels_without_a_game_action() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, window) = keyboard_menu_app(dir.path().join("navigation.json"));
    let before = app.world().resource::<Table>().view();
    act(
        &mut app,
        Action::Select(before.revision, Source::Card(2), Vec3::ZERO),
    );
    assert_eq!(focused_button_label(&app), Some("Fight  /  bonus action"));
    keyboard_press(&mut app, window, KeyCode::Tab);
    assert_eq!(focused_button_label(&app), Some("Evade  /  1 action"));
    keyboard_chord(&mut app, window, KeyCode::Tab, true);
    assert_eq!(focused_button_label(&app), Some("Fight  /  bonus action"));
    keyboard_chord(&mut app, window, KeyCode::Tab, true);
    assert_eq!(focused_button_label(&app), Some("Inspect"));
    keyboard_press(&mut app, window, KeyCode::Tab);
    assert_eq!(focused_button_label(&app), Some("Fight  /  bonus action"));
    keyboard_chord(&mut app, window, KeyCode::Tab, true);
    keyboard_press(&mut app, window, KeyCode::Space);
    assert_eq!(app.world().resource::<Table>().inspect, Some(2));
    keyboard_press(&mut app, window, KeyCode::Escape);
    assert!(app.world().resource::<Table>().inspect.is_none());
    act(
        &mut app,
        Action::Select(before.revision, Source::Card(2), Vec3::ZERO),
    );
    keyboard_press(&mut app, window, KeyCode::Escape);
    assert!(!app.world().resource::<Table>().context.open);
    assert_eq!(app.world().resource::<Table>().view(), before);
}

#[test]
fn keyboard_menu_inspection_preserves_staged_copies_until_explicit_confirmation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keyboard.json");
    let (mut app, window) = keyboard_menu_app(path.clone());
    let initial_target = app.world().resource::<Orbit>().pose().target;
    keyboard_press(&mut app, window, KeyCode::KeyW);
    assert_ne!(
        app.world().resource::<Orbit>().pose().target,
        initial_target
    );
    for _ in 0..3 {
        choose(&mut app, "draw");
        act(&mut app, Action::FinishAnimation);
    }
    choose(&mut app, "fight");
    let before = app.world().resource::<Table>().session.clone();
    let revision = before.observe().revision;
    let saved = std::fs::read(&path).unwrap();
    let target = app.world().resource::<Orbit>().pose().target;
    let yaw = app.world().resource::<Orbit>().pose().yaw;
    for id in [0, 3] {
        act(
            &mut app,
            Action::Select(revision, Source::Hand(id), Vec3::ZERO),
        );
        assert_eq!(focused_button_label(&app), Some("Select to commit +1"));
        keyboard_press(&mut app, window, KeyCode::KeyW);
        keyboard_press(&mut app, window, KeyCode::ArrowLeft);
        assert_eq!(app.world().resource::<Orbit>().pose().target, target);
        assert_eq!(app.world().resource::<Orbit>().pose().yaw, yaw);
        keyboard_press(&mut app, window, KeyCode::Enter);
    }
    act(
        &mut app,
        Action::Select(revision, Source::Hand(3), Vec3::ZERO),
    );
    keyboard_press(&mut app, window, KeyCode::Tab);
    assert_eq!(focused_button_label(&app), Some("Inspect"));
    keyboard_press(&mut app, window, KeyCode::Enter);
    assert_eq!(app.world().resource::<Table>().inspect, Some(5));
    keyboard_press(&mut app, window, KeyCode::KeyW);
    keyboard_press(&mut app, window, KeyCode::ArrowLeft);
    assert_eq!(app.world().resource::<Orbit>().pose().target, target);
    assert_eq!(app.world().resource::<Orbit>().pose().yaw, yaw);
    keyboard_press(&mut app, window, KeyCode::Escape);
    assert!(app.world().resource::<Table>().inspect.is_none());
    assert_eq!(
        app.world().resource::<Table>().context.commitments,
        [0, 3].into()
    );
    assert_eq!(app.world().resource::<Table>().session, before);
    assert_eq!(std::fs::read(&path).unwrap(), saved);

    // The ordinary decision panel is outside the contextual tab group. Exercise
    // its explicit confirmation through production UI picking, not an Action.
    let confirm = app
        .world_mut()
        .query_filtered::<(Entity, &Children), With<Button>>()
        .iter(app.world())
        .find_map(|(entity, children)| {
            children
                .iter()
                .any(|child| {
                    app.world().get::<Text>(child).is_some_and(|text| {
                        text.0.starts_with("Commit selected") && text.0.ends_with("+2")
                    })
                })
                .then_some(entity)
        })
        .unwrap_or_else(|| {
            panic!(
                "confirmation button missing from {:?}",
                visible_text(&mut app)
            )
        });
    let position = app
        .world()
        .get::<bevy::ui::UiGlobalTransform>(confirm)
        .unwrap()
        .translation;
    use bevy::{
        camera::RenderTarget,
        picking::pointer::{Location, PointerAction, PointerId, PointerInput, PointerLocation},
        window::WindowRef,
    };
    let location = Location {
        target: RenderTarget::Window(WindowRef::Entity(window))
            .normalize(Some(window))
            .unwrap(),
        position,
    };
    app.world_mut()
        .spawn((PointerId::Mouse, PointerLocation::new(location.clone())));
    app.update();
    assert_eq!(app.world().resource::<Table>().session, before);
    for action in [
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
        app.update();
    }
    let after = app.world().resource::<Table>().view();
    assert_eq!(after.stage, Stage::Reveal);
    assert_eq!(after.revision, revision + 1);
    assert_eq!(
        after
            .committed
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        [0, 3]
    );
    assert_eq!(app.world().resource::<Orbit>().pose().target, target);
    assert_eq!(app.world().resource::<Orbit>().pose().yaw, yaw);
}

#[test]
fn dismissed_stale_and_replayed_component_choices_never_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = presentation_app(dir.path().join("turn.json"));
    act(&mut app, Action::Select(0, Source::Card(2), Vec3::ZERO));
    let epoch = app.world().resource::<Table>().context.epoch;
    let original = app.world().resource::<Table>().session.clone();
    act(&mut app, Action::Inspect(2));
    act(&mut app, Action::Close);
    act(&mut app, Action::ContextChoice(epoch, 0, "fight".into()));
    assert_eq!(
        app.world().resource::<Table>().session,
        original,
        "opening inspection invalidates old menu callbacks"
    );
    act(&mut app, Action::Select(0, Source::Card(2), Vec3::ZERO));
    let epoch = app.world().resource::<Table>().context.epoch;
    choose(&mut app, "resource");
    act(&mut app, Action::FinishAnimation);
    let current = app.world().resource::<Table>().session.clone();
    act(&mut app, Action::ContextChoice(epoch, 0, "fight".into()));
    assert_eq!(app.world().resource::<Table>().session, current);
    act(&mut app, Action::Replay);
    let revision = app.world().resource::<Table>().view().revision;
    act(
        &mut app,
        Action::Select(revision, Source::Card(2), Vec3::ZERO),
    );
    let epoch = app.world().resource::<Table>().context.epoch;
    act(
        &mut app,
        Action::ContextChoice(epoch, revision, "fight".into()),
    );
    assert_eq!(
        app.world().resource::<Table>().session,
        current,
        "replay remains inspection-only"
    );
    assert!(app.world().resource::<Table>().replay.is_some());
}

#[test]
fn component_action_mapping_covers_exact_offered_choices_without_relabeling_actor() {
    let game = Session::new().unwrap();
    let view = game.observe();
    let sources = [
        Source::Card(0),
        Source::Card(1),
        Source::Card(2),
        Source::Card(4),
        Source::Deck,
        Source::Bag,
        Source::Token,
    ];
    let mapped: std::collections::BTreeSet<_> = sources
        .into_iter()
        .flat_map(|source| {
            controls::source_choices(&view, source)
                .into_iter()
                .map(|choice| choice.id.as_str())
        })
        .collect();
    let offered: std::collections::BTreeSet<_> = view
        .choices
        .iter()
        .filter(|c| c.id != "end_turn")
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(mapped, offered);
    assert!(
        controls::source_choices(&view, Source::Card(3)).is_empty(),
        "Contracts has no triggered player choice here"
    );
}

fn orbit_mouse_app() -> (App, Entity) {
    orbit_mouse_app_in(App::new())
}

#[test]
fn camera_adapter_retains_tony_utilities_wheel_and_escape_frame_pan() {
    use bevy::ecs::system::RunSystemOnce;

    let (mut app, _) = orbit_mouse_app();
    let before = app.world().resource::<Orbit>().pose();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(100));
    app.world_mut()
        .resource_mut::<AccumulatedMouseScroll>()
        .delta
        .y = 1.0;
    app.world_mut().resource_mut::<Table>().context.utilities = true;
    app.world_mut().run_system_once(camera_controls).unwrap();
    assert!((app.world().resource::<Orbit>().pose().distance - 15.4).abs() < 0.00001);
    assert!(app.world().resource::<Table>().context.utilities);
    app.world_mut().resource_mut::<Table>().context.utilities = false;
    app.world_mut()
        .resource_mut::<AccumulatedMouseScroll>()
        .delta
        .y = 0.0;
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::Escape);
        keys.press(KeyCode::KeyW);
    }
    app.world_mut().run_system_once(camera_controls).unwrap();
    let panned = app.world().resource::<Orbit>().pose();
    assert!(panned.target.z < before.target.z);
    for inspect in [false, true] {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.history = !inspect;
        table.inspect = inspect.then_some(0);
        app.world_mut().run_system_once(camera_controls).unwrap();
        assert_eq!(app.world().resource::<Orbit>().pose(), panned);
    }
}

#[test]
fn camera_adapter_selects_tony_replay_targets_and_reduced_motion() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = orbit_mouse_app();
    app.world_mut().resource_mut::<Table>().save_path = dir.path().join("replay.json");
    choose(&mut app, "investigate");
    act(&mut app, Action::Focus(Area::Overview));
    act(&mut app, Action::Replay);
    assert_eq!(app.world().resource::<Orbit>().pose().distance, 16.0);
    advance(&mut app, 0.1);
    let moving = app.world().resource::<Orbit>().pose();
    assert!((moving.target.x + 4.8).abs() < 0.00001);
    assert!((moving.target.z - 6.4).abs() < 0.00001);
    act(&mut app, Action::Follow);
    advance(&mut app, 0.1);
    assert!(!app.world().resource::<Orbit>().is_following());
    assert!(app.world().resource::<Orbit>().pose().target.x < moving.target.x);
    act(&mut app, Action::Follow);
    act(&mut app, Action::Motion);
    act(&mut app, Action::Next);
    assert!(app.world().resource::<Table>().harm().is_some());
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        scene::INVESTIGATOR_AT
    );
    act(&mut app, Action::Next);
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        layout::investigator_center(0, 3)
    );
    act(&mut app, Action::Live);
    choose(&mut app, "commit:none");
    assert_eq!(app.world().resource::<Table>().view().stage, Stage::Reveal);
    choose(&mut app, "reveal");
    assert!(app.world().resource::<Table>().view().held_token.is_some());
    act(&mut app, Action::Replay);
    let last = app
        .world()
        .resource::<Table>()
        .replay
        .as_ref()
        .unwrap()
        .len()
        - 1;
    for _ in 0..last {
        act(&mut app, Action::Next);
    }
    assert!(app.world().resource::<Table>().view().held_token.is_some());
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        layout::TEST_CENTER
    );
}

fn orbit_mouse_app_in(app: App) -> (App, Entity) {
    let directory = tempfile::tempdir().unwrap();
    let mut app = presentation_app_in(app, directory.path().join("orbit.json"));
    app.add_plugins(bevy::input::InputPlugin)
        .add_message::<bevy::window::CursorMoved>()
        .init_resource::<bevy::picking::hover::HoverMap>()
        .add_systems(
            Update,
            camera_controls.after(handle_actions).before(rebuild_panel),
        );
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    {
        let mut window = app.world_mut().get_mut::<Window>(window).unwrap();
        window.resolution = (1600, 1000).into();
        window.focused = true;
        window.set_cursor_position(Some(Vec2::new(800.0, 400.0)));
    }
    app.world_mut().spawn((MainCamera, Transform::default()));
    app.update();
    (app, window)
}

fn orbit_button(app: &mut App, window: Entity, state: bevy::input::ButtonState) {
    app.world_mut()
        .write_message(bevy::input::mouse::MouseButtonInput {
            button: MouseButton::Right,
            state,
            window,
        });
    app.update();
}

fn orbit_cursor(app: &mut App, window: Entity, position: Vec2) {
    let delta = {
        let mut window = app.world_mut().get_mut::<Window>(window).unwrap();
        let delta = window.cursor_position().map(|previous| position - previous);
        window.set_cursor_position(Some(position));
        delta
    };
    // This is the window-event path used by Winit, independently of raw motion.
    app.world_mut().write_message(bevy::window::CursorMoved {
        window,
        position,
        delta,
    });
    app.update();
}

#[test]
fn right_drag_through_real_navigation_layout_ignores_empty_space_but_not_buttons() {
    use bevy::{
        camera::{ComputedCameraValues, RenderTarget, RenderTargetInfo},
        picking::{
            hover::HoverMap,
            pointer::{Location, PointerId, PointerLocation},
        },
        window::{PrimaryWindow, WindowRef},
    };

    // Use the production panel, Bevy layout, and UI picking. In particular, do
    // not inject a HoverMap that silently omits the transparent navigation root.
    let mut app = App::new();
    app.add_plugins((
        bevy::app::TaskPoolPlugin::default(),
        bevy::asset::AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        bevy::text::TextPlugin,
        bevy::transform::TransformPlugin,
        bevy::camera::visibility::VisibilityPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        bevy::ui::UiPlugin,
    ))
    .init_resource::<Assets<TextureAtlasLayout>>()
    .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
    .init_resource::<bevy::prelude::Time<Real>>();
    let (mut app, window) = orbit_mouse_app_in(app);
    app.world_mut().entity_mut(window).insert(PrimaryWindow);
    let camera = app
        .world_mut()
        .query_filtered::<Entity, With<MainCamera>>()
        .single(app.world())
        .unwrap();
    let target = RenderTarget::Window(WindowRef::Entity(window));
    app.world_mut().entity_mut(camera).insert((
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(1600, 1000),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
        target.clone(),
    ));
    let pointer = app
        .world_mut()
        .spawn((
            PointerId::Mouse,
            PointerLocation::new(Location {
                target: target.normalize(Some(window)).unwrap(),
                position: Vec2::new(60.0, 700.0),
            }),
        ))
        .id();
    for _ in 0..3 {
        app.update();
    }
    let nav = app
        .world_mut()
        .query_filtered::<(Entity, &Node), With<Panel>>()
        .iter(app.world())
        .find(|(_, node)| node.left == px(10) && node.top == px(76))
        .map(|(entity, _)| entity)
        .unwrap();
    assert_eq!(
        app.world().get::<ComputedNode>(nav).unwrap().size,
        Vec2::new(124.0, 744.0)
    );
    let move_cursor = |app: &mut App, position: Vec2| {
        app.world_mut()
            .get_mut::<PointerLocation>(pointer)
            .unwrap()
            .location
            .as_mut()
            .unwrap()
            .position = position;
        orbit_cursor(app, window, position);
    };
    move_cursor(&mut app, Vec2::new(60.0, 700.0));
    let before = app.world().resource::<Orbit>().pose().yaw;
    orbit_button(&mut app, window, bevy::input::ButtonState::Pressed);
    move_cursor(&mut app, Vec2::new(100.0, 710.0));
    assert_ne!(
        app.world().resource::<Orbit>().pose().yaw,
        before,
        "the actual empty navigation root {nav:?} must not consume a scene drag; actual hover: {:?}",
        app.world().resource::<HoverMap>().get(&PointerId::Mouse)
    );
    orbit_button(&mut app, window, bevy::input::ButtonState::Released);
    // Camera movement rebuilds the production panel; use its current entities.
    let nav = app
        .world_mut()
        .query_filtered::<(Entity, &Node), With<Panel>>()
        .iter(app.world())
        .find(|(_, node)| node.left == px(10) && node.top == px(76))
        .map(|(entity, _)| entity)
        .unwrap();
    let button = app
        .world()
        .get::<Children>(nav)
        .unwrap()
        .iter()
        .find(|entity| app.world().get::<Button>(*entity).is_some())
        .unwrap();
    let button_position = app
        .world()
        .get::<bevy::ui::UiGlobalTransform>(button)
        .unwrap()
        .translation;
    move_cursor(&mut app, button_position);
    assert!(
        app.world().resource::<HoverMap>()[&PointerId::Mouse]
            .keys()
            .any(|entity| *entity == button
                || app
                    .world()
                    .get::<ChildOf>(*entity)
                    .is_some_and(|parent| parent.parent() == button))
    );
    let before = app.world().resource::<Orbit>().pose().yaw;
    orbit_button(&mut app, window, bevy::input::ButtonState::Pressed);
    move_cursor(&mut app, button_position + Vec2::new(5.0, 0.0));
    assert_eq!(app.world().resource::<Orbit>().pose().yaw, before);
}
