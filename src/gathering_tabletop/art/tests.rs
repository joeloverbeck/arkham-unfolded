use super::*;

fn loader_app(prepared: &Prepared) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(prepared.asset_plugin())
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            })
            .set(bevy::render::RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .disable::<bevy::winit::WinitPlugin>(),
    )
    .add_plugins(crate::tabletop_chaos_pouch::ChaosPouchPlugin);
    app
}

#[test]
fn hidden_location_art_never_falls_back_to_the_revealed_face() {
    let mut images = Assets::<Image>::default();
    let revealed = images.add(Image::default());
    let hidden = images.add(Image::default());
    let mut art = Art::default();
    art.handles.insert("card-01111".into(), revealed.clone());

    assert_eq!(art.card("01111", false), Some(revealed));
    assert_eq!(art.card("01111", true), None);
    art.handles.insert("card-01111-back".into(), hidden.clone());
    assert_eq!(art.card("01111", true), Some(hidden));
    assert_eq!(art.card("01112", false), None);
}

#[test]
fn artwork_text_retains_plain_english_accessible_labels() {
    let art = Art::default();
    let mut world = World::new();
    let entity = world
        .spawn(art.text("The Gathering · Damage 2 → Horror 1", 18., Color::WHITE))
        .id();
    assert_eq!(
        world.get::<AccessibleLabel>(entity).unwrap().0,
        "The Gathering · Damage 2 → Horror 1"
    );
}

#[test]
fn absent_collection_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    assert!(Prepared::new(directory.path().join("absent")).is_err());
}

#[test]
fn unknown_collection_does_not_admit_an_existing_card_file() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("cards")).unwrap();
    std::fs::write(directory.path().join("cards/01111.png"), b"not verified").unwrap();
    std::fs::write(directory.path().join("manifest.json"), b"{}").unwrap();

    assert!(Prepared::new(directory.path().to_owned()).is_err());
    assert_eq!(
        std::fs::read(directory.path().join("cards/01111.png")).unwrap(),
        b"not verified"
    );
}

/// The prepared payloads remain outside Git. Run this explicitly after preparing
/// the real collection; ordinary checks retain their clean-clone independence.
#[test]
#[ignore = "requires ARKHAM_GATHERING_TEST_ASSETS pointing at a complete prepared collection"]
fn real_collection_loads_every_verified_image_font_and_pouch() {
    use std::time::{Duration, Instant};

    let root = std::env::var_os("ARKHAM_GATHERING_TEST_ASSETS")
        .expect("set ARKHAM_GATHERING_TEST_ASSETS to the prepared collection");
    let prepared = Prepared::new(root.into()).unwrap();
    let report = &prepared.collection;
    assert!(report.complete, "{}", prepared.status);
    let expected = report.valid_count();

    let mut app = loader_app(&prepared);
    // Match native startup: register the complete admission set before loading,
    // rather than requesting model dependencies after the scene has loaded.
    let settings = tempfile::tempdir().unwrap();
    let config = crate::asset_config::AssetConfig::load_at(
        settings.path().join("settings"),
        prepared.root.clone(),
    );
    let (recovery, _) = crate::tabletop_startup::Recovery::prepare(
        config,
        prepared.root.clone(),
        crate::tabletop_support::asset_settings::Collection::Gathering,
    )
    .unwrap();
    recovery.install(&mut app, report);
    let art = prepared.load(app.world().resource::<AssetServer>());
    assert_eq!(art.handles.len() + 2, expected);
    let font = &art.symbol_font;
    let velvet: Handle<crate::tabletop_chaos_pouch::VelvetMaterial> = app
        .world()
        .resource::<AssetServer>()
        .load(crate::tabletop_chaos_pouch::VELVET);
    let render_output = std::env::var_os("ARKHAM_POUCH_RENDER_DIR").map(PathBuf::from);
    if let Some(output) = &render_output {
        std::fs::create_dir(output).unwrap();
        use super::super::{Orbit, PresentationTime, Redraw, RenderHealth, Table};
        app.insert_resource(Table::new(
            crate::gathering::Session::with_hostile_fixture(
                crate::gathering::HostileFixture::JacquelineCancellation,
            ),
            output.join("render-fixture.json"),
        ))
        .insert_resource(RenderHealth::ready_for_test())
        .init_resource::<PresentationTime>()
        .init_resource::<Redraw>()
        .insert_resource(Orbit::new().with_home((crate::tabletop_layout::test_bag(), 6.0)));
        super::super::install(&mut app);
        app.world_mut().spawn((
            Window {
                resolution: (1280, 900).into(),
                focused: true,
                ..default()
            },
            bevy::window::PrimaryWindow,
        ));
        app.insert_resource(prepared.load(app.world().resource::<AssetServer>()));
    }
    app.finish();
    app.cleanup();
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        app.update();
        let server = app.world().resource::<AssetServer>();
        if art
            .handles
            .values()
            .all(|handle| server.is_loaded_with_dependencies(handle.id()))
            && server.is_loaded_with_dependencies(font.id())
            && server.is_loaded_with_dependencies(art.chaos_pouch.id())
            && server.is_loaded_with_dependencies(velvet.id())
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "prepared assets did not finish loading"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let admission_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.update();
        match crate::tabletop_health::required_art_ready_for_test(&mut app) {
            Ok(()) => break,
            Err(reason) => assert!(
                Instant::now() < admission_deadline,
                "native artwork admission remained blocked: {reason}"
            ),
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let images = app.world().resource::<Assets<Image>>();
    for (role, handle) in &art.handles {
        let image = images
            .get(handle)
            .unwrap_or_else(|| panic!("missing image {role}"));
        assert!(image.width() > 0 && image.height() > 0, "{role}");
    }
    assert!(app.world().resource::<Assets<Font>>().contains(font));
    let required =
        crate::tabletop_chaos_pouch::material_and_textures(app.world().resource::<AssetServer>());
    for handle in required.iter().skip(1) {
        let images = app.world().resource::<Assets<Image>>();
        let texture = images.get(handle.id().typed::<Image>()).unwrap();
        assert!(
            crate::tabletop_chaos_pouch::texture_ready(texture),
            "pouch texture must have its minification levels"
        );
    }
    crate::tabletop_health::required_art_ready_for_test(&mut app).unwrap();
    let material = app
        .world_mut()
        .resource_mut::<Assets<crate::tabletop_chaos_pouch::VelvetMaterial>>()
        .remove(velvet.id())
        .unwrap();
    assert!(
        crate::tabletop_health::required_art_ready_for_test(&mut app).is_err(),
        "a decoded scene without its custom material must not admit play"
    );
    app.world_mut()
        .resource_mut::<Assets<crate::tabletop_chaos_pouch::VelvetMaterial>>()
        .insert(velvet.id(), material)
        .unwrap();
    crate::tabletop_health::required_art_ready_for_test(&mut app).unwrap();
    if let Some(output) = render_output {
        render_pouch(&mut app, velvet, &output);
    }
}

fn render_pouch(
    app: &mut App,
    velvet: Handle<crate::tabletop_chaos_pouch::VelvetMaterial>,
    output: &std::path::Path,
) {
    use bevy::{camera::RenderTarget, render::render_resource::TextureFormat};
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            1280,
            900,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
    let at = crate::tabletop_layout::test_bag();
    use super::super::{MainCamera, Orbit};
    let world = app.world_mut();
    let camera = world
        .query_filtered::<Entity, With<MainCamera>>()
        .single(world)
        .unwrap();
    app.world_mut().entity_mut(camera).insert((
        bevy::ui::IsDefaultUiCamera,
        RenderTarget::Image(image.clone().into()),
        Transform::from_translation(at + Vec3::new(0.0, 3.8, 4.5)).looking_at(at, Vec3::Y),
    ));
    for _ in 0..60 {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let world = app.world_mut();
    assert_eq!(
        world
            .query::<&MeshMaterial3d<crate::tabletop_chaos_pouch::VelvetMaterial>>()
            .iter(world)
            .count(),
        2,
        "both velvet primitives must use the custom material"
    );
    let mut lowest = f32::INFINITY;
    let mut primitives = 0;
    let mut triangles = 0;
    for (entity, mesh, transform) in world
        .query::<(Entity, &Mesh3d, &GlobalTransform)>()
        .iter(world)
    {
        let mut ancestor = entity;
        let mut pouch = false;
        while let Some(parent) = world.get::<ChildOf>(ancestor) {
            ancestor = parent.parent();
            if world.get::<WorldAssetRoot>(ancestor).is_some() {
                pouch = true;
                break;
            }
        }
        if !pouch {
            continue;
        }
        let mesh = world.resource::<Assets<Mesh>>().get(&mesh.0).unwrap();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("pouch mesh has no positions")
        };
        for position in positions {
            lowest = lowest.min(transform.transform_point(Vec3::from_array(*position)).y);
        }
        triangles += mesh
            .indices()
            .map_or(positions.len(), |indices| indices.len())
            / 3;
        primitives += 1;
    }
    assert_eq!((primitives, triangles), (6, 208_708));
    assert!(
        (lowest - crate::tabletop_layout::MAT_TOP).abs() < 0.001,
        "pouch floor must rest on the mat: {lowest}"
    );
    let enabled = capture(app, &image, &output.join("pouch-sheen.png"));
    let original = app
        .world()
        .resource::<Assets<crate::tabletop_chaos_pouch::VelvetMaterial>>()
        .get(&velvet)
        .unwrap()
        .clone();
    app.world_mut()
        .resource_mut::<Assets<crate::tabletop_chaos_pouch::VelvetMaterial>>()
        .get_mut(&velvet)
        .unwrap()
        .extension
        .disable_for_test();
    for _ in 0..8 {
        app.update();
    }
    let disabled = capture(app, &image, &output.join("pouch-no-sheen.png"));
    let changed = enabled
        .as_raw()
        .iter()
        .zip(disabled.as_raw())
        .filter(|(a, b)| a.abs_diff(**b) > 1)
        .count();
    assert!(
        changed > 100,
        "the sheen shader must change actual rendered pixels: {changed}"
    );
    {
        let mut materials = app
            .world_mut()
            .resource_mut::<Assets<crate::tabletop_chaos_pouch::VelvetMaterial>>();
        let mut material = materials.get_mut(&velvet).unwrap();
        *material = original.clone();
        material.base.normal_map_texture = None;
    }
    for _ in 0..8 {
        app.update();
    }
    capture(app, &image, &output.join("pouch-geometric-normals.png"));
    *app.world_mut()
        .resource_mut::<Assets<crate::tabletop_chaos_pouch::VelvetMaterial>>()
        .get_mut(&velvet)
        .unwrap() = original;
    for _ in 0..8 {
        app.update();
    }
    let restored = capture(app, &image, &output.join("pouch-restored.png"));
    assert!(
        enabled
            .as_raw()
            .iter()
            .zip(restored.as_raw())
            .filter(|(a, b)| a.abs_diff(**b) > 2)
            .count()
            < 100,
        "first material upload must use the prepared texture levels"
    );
    let (at, distance) = crate::tabletop_layout::focus(crate::tabletop_layout::Area::Test);
    app.insert_resource(Orbit::new().with_home((at, distance)));
    app.world_mut().entity_mut(camera).insert(
        Transform::from_translation(at + Vec3::new(0.0, 1.0_f32.sin(), 1.0_f32.cos()) * distance)
            .looking_at(at, Vec3::Y),
    );
    for _ in 0..8 {
        app.update();
    }
    capture(app, &image, &output.join("test-area.png"));
    use super::super::{CardFeedback, ContextTarget, PresentationTime, Table, controls::Source};
    let opportunity = |app: &mut App| {
        let world = app.world_mut();
        world
            .query::<(&ContextTarget, &CardFeedback)>()
            .iter(world)
            .find(|(target, _)| target.source == Source::Bag)
            .unwrap()
            .1
            .opportunity
    };
    assert!(!opportunity(app), "Inspect alone must not light the pouch");
    assert_eq!(bag_halo(app).1, Visibility::Hidden);
    let mut session = app.world().resource::<Table>().session.clone();
    for _ in 0..8 {
        let view = session.observe();
        if view
            .choices
            .iter()
            .any(|choice| choice.id == "tokens:jacqueline")
        {
            break;
        }
        let id = ["window:pass", "test:confirm"]
            .into_iter()
            .find(|id| view.choices.iter().any(|choice| choice.id == *id))
            .unwrap();
        session.choose(view.revision, id).unwrap();
    }
    app.insert_resource(Table::new(session, output.join("available-fixture.json")));
    for _ in 0..12 {
        app.update();
    }
    assert!(
        opportunity(app),
        "a real reveal choice must light the imported pouch"
    );
    capture(app, &image, &output.join("test-area-available.png"));
    app.insert_resource(Orbit::new().with_home((crate::tabletop_layout::test_bag(), 6.0)));
    for _ in 0..8 {
        app.update();
    }
    let low = capture(app, &image, &output.join("pouch-cue-low.png"));
    assert!((bag_halo(app).0 - 0.46).abs() < 0.001);
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_millis(800));
    for _ in 0..8 {
        app.update();
    }
    let high = capture(app, &image, &output.join("pouch-cue-high.png"));
    assert!((bag_halo(app).0 - 0.64).abs() < 0.001);
    assert_ne!(
        low.as_raw(),
        high.as_raw(),
        "available pouch feedback must breathe"
    );
}

fn bag_halo(app: &mut App) -> (f32, Visibility) {
    use super::super::{ContextTarget, controls::Source};
    let world = app.world_mut();
    let bag = world
        .query::<(Entity, &ContextTarget)>()
        .iter(world)
        .find(|(_, target)| target.source == Source::Bag)
        .unwrap()
        .0;
    let rim = world
        .get::<Children>(bag)
        .unwrap()
        .iter()
        .find(|entity| {
            world
                .get::<MeshMaterial3d<StandardMaterial>>(*entity)
                .is_some()
        })
        .unwrap();
    let handle = &world
        .get::<MeshMaterial3d<StandardMaterial>>(rim)
        .unwrap()
        .0;
    (
        world
            .resource::<Assets<StandardMaterial>>()
            .get(handle)
            .unwrap()
            .base_color
            .alpha(),
        *world.get::<Visibility>(rim).unwrap(),
    )
}

fn capture(app: &mut App, target: &Handle<Image>, path: &std::path::Path) -> image::RgbImage {
    use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
    let result = std::sync::Arc::new(std::sync::Mutex::new(None));
    let slot = result.clone();
    app.world_mut()
        .spawn(Screenshot::image(target.clone()))
        .observe(move |event: On<ScreenshotCaptured>| {
            *slot.lock().unwrap() = Some(event.image.clone());
        });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        app.update();
        if let Some(image) = result.lock().unwrap().take() {
            let image = image.try_into_dynamic().unwrap().to_rgb8();
            image.save(path).unwrap();
            return image;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "GPU capture timed out"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
