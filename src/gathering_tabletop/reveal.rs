//! Gathering's offered reveal routes, using the shared screen-fixed control.
//! This adapter never commits a choice or moves the camera. The parent's normal
//! save-before-install branch owns both, and focuses the resolution after success.
use super::*;
use crate::tabletop_reveal::{Artwork, Shortcut};

pub(super) fn announcement(view: &Observation) -> Option<String> {
    let tokens = view.play.as_ref()?.tokens.as_ref()?;
    let sealed = tokens.pending_sealed()?;
    let faces = sealed
        .faces
        .iter()
        .map(|face| match face {
            crate::TokenFace::Skull => "[skull]",
            crate::TokenFace::Cultist => "[cultist]",
            crate::TokenFace::Tablet => "[tablet]",
            crate::TokenFace::AutoFail => "[auto_fail]",
            crate::TokenFace::ElderSign => "[elder_sign]",
            crate::TokenFace::Curse => "[curse]",
            _ => face.label(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut label = format!("Resolve sealed {faces} from Premonition");
    match tokens.fresh_reveal_count() {
        Some(0) => {}
        Some(count) => label.push_str(&format!(
            "; reveal {count} additional token{}",
            if count == 1 { "" } else { "s" }
        )),
        None => label.push_str("; reveal additional tokens as declared"),
    }
    Some(label)
}

fn offered(view: &Observation) -> Vec<crate::Choice> {
    if !view.choices.iter().any(|choice| choice.id == "test:reveal") {
        return vec![];
    }
    controls::source_choices(view, &controls::Source::Bag)
}

pub(super) fn refresh(
    table: &Table,
    settings: Option<&AssetSettings>,
    health: &RenderHealth,
    shortcut: &mut Shortcut,
) {
    let view = table.view();
    shortcut.sync_offered(
        table.epoch,
        view.revision,
        &offered(&view),
        true && !table.modal()
            && !table.menu_open
            && !table.is_presenting()
            && health.allows_play()
            && !health.just_became_ready()
            && !settings.is_some_and(AssetSettings::is_busy),
        table.reduced_motion,
    );
}

pub(super) fn sync(
    table: Res<Table>,
    settings: Option<Res<AssetSettings>>,
    health: Res<RenderHealth>,
    art: Res<art::Art>,
    mut shortcut: ResMut<Shortcut>,
    mut artwork: ResMut<Artwork>,
) {
    artwork.0 = art.image(crate::tabletop_reveal::ART_ROLE);
    shortcut.attention_cue(true);
    shortcut.caption(
        Some(
            if announcement(&table.view()).is_some() {
                "Resolve tokens"
            } else {
                "Reveal tokens"
            }
            .into(),
        ),
        art.symbol_font().clone(),
    );
    refresh(&table, settings.as_deref(), &health, &mut shortcut);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gathering::{HostileFixture, TokenFixture};

    fn declared_reveal() -> Session {
        let mut session =
            Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::JacquelineOlive));
        for _ in 0..4 {
            let view = session.observe();
            if view.choices.iter().any(|choice| choice.id == "test:reveal") {
                return session;
            }
            let id = if view
                .choices
                .iter()
                .any(|choice| choice.id == "commit:confirm")
            {
                "commit:confirm"
            } else {
                "window:pass"
            };
            session.choose(view.revision, id).unwrap();
        }
        panic!("The declared reveal must be reached through the actual choices");
    }

    fn ordinary_reveal() -> Session {
        let mut session = Session::with_seed(89);
        for id in [
            "lead:tony",
            "confirm-mulligan",
            "confirm-mulligan",
            "confirm-mulligan",
            "turn:tony",
            "action:ordinary:investigate",
        ] {
            session.choose(session.observe().revision, id).unwrap();
        }
        for _ in 0..8 {
            let view = session.observe();
            if view.choices.iter().any(|choice| choice.id == "test:reveal") {
                return session;
            }
            let id = if view
                .choices
                .iter()
                .any(|choice| choice.id == "commit:confirm")
            {
                "commit:confirm"
            } else {
                "window:pass"
            };
            session.choose(view.revision, id).unwrap();
        }
        panic!("Ordinary setup did not reach its offered reveal");
    }

    fn sealed_reveal() -> Session {
        let mut session =
            Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::PremonitionVoice));
        session
            .choose(session.observe().revision, "test:reveal")
            .unwrap();
        let voice = session.observe().play.unwrap().players[2]
            .hand
            .iter()
            .find(|card| card.code == "60416")
            .unwrap()
            .id;
        session
            .choose(
                session.observe().revision,
                &format!("action:ordinary:play:spell:{voice}"),
            )
            .unwrap();
        session
    }

    #[test]
    fn sealed_reveal_preserves_the_button_gap_and_keeps_its_explanation_in_the_panel() {
        let directory = tempfile::tempdir().unwrap();
        let session = sealed_reveal();
        let announcement = announcement(&session.observe()).unwrap();
        let footer_label = session
            .observe()
            .choices
            .iter()
            .find(|choice| choice.id == "test:reveal")
            .unwrap()
            .label
            .clone();
        let mut app = host(session, directory.path().join("sealed-layout.json"));
        for (size, scale) in [(Vec2::new(1600., 1000.), 1.), (Vec2::new(960., 540.), 1.25)] {
            crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, scale);
            app.world_mut().resource_mut::<Table>().dirty = true;
            for _ in 0..4 {
                app.update();
            }
            let footer = button(&mut app, &footer_label).unwrap();
            let world = app.world_mut();
            let (reveal, node, at) = world
            .query_filtered::<(Entity, &ComputedNode, &UiGlobalTransform), With<crate::tabletop_reveal::RevealButton>>()
            .single(world).unwrap();
            let right = at.transform_point2(node.size() / 2.).x;
            let mut panel = footer;
            while world.get::<Panel>(panel).is_none() {
                panel = world.get::<ChildOf>(panel).unwrap().parent();
            }
            let panel_node = world.get::<ComputedNode>(panel).unwrap();
            let panel_at = world.get::<UiGlobalTransform>(panel).unwrap();
            let left = panel_at.transform_point2(-panel_node.size() / 2.).x;
            assert!(
                ((left - right) / scale - 16.).abs() < 0.1,
                "Preserve 16px gap: button right {right}, panel left {left}"
            );
            assert_eq!(
                world.get::<AccessibleLabel>(reveal).unwrap().0,
                "Resolve tokens"
            );
            let explanation = world
                .query::<(Entity, &AccessibleLabel)>()
                .iter(world)
                .find_map(|(entity, label)| (label.0 == announcement).then_some(entity))
                .expect("Keep the known faces and additional draws visible");
            let mut ancestor = explanation;
            while ancestor != panel {
                ancestor = world
                    .get::<ChildOf>(ancestor)
                    .expect("Explanation belongs to the adjacent panel")
                    .parent();
            }
            super::super::tests::assert_unclipped_button(&mut app, &footer_label, size, scale);
        }
    }

    #[test]
    fn reveal_control_compacts_the_panel_and_explains_jacquelines_assistance() {
        let directory = tempfile::tempdir().unwrap();
        let session = ordinary_reveal();
        let mut expected_session = session.clone();
        let before = session.observe();
        let expected = before
            .choices
            .iter()
            .filter(|choice| choice.id.starts_with("tokens:jacqueline"))
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            expected.len() >= 2,
            "Historical engine choice IDs remain compatible"
        );
        let mut app = host(session, directory.path().join("ability-layout.json"));
        crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
            &mut app,
            Vec2::new(1600., 1000.),
            1.,
        );
        app.world_mut().resource_mut::<Table>().dirty = true;
        for _ in 0..4 {
            app.update();
        }
        let footer = button(
            &mut app,
            &before
                .choices
                .iter()
                .find(|choice| choice.id == "test:reveal")
                .unwrap()
                .label,
        )
        .unwrap();
        let mut panel = footer;
        while app.world().get::<Panel>(panel).is_none() {
            panel = app.world().get::<ChildOf>(panel).unwrap().parent();
        }
        assert!(
            app.world().get::<ComputedNode>(panel).unwrap().size().y < 300.,
            "Short token options must fit their content"
        );
        assert_eq!(app.world().resource::<Table>().session, expected_session);
        let text = app
            .world_mut()
            .query::<&AccessibleLabel>()
            .iter(app.world())
            .map(|label| label.0.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Tony Morgan is testing"), "{text}");
        assert!(text.contains("at the same location"), "{text}");
        assert!(text.contains("Add 2 tokens"), "{text}");
        for choice in &expected {
            assert!(
                button(&mut app, &choice.label).is_none(),
                "Redundant alias shown: {}",
                choice.id
            );
        }
        click(&mut app, "Use Jacqueline — reveal 3 tokens");
        expected_session
            .choose(before.revision, &expected[0].id)
            .unwrap();
        assert_eq!(app.world().resource::<Table>().session, expected_session);
    }

    fn host(session: Session, path: PathBuf) -> App {
        let mut app = super::super::tests::host(path.clone());
        let mut table = Table::new(session, path);
        table.reduced_motion = true;
        app.insert_resource(table);
        app.world_mut()
            .resource_mut::<Orbit>()
            .focus(layout::focus(Area::Investigator(0)));
        app.update();
        app
    }

    fn button(app: &mut App, label: &str) -> Option<Entity> {
        super::super::tests::choice_button(app, label)
    }

    fn click(app: &mut App, label: &str) {
        let entity =
            button(app, label).unwrap_or_else(|| panic!("Missing rendered button {label}"));
        app.world_mut().trigger(Activate { entity });
        app.update();
    }

    fn key(app: &mut App, code: KeyCode, logical_key: bevy::input::keyboard::Key, pressed: bool) {
        let world = app.world_mut();
        let window = world
            .query_filtered::<Entity, With<Window>>()
            .single(world)
            .unwrap();
        world.write_message(bevy::input::keyboard::KeyboardInput {
            key_code: code,
            logical_key,
            state: if pressed {
                bevy::input::ButtonState::Pressed
            } else {
                bevy::input::ButtonState::Released
            },
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }

    #[test]
    fn reveal_chooser_keyboard_focus_and_escape_preserve_camera_and_decision() {
        use bevy::input::keyboard::Key;
        let directory = tempfile::tempdir().unwrap();
        let session = ordinary_reveal();
        let label = session
            .observe()
            .choices
            .iter()
            .find(|choice| choice.id == "test:reveal")
            .unwrap()
            .label
            .clone();
        let mut app = host(session.clone(), directory.path().join("keyboard.json"));
        let camera = app.world().resource::<Orbit>().pose();
        click(&mut app, "Reveal tokens");
        let first = button(&mut app, &label).unwrap();
        let focused = app.world().resource::<InputFocus>().get();
        assert_eq!(
            focused,
            Some(first),
            "Focused control: {:?}",
            focused.and_then(|entity| app.world().get::<AccessibleLabel>(entity))
        );
        key(&mut app, KeyCode::Tab, Key::Tab, true);
        assert_ne!(app.world().resource::<InputFocus>().get(), Some(first));
        key(&mut app, KeyCode::Tab, Key::Tab, false);
        key(&mut app, KeyCode::Home, Key::Home, true);
        key(&mut app, KeyCode::Home, Key::Home, false);
        assert_eq!(app.world().resource::<Orbit>().pose(), camera);
        key(&mut app, KeyCode::Escape, Key::Escape, true);
        key(&mut app, KeyCode::Escape, Key::Escape, false);
        assert!(!app.world().resource::<Shortcut>().is_open());
        assert_eq!(app.world().resource::<Table>().session, session);
        assert_eq!(app.world().resource::<Orbit>().pose(), camera);
    }

    #[test]
    fn held_pan_preserves_the_reveal_button_and_dismisses_only_an_open_chooser() {
        use bevy::input::keyboard::Key;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("pan.json");
        let session = ordinary_reveal();
        let mut app = host(session.clone(), path.clone());
        let reveal = button(&mut app, "Reveal tokens").unwrap();
        // Start with the renderer's actual control focused; keyboard input and
        // activation below use the normal plugins and registered observers.
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(reveal, bevy::input_focus::FocusCause::Navigated);
        app.world_mut()
            .resource_mut::<PresentationTime>()
            .advance_by(std::time::Duration::from_millis(100));
        key(&mut app, KeyCode::KeyW, Key::Character("w".into()), true);
        for _ in 0..5 {
            let before = app.world().resource::<Orbit>().pose();
            app.update();
            assert!(app.world().resource::<Orbit>().pose().target.z < before.target.z);
            let focused = app.world().resource::<InputFocus>().get();
            assert!(
                focused.is_some(),
                "Panning must retain the focused Reveal control"
            );
            assert_eq!(focused, button(&mut app, "Reveal tokens"));
        }
        key(&mut app, KeyCode::KeyW, Key::Character("w".into()), false);
        key(&mut app, KeyCode::Enter, Key::Enter, true);
        key(&mut app, KeyCode::Enter, Key::Enter, false);
        assert!(app.world().resource::<Shortcut>().is_open());
        key(&mut app, KeyCode::KeyW, Key::Character("w".into()), true);
        assert!(!app.world().resource::<Shortcut>().is_open());
        key(&mut app, KeyCode::KeyW, Key::Character("w".into()), false);
        assert_eq!(app.world().resource::<Table>().session, session);
        assert!(
            !path.exists(),
            "Opening/dismissing a chooser cannot reveal a token"
        );
    }

    #[test]
    fn failed_health_removes_live_reveal_input() {
        let directory = tempfile::tempdir().unwrap();
        let session = ordinary_reveal();
        let mut app = host(session.clone(), directory.path().join("read-only.json"));
        app.insert_resource(RenderHealth::failed_for_test());
        app.update();
        assert!(button(&mut app, "Reveal tokens").is_none());
        assert_eq!(app.world().resource::<Table>().session, session);
    }

    #[test]
    fn keyboard_reveal_records_the_offered_replacement_id() {
        use bevy::input::keyboard::Key;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("replacement.json");
        let session = ordinary_reveal();
        let choices = offered(&session.observe());
        let selected = choices
            .iter()
            .position(|choice| choice.id.starts_with("tokens:jacqueline"))
            .expect("Jacqueline is eligible in this ordinary investigation");
        let revision = session.observe().revision;
        let mut app = host(session, path.clone());
        click(&mut app, "Reveal tokens");
        for _ in 0..selected {
            key(&mut app, KeyCode::Tab, Key::Tab, true);
            key(&mut app, KeyCode::Tab, Key::Tab, false);
        }
        let chosen = button(&mut app, &choices[selected].label).unwrap();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(chosen));
        key(&mut app, KeyCode::Enter, Key::Enter, true);
        key(&mut app, KeyCode::Enter, Key::Enter, false);
        let table = app.world().resource::<Table>();
        assert_eq!(table.session.observe().revision, revision + 1);
        assert_eq!(continuity::load_session(&path).unwrap(), table.session);
        let record = serde_json::to_value(&table.session).unwrap();
        assert_eq!(
            record["decisions"].as_array().unwrap().last().unwrap(),
            choices[selected].id.as_str()
        );
        assert_eq!(
            app.world().resource::<Orbit>().pose().target,
            layout::focus(Area::Test).0
        );
    }

    #[test]
    fn rendered_reveal_saves_the_original_choice_once_then_focuses_test_and_reopens() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ordinary.json");
        let session = ordinary_reveal();
        continuity::save_session(&path, &session).unwrap();
        let before = session.observe();
        let label = before
            .choices
            .iter()
            .find(|choice| choice.id == "test:reveal")
            .unwrap()
            .label
            .clone();
        let mut app = host(session.clone(), path.clone());
        let camera = app.world().resource::<Orbit>().pose();
        click(&mut app, "Reveal tokens");
        assert_eq!(app.world().resource::<Table>().session, session);
        assert_eq!(app.world().resource::<Orbit>().pose(), camera);
        let entity = button(&mut app, &label).unwrap();
        app.world_mut().trigger(Activate { entity });
        app.world_mut().trigger(Activate { entity });
        app.update();
        let after = app.world().resource::<Table>().session.clone();
        assert_eq!(after.observe().revision, before.revision + 1);
        assert_eq!(continuity::load_session(&path).unwrap(), after);
        assert_eq!(
            app.world().resource::<Orbit>().pose().target,
            layout::focus(Area::Test).0
        );
        assert_eq!(
            serde_json::to_value(&after).unwrap()["decisions"]
                .as_array()
                .unwrap()
                .last()
                .unwrap(),
            "test:reveal"
        );
        click(&mut app, "Menu");
        click(&mut app, "Reopen saved attempt");
        assert_eq!(app.world().resource::<Table>().session, after);
        assert_eq!(continuity::load_session(path).unwrap(), after);
    }

    #[test]
    fn rendered_reveal_failed_save_and_a_queued_modal_preserve_session_and_camera() {
        let directory = tempfile::tempdir().unwrap();
        let session = ordinary_reveal();
        let label = session
            .observe()
            .choices
            .iter()
            .find(|choice| choice.id == "test:reveal")
            .unwrap()
            .label
            .clone();
        let mut app = host(session.clone(), directory.path().to_owned());
        let camera = app.world().resource::<Orbit>().pose();
        click(&mut app, "Reveal tokens");
        click(&mut app, &label);
        assert_eq!(app.world().resource::<Table>().session, session);
        assert_eq!(app.world().resource::<Orbit>().pose(), camera);
        assert!(
            app.world()
                .resource::<Table>()
                .notice
                .contains("not committed")
        );
        click(&mut app, "Reveal tokens");
        let entity = button(&mut app, &label).unwrap();
        app.world_mut().trigger(Activate { entity });
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::History);
        app.update();
        assert_eq!(app.world().resource::<Table>().session, session);
        assert_eq!(app.world().resource::<Orbit>().pose(), camera);
        assert!(button(&mut app, "Reveal tokens").is_none());
    }

    #[test]
    fn declared_reveal_keeps_every_current_replacement_id_and_bag_label() {
        let session = declared_reveal();
        let view = session.observe();
        let offers = offered(&view);
        assert_eq!(
            offers,
            controls::source_choices(&view, &controls::Source::Bag)
        );
        assert!(offers.iter().any(|choice| choice.id == "test:reveal"));
        assert!(
            offers
                .iter()
                .any(|choice| choice.id.starts_with("tokens:jacqueline"))
        );
        assert!(
            offers
                .iter()
                .any(|choice| choice.id.starts_with("tokens:olive:"))
        );
        for choice in offers {
            assert!(
                view.choices.iter().any(|offered| offered.id == choice.id
                    && offered.cancel_faces == choice.cancel_faces),
                "Display labels must retain the authority's entire choice identity"
            );
        }
    }

    #[test]
    fn a_pre_reveal_choice_without_an_offered_reveal_never_enables_the_shortcut() {
        let view = Session::with_seed(89).observe();
        assert!(offered(&view).is_empty());
        let mut view = declared_reveal().observe();
        view.choices.retain(|choice| choice.id != "test:reveal");
        assert!(offered(&view).is_empty());
    }
}
