use super::*;

fn loader_app(prepared: &Prepared) -> App {
    use bevy::image::{CompressedImageFormats, ImageLoader, ImagePlugin};

    let mut app = App::new();
    app.add_plugins((
        bevy::app::TaskPoolPlugin::default(),
        prepared.asset_plugin(),
        ImagePlugin::default(),
    ))
    // Native RenderPlugin supplies the GPU image-format capabilities. These
    // prepared PNG/JPEG files need no GPU texture compression or graphics device.
    .register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE))
    .init_asset::<Font>()
    .init_asset_loader::<bevy::text::FontLoader>();
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
fn real_collection_loads_every_verified_image_and_symbol_font() {
    use std::time::{Duration, Instant};

    let root = std::env::var_os("ARKHAM_GATHERING_TEST_ASSETS")
        .expect("set ARKHAM_GATHERING_TEST_ASSETS to the prepared collection");
    let prepared = Prepared::new(root.into()).unwrap();
    let report = &prepared.collection;
    assert!(report.complete, "{}", prepared.status);
    let expected = report.valid_count();

    let mut app = loader_app(&prepared);
    let art = prepared.load(app.world().resource::<AssetServer>());
    assert_eq!(art.handles.len() + 1, expected);
    let font = &art.symbol_font;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        app.update();
        let server = app.world().resource::<AssetServer>();
        if art
            .handles
            .values()
            .all(|handle| server.is_loaded_with_dependencies(handle.id()))
            && server.is_loaded_with_dependencies(font.id())
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "prepared assets did not finish loading"
        );
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
}
