//! Scenario entry uses no scenario artwork and constructs no rules session.
use bevy::{
    input_focus::{
        AutoFocus, InputFocus,
        tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
    },
    picking::hover::Hovered,
    prelude::*,
    render::view::screenshot::Screenshot,
    ui_widgets::{Activate, Button},
};
use std::{
    ffi::OsString,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy)]
enum Action {
    Select,
    Back,
    New,
    Continue,
    Open,
    Quit,
    Import,
    CancelImport,
}

#[derive(Resource, Default)]
struct Selection {
    selected: bool,
    notice: String,
    remembered: Option<PathBuf>,
    root: Option<PathBuf>,
    next: std::sync::Arc<std::sync::Mutex<Option<(String, PathBuf)>>>,
    actions: Vec<Action>,
    screenshot: Option<PathBuf>,
    quit_after_screenshot: bool,
    frames: u32,
    imported_root: std::sync::Arc<std::sync::Mutex<Option<PathBuf>>>,
    importing: bool,
    quit_after_import: bool,
    artwork_status: String,
}
#[derive(Component)]
struct Panel;

#[derive(Resource)]
struct FilePicker(fn() -> Result<Option<PathBuf>, String>);

impl Default for FilePicker {
    fn default() -> Self {
        Self(open_file)
    }
}

pub fn run() -> Result<(), String> {
    let mut state = Selection::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--entry" if args.next().as_deref() == Some("selection") => {}
            "--verification-attempts" => {
                crate::attempts::verification_directory()?
                    .ok_or("Verification requires its disposable attempts directory.")?;
            }
            "--art-root" => state.root = Some(args.next().ok_or("--art-root needs a path")?.into()),
            "--selection-notice" => state.notice = args.next().ok_or("Missing notice")?,
            "--screenshot" => {
                state.screenshot = Some(args.next().ok_or("--screenshot needs a path")?.into())
            }
            "--quit-after-screenshot" => state.quit_after_screenshot = true,
            "--help" | "-h" => {
                println!(
                    "Arkham Unfolded\nOrdinary launch opens scenario selection.\n--entry selection\n--art-root PREPARED_FOLDER\n--screenshot PATH\n--quit-after-screenshot\nDirect play: --experiment gathering --save PATH\nOnly The Gathering (in development), Standard, with Tony Morgan, Patrice Hathaway and Jacqueline Fine is available."
                );
                return Ok(());
            }
            _ => {
                return Err(format!(
                    "Unknown selection option: {arg}. Use --experiment gathering for direct play."
                ));
            }
        }
    }
    if state.quit_after_screenshot && state.screenshot.is_none() {
        return Err("--quit-after-screenshot requires --screenshot PATH".into());
    }
    match crate::attempts::reference_path().and_then(|p| crate::attempts::recalled(&p)) {
        Ok(path) => state.remembered = path,
        Err(error) => state.notice = format!("{}\n{error}", state.notice),
    }
    let next = state.next.clone();
    let artwork = crate::tabletop_support::asset_settings::AssetSettings::new_gathering(
        crate::asset_config::AssetConfig::load_gathering()?,
    );
    refresh_artwork_status(&mut state, &artwork);
    let root = state.root.clone();
    let imported_root = state.imported_root.clone();
    let mut app = App::new();
    app.insert_resource(state)
        .insert_resource(artwork)
        .insert_resource(ClearColor(Color::srgb(0.035, 0.055, 0.065)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Arkham Unfolded — Scenario selection".into(),
                resolution: (1200, 800).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TabNavigationPlugin);
    install(&mut app);
    let exit = app.run();
    if let Some((intent, save)) = &*next.lock().expect("selection request") {
        let mut args: Vec<OsString> = vec![
            "--experiment".into(),
            "gathering".into(),
            "--attempt".into(),
            intent.into(),
            "--save".into(),
            save.into(),
        ];
        let effective_root = imported_root
            .lock()
            .expect("imported artwork")
            .clone()
            .or(root);
        if let Some(root) = &effective_root {
            args.extend(["--art-root".into(), root.into()]);
        }
        return replace(command(args)?);
    }
    crate::tabletop_health::exit_result(exit)
}

fn install(app: &mut App) {
    app.init_resource::<FilePicker>()
        .add_message::<AppExit>()
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(Update, (handle, render, button_feedback, capture).chain());
}

pub(crate) fn return_to_selection(root: Option<&Path>, notice: Option<&str>) -> Result<(), String> {
    replace(return_command(root, notice)?)
}

pub(crate) fn return_command(root: Option<&Path>, notice: Option<&str>) -> Result<Command, String> {
    let mut args: Vec<OsString> = vec!["--entry".into(), "selection".into()];
    if let Some(root) = root {
        args.extend(["--art-root".into(), root.into()]);
    }
    if let Some(notice) = notice {
        args.extend(["--selection-notice".into(), notice.into()]);
    }
    command(args)
}

fn command(args: Vec<OsString>) -> Result<Command, String> {
    let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    command.args(args);
    Ok(command)
}

pub(crate) fn replace(mut command: Command) -> Result<(), String> {
    let error = command.exec();
    Err(format!("Could not open the requested screen: {error}"))
}

fn refresh_artwork_status(
    state: &mut Selection,
    artwork: &crate::tabletop_support::asset_settings::AssetSettings,
) {
    state.artwork_status = artwork.status();
    if state
        .imported_root
        .lock()
        .expect("imported artwork")
        .is_none()
        && let Some(root) = &state.root
    {
        state
            .artwork_status
            .push_str(&format!("\nTemporary launch override: {}", root.display()));
    }
}

fn handle(
    mut state: ResMut<Selection>,
    picker: Res<FilePicker>,
    mut exit: MessageWriter<AppExit>,
    mut artwork: Option<ResMut<crate::tabletop_support::asset_settings::AssetSettings>>,
) {
    if let Some(artwork) = artwork.as_mut()
        && let Some(message) = artwork.poll()
    {
        state.notice = message;
        state.importing = false;
        if let Some(root) = artwork.take_imported_root() {
            *state.imported_root.lock().expect("imported artwork") = Some(root);
        }
        refresh_artwork_status(&mut state, artwork);
        if state.quit_after_import {
            exit.write(AppExit::Success);
            return;
        }
    }
    if state.actions.is_empty() {
        return;
    }
    let actions = std::mem::take(&mut state.actions);
    let Some(action) = actions.first() else {
        return;
    };
    if state.next.lock().expect("selection request").is_some() {
        return;
    }
    if state.importing {
        if matches!(action, Action::CancelImport | Action::Quit) {
            state.quit_after_import |= matches!(action, Action::Quit);
            if let Some(artwork) = artwork.as_ref() {
                artwork.cancel_import();
            }
            state.notice = "Canceling import. Close the file picker if it is still open.".into();
        }
        return;
    }
    let request = match action {
        Action::Import => {
            if let Some(artwork) = artwork.as_mut() {
                artwork.import();
                state.importing = true;
                state.notice = format!(
                    "Choose the {} ZIP. Importing verifies every required file; you can cancel safely.",
                    crate::assets::package::required_collection()
                );
            }
            return;
        }
        Action::CancelImport => return,
        Action::Select => {
            state.selected = true;
            return;
        }
        Action::Back => {
            state.selected = false;
            return;
        }
        Action::Quit => {
            exit.write(AppExit::Success);
            return;
        }
        Action::New => crate::attempts::unused_path().map(|p| Some(("new".to_owned(), p))),
        Action::Continue => state
            .remembered
            .clone()
            .map(|p| Some(("open".to_owned(), p)))
            .ok_or_else(|| {
                "No remembered attempt. Choose New attempt or Open saved attempt.".to_owned()
            }),
        Action::Open => (picker.0)().map(|p| p.map(|p| ("open".to_owned(), p))),
    };
    match request {
        Ok(Some((intent, path))) => {
            if let Err(error) = crate::attempts::verification_path(&path) {
                state.notice = error;
                return;
            }
            // Validate explicit opens while selection is still available. Native
            // startup checks again; racing changes cannot become a new attempt.
            if intent == "open" {
                match crate::gathering::continuity::load_session(&path) {
                    Ok(session) if !session.is_verification_fixture() => {}
                    Ok(_) => {
                        state.notice = "Verification fixtures cannot be opened for play.".into();
                        return;
                    }
                    Err(error) => {
                        state.notice = format!("Cannot open {}: {error}", path.display());
                        return;
                    }
                }
            }
            *state.next.lock().expect("selection request") = Some((intent, path));
            exit.write(AppExit::Success);
        }
        Ok(None) => {}
        Err(error) => state.notice = error,
    }
}

fn open_file() -> Result<Option<PathBuf>, String> {
    let output = Command::new("zenity")
        .args([
            "--file-selection",
            "--title=Open saved Gathering attempt",
            "--file-filter=Saved attempts | *.json",
            "--file-filter=All files | *",
        ])
        .output()
        .map_err(|e| format!("Cannot open the file chooser: {e}"))?;
    if output.status.success() {
        let path = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
        let path = path.strip_suffix('\n').unwrap_or(&path);
        if path.is_empty() {
            return Ok(None);
        }
        Ok(Some(path.into()))
    } else if output.status.code() == Some(1) && output.stderr.is_empty() {
        Ok(None)
    } else {
        Err(format!(
            "File chooser failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn render(mut commands: Commands, state: Res<Selection>, panels: Query<Entity, With<Panel>>) {
    if !state.is_changed() && !panels.is_empty() {
        return;
    }
    for panel in &panels {
        commands.entity(panel).despawn();
    }
    commands.spawn((
        Panel,
        TabGroup::default(),
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            row_gap: px(18),
            padding: UiRect::all(px(30)),
            ..default()
        },
    )).with_children(|p| {
        p.spawn((
            Text::new("Arkham Unfolded"),
            TextFont::from_font_size(36.),
            TextColor(Color::srgb(0.86, 0.69, 0.37)),
        ));
        p.spawn((
            Text::new("The Gathering / In development"),
            TextFont::from_font_size(26.),
        ));
        p.spawn((
            Text::new("Standard / Tony Morgan / Patrice Hathaway / Jacqueline Fine\nFixed starting decks"),
            TextFont::from_font_size(18.),
        ));
        if state.importing {
            button(p, "Cancel import", Action::CancelImport, true);
        } else if state.selected {
            button(p, "New attempt", Action::New, true);
            button(p, "Continue", Action::Continue, false);
            let remembered = state.remembered.as_ref()
                .map(|p| format!("Continue: {}", p.display()))
                .unwrap_or_else(|| "Continue unavailable: no remembered attempt.".into());
            p.spawn((Text::new(remembered), TextFont::from_font_size(14.)));
            button(p, "Open saved attempt", Action::Open, false);
            button(p, "Back", Action::Back, false);
        } else {
            button(p, "Select The Gathering", Action::Select, true);
        }
        if !state.importing { button(p, "Import artwork", Action::Import, false); }
        if !state.artwork_status.is_empty() {
            p.spawn((Text::new(&state.artwork_status), TextFont::from_font_size(14.)));
        }
        if !state.notice.is_empty() {
            p.spawn((
                Text::new(&state.notice),
                TextFont::from_font_size(16.),
                TextColor(Color::srgb(1., 0.75, 0.5)),
            ));
        }
        button(p, "Quit", Action::Quit, false);
    });
}

fn button(parent: &mut ChildSpawnerCommands, label: &str, action: Action, focus: bool) {
    let mut entity = parent.spawn((
        Button,
        TabIndex(0),
        Hovered::default(),
        AccessibleLabel::new(label),
        Outline::new(px(2), px(2), Color::NONE),
        Node {
            padding: UiRect::axes(px(24), px(12)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.12, 0.25, 0.26)),
    ));
    if focus {
        entity.insert(AutoFocus);
    }
    entity
        .with_child((Text::new(label), TextFont::from_font_size(20.)))
        .observe(move |_: On<Activate>, mut state: ResMut<Selection>| {
            state.actions.push(action);
        });
}

fn button_feedback(
    focus: Option<Res<InputFocus>>,
    mut buttons: Query<(Entity, &Hovered, &mut BackgroundColor, &mut Outline), With<Button>>,
) {
    for (entity, hovered, mut background, mut outline) in &mut buttons {
        let focused = focus
            .as_ref()
            .is_some_and(|focus| focus.get() == Some(entity));
        background.0 = if hovered.get() || focused {
            Color::srgb(0.24, 0.40, 0.39)
        } else {
            Color::srgb(0.12, 0.25, 0.26)
        };
        outline.color = if focused {
            Color::srgb(0.86, 0.69, 0.37)
        } else {
            Color::NONE
        };
    }
}

fn capture(mut commands: Commands, mut state: ResMut<Selection>, time: Res<Time>) {
    if state.screenshot.is_none() {
        return;
    }
    state.bypass_change_detection().frames += 1;
    if state.frames < 90 {
        return;
    }
    let path = state.screenshot.take().unwrap();
    let mut shot = commands.spawn(Screenshot::primary_window());
    if state.quit_after_screenshot {
        shot.observe(crate::tabletop_capture::save_selection_and_exit(
            path,
            time.elapsed(),
        ));
    } else {
        shot.observe(crate::tabletop_capture::save_to_disk(path, time.elapsed()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        asset::RenderAssetUsages,
        render::{
            render_resource::{Extent3d, TextureDimension, TextureFormat},
            view::screenshot::ScreenshotCaptured,
        },
    };

    fn host(state: Selection) -> App {
        let mut app = App::new();
        app.insert_resource(state).init_resource::<Time>();
        install(&mut app);
        app
    }

    #[test]
    fn quit_during_import_waits_for_the_worker_then_exits() {
        let directory = tempfile::tempdir().unwrap();
        let settings = crate::tabletop_support::asset_settings::AssetSettings::new_gathering(
            crate::asset_config::AssetConfig::load_at(
                directory.path().join("settings"),
                directory.path().join("assets"),
            ),
        );
        let (settings, pending) = settings.with_pending_selection();
        let mut app = host(Selection {
            importing: true,
            actions: vec![Action::Quit],
            ..default()
        });
        app.insert_resource(settings);
        app.update();
        assert!(
            app.world()
                .resource::<Messages<AppExit>>()
                .iter_current_update_messages()
                .next()
                .is_none()
        );
        drop(pending);
        app.update();
        assert!(
            app.world()
                .resource::<Messages<AppExit>>()
                .iter_current_update_messages()
                .any(AppExit::is_success)
        );
    }

    #[test]
    fn failed_import_keeps_the_effective_override_visible() {
        let directory = tempfile::tempdir().unwrap();
        let settings = crate::tabletop_support::asset_settings::AssetSettings::new_gathering(
            crate::asset_config::AssetConfig::load_at(
                directory.path().join("settings"),
                directory.path().join("assets"),
            ),
        );
        let (settings, pending) = settings.with_pending_selection();
        let mut app = host(Selection {
            importing: true,
            root: Some(PathBuf::from("/temporary-artwork-override")),
            ..default()
        });
        app.insert_resource(settings);
        drop(pending);
        app.update();
        assert!(
            app.world()
                .resource::<Selection>()
                .artwork_status
                .contains("/temporary-artwork-override")
        );
        assert!(
            app.world()
                .resource::<Selection>()
                .imported_root
                .lock()
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn selection_captions_use_supported_glyphs_without_prepared_artwork() {
        for selected in [false, true] {
            let mut app = crate::tabletop_layout::heading_checks::app();
            app.insert_resource(Selection {
                selected,
                ..default()
            })
            .init_resource::<Time>();
            install(&mut app);
            app.update();
            let world = app.world_mut();
            assert!(
                world
                    .query::<&Text>()
                    .iter(world)
                    .any(|text| text.0.starts_with("The Gathering"))
            );
            crate::tabletop_layout::font_checks::assert_supported(world);
        }
    }

    #[test]
    fn selection_capture_finishes_without_a_play_admission_resource() {
        let directory = tempfile::tempdir().unwrap();
        for (name, succeeds) in [("selection.png", true), ("bad.unsupported", false)] {
            let path = directory.path().join(name);
            let mut app = host(Selection {
                screenshot: Some(path.clone()),
                quit_after_screenshot: true,
                ..default()
            });
            for _ in 0..90 {
                app.update();
            }
            let world = app.world_mut();
            assert!(!world.contains_resource::<crate::tabletop_health::RenderHealth>());
            assert!(!world.contains_resource::<AssetServer>());
            let entity = world
                .query_filtered::<Entity, With<Screenshot>>()
                .single(world)
                .unwrap();
            world.trigger(ScreenshotCaptured {
                entity,
                image: Image::new_fill(
                    Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    &[255, 0, 0, 255],
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::MAIN_WORLD,
                ),
            });
            let exits: Vec<_> = world
                .resource::<Messages<AppExit>>()
                .iter_current_update_messages()
                .collect();
            assert_eq!(exits.len(), 1);
            assert_eq!(exits[0].is_success(), succeeds);
            assert_eq!(path.exists(), succeeds);
            assert!(world.resource::<Selection>().next.lock().unwrap().is_none());
        }
    }

    #[test]
    fn browsing_and_cancelled_open_preserve_the_remembered_attempt_without_loading_art() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("not-created.json");
        let mut app = host(Selection {
            remembered: Some(missing.clone()),
            ..default()
        });
        app.insert_resource(FilePicker(|| Ok(None)));
        for action in [Action::Select, Action::Open, Action::Back] {
            app.world_mut()
                .resource_mut::<Selection>()
                .actions
                .push(action);
            app.update();
        }
        let state = app.world().resource::<Selection>();
        assert!(!state.selected);
        assert_eq!(state.remembered.as_ref(), Some(&missing));
        assert!(state.next.lock().unwrap().is_none());
        assert!(!missing.exists());
        assert!(!app.world().contains_resource::<AssetServer>());
        assert!(app.world().resource::<Messages<AppExit>>().is_empty());
    }

    #[test]
    fn unavailable_or_invalid_continue_keeps_selection_and_file_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let save = dir.path().join("attempt.json");
        for bytes in [None, Some(b"invalid save".as_slice())] {
            if let Some(bytes) = bytes {
                std::fs::write(&save, bytes).unwrap();
            }
            let mut app = host(Selection {
                selected: true,
                remembered: Some(save.clone()),
                actions: vec![Action::Continue],
                ..default()
            });
            app.update();
            let state = app.world().resource::<Selection>();
            assert!(state.selected);
            assert_eq!(state.remembered.as_ref(), Some(&save));
            assert!(state.next.lock().unwrap().is_none());
            assert!(state.notice.contains("Cannot open"));
            assert_eq!(std::fs::read(&save).ok().as_deref(), bytes);
        }
        let mut app = host(Selection {
            actions: vec![Action::Continue],
            ..default()
        });
        app.update();
        assert!(
            app.world()
                .resource::<Selection>()
                .notice
                .contains("No remembered attempt")
        );
    }

    #[test]
    fn open_request_survives_app_runner_ownership_and_repeated_activation() {
        let dir = tempfile::tempdir().unwrap();
        let save = dir.path().join("attempt.json");
        let session = crate::gathering::Session::new().unwrap();
        crate::gathering::continuity::save_session(&save, &session).unwrap();
        let bytes = std::fs::read(&save).unwrap();
        let state = Selection {
            selected: true,
            remembered: Some(save.clone()),
            actions: vec![Action::Continue, Action::New],
            ..default()
        };
        let next = state.next.clone();
        let mut app = host(state);
        app.set_runner(|mut app| {
            app.update();
            assert!(app.world().resource::<Selection>().actions.is_empty());
            app.world_mut()
                .resource_mut::<Selection>()
                .actions
                .push(Action::New);
            app.update();
            AppExit::Success
        });
        assert_eq!(app.run(), AppExit::Success);
        assert!(!app.world().contains_resource::<Selection>());
        assert_eq!(*next.lock().unwrap(), Some(("open".into(), save.clone())));
        assert_eq!(std::fs::read(save).unwrap(), bytes);
    }
}
