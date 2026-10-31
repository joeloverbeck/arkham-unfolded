//! Finite presentation witness for the accepted table arrangement; no rules or records.
//! Asset faces repeat deliberately: this challenges geometry, not legal coexistence.
//! This standalone example imports unchanged production sources below; it does not
//! instrument the library or replace the application binary, so its normal Cargo target is safe.
use arkham_unfolded_experiment::assets;
use bevy::{
    input::mouse::AccumulatedMouseScroll,
    prelude::*,
    render::view::screenshot::{Capturing, Screenshot, save_to_disk},
};
use std::{collections::BTreeMap, path::PathBuf, process::ExitCode};

// Import the real geometry and renderer implementations without widening library APIs.
#[allow(dead_code)]
#[rustfmt::skip]
#[path = "../src/tabletop_camera.rs"]
mod tabletop_camera;
#[path = "../src/tabletop_harm_tokens.rs"]
mod tabletop_harm_tokens;
#[allow(dead_code)]
#[path = "../src/tabletop_layout.rs"]
mod tabletop_layout;
#[allow(dead_code)]
#[path = "../src/tabletop_standees.rs"]
mod tabletop_standees;

use tabletop_camera::{CameraMode, FrameInput, Orbit, parse_initial_view};
use tabletop_layout::{self as layout, Area};

const ASSETS: [(&str, &str); 4] = [
    ("06010", "Bounty Contracts"),
    ("04109", "Arcane Research"),
    ("60408", "Clairvoyance"),
    ("11067", "Eyes of the Dreamer"),
];
const HAND: [(&str, &str); 4] = [
    ("03263", "On the Hunt"),
    ("07032", "Promise of Power"),
    ("01080", "Lucky!"),
    ("01093", "Unexpected Courage"),
];
const INVESTIGATORS: [(&str, &str); 3] = [
    ("06003", "Tony"),
    ("06005", "Patrice"),
    ("60401", "Jacqueline"),
];
const VIEWS: [Area; 6] = [
    Area::Overview,
    Area::Scenario,
    Area::Test,
    Area::Investigator(0),
    Area::Investigator(1),
    Area::Investigator(2),
];

#[derive(Resource)]
struct Capture {
    path: Option<PathBuf>,
    quit: bool,
    frame: usize,
    requested: bool,
}

#[derive(Resource, Default)]
struct Art(BTreeMap<String, Handle<Image>>);

#[derive(Component)]
struct Label(Vec3, f32);

fn main() -> ExitCode {
    let mut root = None;
    let mut screenshot = None;
    let mut quit = false;
    let mut initial = Area::Investigator(1);
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--no-art" => root = None,
            "--quit-after-screenshot" => quit = true,
            "--art-root" | "--screenshot" | "--initial-view" | "--experiment" | "--save" => {
                let Some(value) = args.next() else {
                    eprintln!("{argument} needs a value");
                    return ExitCode::FAILURE;
                };
                match argument.as_str() {
                    "--art-root" => root = Some(PathBuf::from(value)),
                    "--screenshot" => screenshot = Some(PathBuf::from(value)),
                    "--initial-view" => match parse_initial_view(&value) {
                        Ok(area) => initial = area,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::FAILURE;
                        }
                    },
                    // Accepted for the native launcher; this witness has no session or save.
                    _ => {}
                }
            }
            "--help" | "-h" => {
                println!(
                    "Presentation fixture — not a playable position\n\
                     --art-root PATH | --no-art\n--screenshot PATH --quit-after-screenshot\n\
                     --initial-view overview|scenario|test|tony|patrice|jacqueline\n\
                     Tab cycles views; 1–6 selects them. Existing camera keys remain available.\n\
                     --experiment and --save are ignored; no records are read or written."
                );
                return ExitCode::SUCCESS;
            }
            _ => {
                eprintln!("Unknown option: {argument}");
                return ExitCode::FAILURE;
            }
        }
    }
    if quit && screenshot.is_none() {
        eprintln!("--quit-after-screenshot requires --screenshot");
        return ExitCode::FAILURE;
    }
    let reports = root.as_ref().map(|root| {
        [
            assets::inspect_party(root),
            assets::inspect_tony(root),
            assets::inspect(root),
        ]
    });
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Arkham Unfolded — Layout Capacity Fixture".into(),
                    resolution: (1600, 1000).into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: root.map_or_else(
                    || "local/experiment/absent-art".into(),
                    |path| path.to_string_lossy().into_owned(),
                ),
                ..default()
            }),
    );
    let server = app.world().resource::<AssetServer>();
    let mut art = Art::default();
    let roles = ASSETS
        .into_iter()
        .chain(HAND)
        .chain(INVESTIGATORS)
        .chain([
            ("02038", "Internal Injury"),
            ("01160", "Ghoul Minion"),
            ("01111", "Study"),
        ])
        .map(|(code, _)| format!("card/{code}"))
        .chain(INVESTIGATORS.map(|(code, _)| format!("standee/{code}")))
        .chain(
            [
                "card-back/player",
                "token/damage",
                "token/horror",
                "token/minus_one",
                "token/zero",
                "token/curse",
            ]
            .map(str::to_owned),
        );
    for role in roles {
        if let Some(path) = reports.as_ref().and_then(|reports| {
            reports
                .iter()
                .find_map(|report| report.verified_path(&role))
        }) {
            art.0.insert(role, server.load(path.to_owned()));
        }
    }
    app.insert_resource(ClearColor(Color::srgb(0.025, 0.045, 0.055)))
        .insert_resource(art)
        .insert_resource(Orbit::new(CameraMode::Jacqueline).with_initial_view(Some(initial)))
        .insert_resource(Capture {
            path: screenshot,
            quit,
            frame: 0,
            requested: false,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (navigate, position_labels, capture).chain());
    match app.run() {
        AppExit::Success => ExitCode::SUCCESS,
        AppExit::Error(code) => ExitCode::from(code.get()),
    }
}

fn label(commands: &mut Commands, text: impl Into<String>, at: Vec3, width: f32) {
    commands.spawn((
        Text::new(text.into().replace('·', "/")),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::srgb(0.91, 0.89, 0.81)),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            width: px(width),
            padding: UiRect::all(px(3)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.025, 0.045, 0.055, 0.88)),
        Pickable::IGNORE,
        Label(at, width),
    ));
}

#[allow(clippy::too_many_arguments)]
fn card(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Art,
    role: &str,
    title: &str,
    at: Vec3,
    size: Vec2,
    sideways: bool,
) {
    let image = art.0.get(role).cloned();
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(size.x, size.y))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: if image.is_some() {
                Color::WHITE
            } else {
                Color::srgb(0.34, 0.31, 0.23)
            },
            base_color_texture: image.clone(),
            unlit: true,
            ..default()
        })),
        Transform::from_translation(at).with_rotation(Quat::from_rotation_y(if sideways {
            std::f32::consts::FRAC_PI_2
        } else {
            0.0
        })),
    ));
    if image.is_none() {
        label(commands, title, at + Vec3::Y * 0.1, 72.0);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    art: Res<Art>,
) {
    layout::spawn_surfaces(&mut commands, &mut meshes, &mut materials);
    commands.spawn((Camera3d::default(), Transform::default()));
    commands.spawn((
        DirectionalLight {
            illuminance: 7_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.6, 0.0)),
    ));
    commands.spawn((
        Text::new("Presentation fixture - not a playable position\nRepeated selected-deck faces test space only. Tab: next view / 1-6: Overview / Scenario / Test / Tony / Patrice / Jacqueline"),
        TextFont { font_size: FontSize::Px(18.0), ..default() },
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, top: px(10), left: px(12), right: px(12), ..default() },
        BackgroundColor(Color::srgb(0.12, 0.08, 0.04)),
    ));
    for (index, (code, name)) in INVESTIGATORS.into_iter().enumerate() {
        let size = if index == 2 {
            Vec2::new(1.8, 2.52)
        } else {
            Vec2::new(1.35, 1.89)
        };
        label(
            &mut commands,
            format!("{name} · {} hand cards", [8, 9, 1][index]),
            layout::investigator_heading(index),
            210.0,
        );
        for (name, at) in layout::zone_labels(index) {
            label(&mut commands, name, at, 90.0);
        }
        card(
            &mut commands,
            &mut meshes,
            &mut materials,
            &art,
            &format!("card/{code}"),
            name,
            layout::investigator_card(index),
            size,
            true,
        );
        card(
            &mut commands,
            &mut meshes,
            &mut materials,
            &art,
            "card-back/player",
            "Deck",
            layout::investigator_deck(index),
            size,
            false,
        );
        card(
            &mut commands,
            &mut meshes,
            &mut materials,
            &art,
            "card/07032",
            "Discard · Promise of Power",
            layout::investigator_discard(index),
            size,
            false,
        );
        for (slot, (code, title)) in [
            ("02038", "Internal Injury"),
            ("01160", "Ghoul Minion"),
            ("02038", "Internal Injury"),
            ("01160", "Ghoul Minion"),
            ("02038", "Internal Injury"),
        ]
        .into_iter()
        .enumerate()
        {
            card(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                &format!("card/{code}"),
                title,
                layout::investigator_threat(index, slot),
                size,
                false,
            );
        }
        for slot in 0..12 {
            let (code, title) = ASSETS[slot % ASSETS.len()];
            let at = layout::investigator_asset(index, slot);
            card(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                &format!("card/{code}"),
                title,
                at,
                size,
                slot == 7,
            );
            if slot == 7 {
                label(
                    &mut commands,
                    "Exhausted · 1 Damage",
                    at + Vec3::Z * 1.1,
                    105.0,
                );
                let _ = tabletop_harm_tokens::spawn(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    art.0.get("token/damage").cloned(),
                    at + Vec3::new(0.5, 0.03, 0.0),
                    false,
                );
            }
        }
        let count = [8, 9, 1][index];
        for slot in 0..count {
            let (code, title) = HAND[slot % HAND.len()];
            card(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                &format!("card/{code}"),
                title,
                layout::hand_position(index, count, slot),
                size,
                false,
            );
        }
        for horror in [false, true] {
            let at = layout::investigator_harm(index, horror);
            let role = if horror {
                "token/horror"
            } else {
                "token/damage"
            };
            let _ = tabletop_harm_tokens::spawn(
                &mut commands,
                &mut meshes,
                &mut materials,
                art.0.get(role).cloned(),
                at,
                horror,
            );
            let (amount, capacity) = if horror {
                ([0, 0, 2][index], [5, 7, 9][index])
            } else {
                (0, [9, 7, 6][index])
            };
            let count = format!("{amount}/{capacity}");
            label(
                &mut commands,
                if art.0.contains_key(role) {
                    count
                } else {
                    format!("{}\n{count}", if horror { "Horror" } else { "Damage" })
                },
                at + Vec3::new(0.0, 0.03, 0.55),
                58.0,
            );
        }
    }
    for (center, size) in [
        (Vec3::new(-4.0, 0.05, -3.8), Vec2::new(1.35, 1.89)),
        (Vec3::new(3.0, 0.05, -3.8), Vec2::new(1.8, 2.52)),
    ] {
        card(
            &mut commands,
            &mut meshes,
            &mut materials,
            &art,
            "card/01111",
            "Study · four-standee fit",
            center,
            size,
            false,
        );
        for slot in 0..4 {
            let code = INVESTIGATORS[slot % 3].0;
            let at = layout::location_standee_position(center.with_y(0.0), size, slot).unwrap();
            tabletop_standees::spawn(
                &mut commands,
                &mut meshes,
                &mut materials,
                art.0.get(&format!("standee/{code}")),
                at,
                layout::investigator_class(slot % 3),
                size,
            );
        }
    }
    for (name, at) in layout::test_zone_labels() {
        label(&mut commands, name, at, 180.0);
    }
    let size = Vec2::new(1.8, 2.52);
    card(
        &mut commands,
        &mut meshes,
        &mut materials,
        &art,
        "card/01111",
        "Test source · Study",
        layout::test_source(),
        size,
        false,
    );
    for slot in 0..6 {
        let (code, title) = HAND[slot % HAND.len()];
        card(
            &mut commands,
            &mut meshes,
            &mut materials,
            &art,
            &format!("card/{code}"),
            title,
            layout::test_commitment(6, slot),
            size,
            false,
        );
    }
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.45, 0.7))),
        MeshMaterial3d(materials.add(Color::srgb(0.18, 0.10, 0.16))),
        Transform::from_translation(layout::test_bag()),
    ));
    label(
        &mut commands,
        "Chaos bag",
        layout::test_bag() + Vec3::Z * 0.8,
        100.0,
    );
    for canceled in [false, true] {
        for (slot, (role, name)) in [
            ("token/minus_one", "-1"),
            ("token/zero", "0"),
            ("token/curse", "Curse"),
        ]
        .into_iter()
        .enumerate()
        {
            let at = layout::test_token(canceled, slot);
            commands.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.46, 0.12))),
                MeshMaterial3d(materials.add(if canceled {
                    Color::srgb(0.37, 0.25, 0.28)
                } else {
                    Color::srgb(0.86, 0.69, 0.37)
                })),
                Transform::from_translation(at),
            ));
            if let Some(image) = art.0.get(role) {
                commands.spawn((
                    Mesh3d(meshes.add(Plane3d::default().mesh().size(0.88, 0.88))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color_texture: Some(image.clone()),
                        unlit: true,
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    })),
                    Transform::from_translation(at + Vec3::Y * 0.065),
                ));
            }
            label(
                &mut commands,
                format!("{name}\n{}", if canceled { "Canceled" } else { "Revealed" }),
                at + Vec3::Z * 0.7,
                68.0,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn navigate(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    window: Single<&Window>,
    mut orbit: ResMut<Orbit>,
    mut camera: Single<&mut Transform, With<Camera3d>>,
    mut next: Local<usize>,
) {
    for (slot, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
    ]
    .into_iter()
    .enumerate()
    {
        if keys.just_pressed(key) {
            orbit.focus(layout::focus(VIEWS[slot]));
        }
    }
    if keys.just_pressed(KeyCode::Tab) {
        orbit.focus(layout::focus(VIEWS[*next]));
        *next = (*next + 1) % VIEWS.len();
    }
    **camera = orbit
        .step(FrameInput {
            keys: &keys,
            buttons: &buttons,
            window: &window,
            scroll_y: scroll.delta.y,
            dt: time.delta_secs(),
            capture: default(),
            reduced_motion: true,
        })
        .transform;
}

fn position_labels(
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut labels: Query<(&Label, &mut Node, &mut Visibility)>,
) {
    for (label, mut node, mut visible) in &mut labels {
        if let Ok(point) = camera.0.world_to_viewport(camera.1, label.0) {
            node.left = px(point.x - label.1 / 2.0);
            node.top = px(point.y);
            *visible = Visibility::Visible;
        } else {
            *visible = Visibility::Hidden;
        }
    }
}

fn capture(
    mut commands: Commands,
    mut capture: ResMut<Capture>,
    art: Res<Art>,
    server: Res<AssetServer>,
    capturing: Query<(), With<Capturing>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !art
        .0
        .values()
        .all(|image| server.is_loaded_with_dependencies(image.id()))
    {
        return;
    }
    capture.frame += 1;
    if !capture.requested
        && capture.frame >= 180
        && let Some(path) = &capture.path
    {
        if let Some(parent) = path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            eprintln!("Cannot create screenshot directory: {error}");
            exit.write(AppExit::error());
            return;
        }
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        capture.requested = true;
    }
    if capture.quit
        && capture.requested
        && capture.frame > 185
        && capturing.is_empty()
        && capture.path.as_ref().is_some_and(|path| path.is_file())
    {
        exit.write(AppExit::Success);
    }
}
