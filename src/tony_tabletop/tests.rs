use super::*;
use bevy::camera::primitives::MeshAabb;

fn scene_app() -> (App, Handle<Image>) {
    let mut app = App::new();
    let back = Assets::<Image>::default().add(Image::default());
    app.insert_resource(Table {
        session: Session::new().unwrap(),
        replay: None,
        save_path: PathBuf::new(),
        notice: String::new(),
        inspect: None,
        history: false,
        dirty: true,
    })
    .insert_resource(Art {
        handles: [("card-back/player".into(), back.clone())].into(),
        status: String::new(),
    })
    .init_resource::<Assets<Mesh>>()
    .init_resource::<Assets<StandardMaterial>>()
    .add_systems(Update, synchronize_scene);
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
