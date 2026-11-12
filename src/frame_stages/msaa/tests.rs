use super::*;

const BASE: &str = "Arkham Unfolded — Gathering Setup";

fn table() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .add_plugins(MsaaComparisonPlugin);
    // Without the render plugin no camera receives `Msaa` as a required component.
    let main = app.world_mut().spawn((MainCamera, Msaa::Sample4)).id();
    let image = app.world_mut().spawn(Msaa::Sample4).id();
    app.world_mut().spawn(Window {
        title: BASE.into(),
        ..default()
    });
    (app, main, image)
}

fn press(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(key);
    keys.clear();
}

fn msaa(app: &App, entity: Entity) -> Msaa {
    *app.world().get::<Msaa>(entity).unwrap()
}

fn window_title(app: &mut App) -> String {
    app.world_mut()
        .query::<&Window>()
        .single(app.world())
        .unwrap()
        .title
        .clone()
}

#[test]
fn each_key_sets_only_the_table_camera_and_a_repeated_press_keeps_it() {
    let (mut app, main, image) = table();
    app.world_mut().resource_mut::<ComparedMsaa>().counts = Some(2 | 4 | 8);
    app.update();
    assert_eq!(msaa(&app, main), Msaa::Sample4, "starts at Bevy's default");
    assert_eq!(
        window_title(&mut app),
        format!("{BASE} · #189: MSAA 4× · GPU readings off · F5 4×, F6 2×, F7 off")
    );

    for (key, expected) in [
        (KeyCode::F7, Msaa::Off),
        (KeyCode::F7, Msaa::Off),
        (KeyCode::F6, Msaa::Sample2),
        (KeyCode::F6, Msaa::Sample2),
        (KeyCode::F5, Msaa::Sample4),
        (KeyCode::F5, Msaa::Sample4),
    ] {
        press(&mut app, key);
        assert_eq!(msaa(&app, main), expected, "after {key:?}");
        assert_eq!(
            msaa(&app, image),
            Msaa::Sample4,
            "image camera after {key:?}"
        );
        assert!(
            window_title(&mut app)
                .starts_with(&format!("{BASE} · #189: MSAA {} ·", name(expected))),
            "title after {key:?}"
        );
    }
}

#[test]
fn a_setting_the_adapter_does_not_allow_is_refused_and_shown() {
    let (mut app, main, _) = table();
    app.world_mut().resource_mut::<ComparedMsaa>().counts = Some(4);
    app.update();
    // Shown once support is known, before anyone presses F6.
    assert_eq!(
        window_title(&mut app),
        format!("{BASE} · #189: MSAA 4× · GPU readings off · F5 4×, F6 2× unavailable, F7 off")
    );
    press(&mut app, KeyCode::F7);
    press(&mut app, KeyCode::F6);
    assert_eq!(msaa(&app, main), Msaa::Off, "unchanged by the refusal");
    assert_eq!(
        *app.world().resource::<ComparedMsaa>(),
        ComparedMsaa {
            chosen: Msaa::Off,
            counts: Some(4),
            refused: Some(Msaa::Sample2),
        }
    );
    assert!(
        window_title(&mut app).starts_with(&format!("{BASE} · #189: MSAA off (2× unavailable) ·")),
        "{}",
        window_title(&mut app)
    );
    // An allowed choice clears the notice.
    press(&mut app, KeyCode::F5);
    assert_eq!(msaa(&app, main), Msaa::Sample4);
    assert!(window_title(&mut app).starts_with(&format!("{BASE} · #189: MSAA 4× ·")));
}

#[test]
fn the_start_setting_is_read_from_its_variable() {
    assert_eq!(start_setting(None), Ok(Msaa::Sample4));
    assert_eq!(start_setting(Some("")), Ok(Msaa::Sample4));
    assert_eq!(start_setting(Some("4")), Ok(Msaa::Sample4));
    assert_eq!(start_setting(Some(" 2 ")), Ok(Msaa::Sample2));
    assert_eq!(start_setting(Some("off")), Ok(Msaa::Off));
    assert_eq!(start_setting(Some("1")), Ok(Msaa::Off));
    assert!(start_setting(Some("8")).is_err());
}

#[test]
fn support_needs_every_format_and_adapter_specific_features_for_two_and_eight() {
    let all = TextureFormatFeatureFlags::MULTISAMPLE_X2
        | TextureFormatFeatureFlags::MULTISAMPLE_X4
        | TextureFormatFeatureFlags::MULTISAMPLE_X8;
    assert_eq!(support_counts(true, |_| all), 2 | 4 | 8);
    assert_eq!(
        support_counts(false, |_| all),
        4,
        "WebGPU guarantees only 4"
    );
    let no_two_for_depth = |format| {
        if format == TextureFormat::Depth32Float {
            all - TextureFormatFeatureFlags::MULTISAMPLE_X2
        } else {
            all
        }
    };
    assert_eq!(support_counts(true, no_two_for_depth), 4 | 8);

    assert!(allowed(Some(4), Msaa::Off));
    assert!(allowed(Some(4), Msaa::Sample4));
    assert!(!allowed(Some(4), Msaa::Sample2));
    assert!(allowed(Some(2 | 4), Msaa::Sample2));
    assert!(allowed(None, Msaa::Sample4));
    assert!(
        !allowed(None, Msaa::Sample2),
        "unknown until the renderer exists"
    );
}
