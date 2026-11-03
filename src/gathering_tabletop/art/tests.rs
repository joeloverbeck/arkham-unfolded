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
fn substitute_text_keeps_supported_separators_and_plain_harm_names() {
    let art = Art::default();
    let mut world = World::new();
    let entity = world
        .spawn(art.text("The Gathering · Damage 2 → Horror 1", 18., Color::WHITE))
        .id();
    assert_eq!(
        world.get::<Text>(entity).unwrap().0,
        "The Gathering  /  Damage 2 > Horror 1"
    );
}

#[test]
fn disabling_artwork_does_not_select_a_default_collection() {
    let prepared = Prepared::new(None).unwrap();
    assert!(prepared.root().is_none());
    assert_eq!(prepared.status(), DISABLED);
    assert!(prepared.collection.is_none());
    let app = loader_app(&prepared);
    let art = prepared.load(app.world().resource::<AssetServer>());
    assert!(art.handles.is_empty());
    assert!(art.symbol_font().is_none());
    assert_eq!(art.status(), DISABLED);
}

#[test]
fn unknown_collection_does_not_admit_an_existing_card_file() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("cards")).unwrap();
    std::fs::write(directory.path().join("cards/01111.png"), b"not verified").unwrap();
    std::fs::write(directory.path().join("manifest.json"), b"{}").unwrap();

    let prepared = Prepared::new(Some(directory.path().to_owned())).unwrap();
    assert_eq!(prepared.root(), Some(directory.path()));
    let collection = prepared.collection.as_ref().unwrap();
    assert!(!collection.recognized);
    assert_eq!(collection.verified_path("card-01111"), None);
    assert!(
        prepared
            .status()
            .contains("No compatible prepared asset collection")
    );
    let app = loader_app(&prepared);
    let art = prepared.load(app.world().resource::<AssetServer>());
    assert!(art.handles.is_empty());
    assert_eq!(art.card("01111", false), None);
    assert!(art.symbol_font().is_none());
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
    let prepared = Prepared::new(Some(root.into())).unwrap();
    let report = prepared.collection.as_ref().unwrap();
    assert!(report.complete, "{}", prepared.status());
    let expected = report.valid_count();

    let mut app = loader_app(&prepared);
    let art = prepared.load(app.world().resource::<AssetServer>());
    assert_eq!(
        art.handles.len() + usize::from(art.symbol_font.is_some()),
        expected
    );
    let font = art.symbol_font.as_ref().expect("verified symbol font");
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
