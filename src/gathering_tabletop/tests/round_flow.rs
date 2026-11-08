use super::*;
use crate::gathering::Phase;

fn round_review_points() -> Vec<(&'static str, Session, &'static str)> {
    let mut session = Session::with_seed(89);
    for id in [
        "lead:tony",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
        "turn:tony",
        "turn:end",
        "turn:jacqueline",
        "turn:end",
        "turn:patrice",
    ] {
        session.choose(session.observe().revision, id).unwrap();
    }
    let last_turn = session.clone();
    session
        .choose(session.observe().revision, "turn:end")
        .unwrap();
    let mut last_manual = None;
    for _ in 0..40 {
        let view = session.observe();
        if view.choices.iter().any(|choice| choice.id == "round:end") {
            let late_upkeep: Session = last_manual.expect("A real Upkeep choice precedes Mythos");
            assert_eq!(late_upkeep.observe().play.unwrap().phase, Phase::Upkeep);
            return vec![
                ("last-turn", last_turn, "patrice"),
                ("late-upkeep", late_upkeep, "scenario"),
            ];
        }
        let id = if let Some(id) = controls::automatic_choice(&view) {
            id
        } else {
            last_manual = Some(session.clone());
            &view
                .choices
                .iter()
                .find(|choice| {
                    choice.id.starts_with("upkeep:patrice-discard:")
                        || choice.id == "phase-window:pass"
                })
                .expect("Only the ordinary round's explicit choices")
                .id
        };
        session.choose(view.revision, id).unwrap();
    }
    panic!("The ordinary round must reach its end within the bounded route");
}

#[test]
fn round_review_saves_open_at_manual_choices_before_the_visible_transitions() {
    for (_, session, initial_view) in round_review_points() {
        let (_directory, mut app) =
            review_saves::open_review_save_in_view(&session, Some(initial_view)).unwrap();
        assert_eq!(app.world().resource::<Table>().session, session);
        assert!(!app.world().resource::<Table>().is_presenting());
        assert!(
            app.world()
                .resource::<Table>()
                .presentation
                .story()
                .is_none()
        );
        let view = session.observe();
        let id = view
            .choices
            .iter()
            .find(|choice| {
                choice.id == "turn:end"
                    || choice.id == "phase-window:pass"
                    || choice.id.starts_with("upkeep:patrice-discard:")
            })
            .unwrap()
            .id
            .clone();
        activate_choice(&mut app, &id);
        assert!(app.world().resource::<Table>().session.observe().revision > view.revision);
    }
}

#[test]
#[ignore = "exports ordinary native round checkpoints to ARKHAM_ROUND_REVIEW_DIR"]
fn export_round_review_saves() {
    let output = PathBuf::from(
        std::env::var_os("ARKHAM_ROUND_REVIEW_DIR").expect("Set a fresh output directory"),
    );
    let points = round_review_points();
    let launches: Vec<_> = points
        .iter()
        .map(|(name, session, view)| (*name, session, *view))
        .collect();
    review_saves::export_review_saves(&output, &launches);
}

#[test]
fn failed_compulsory_step_preserves_state_and_waits_for_explicit_retry() {
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringDoomThreshold);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("blocked.json");
    std::fs::create_dir(&path).unwrap();
    let mut app = host_with_table(Table::new(session.clone(), path.clone()), false);
    app.update();
    assert_eq!(app.world().resource::<Table>().session, session);
    assert!(
        app.world()
            .resource::<Table>()
            .notice
            .starts_with("Decision not committed:")
    );
    std::fs::remove_dir(&path).unwrap();
    for _ in 0..10 {
        draws::tick(&mut app, 0.1);
    }
    assert_eq!(app.world().resource::<Table>().session, session);
    assert!(
        !path.exists(),
        "Recovery does not silently retry the failed choice"
    );
    let retry = choice_button(&mut app, "Retry step").expect("An explicit recovery control");
    app.world_mut().trigger(Activate { entity: retry });
    app.update();
    let saved = continuity::load_session(&path).unwrap();
    assert_eq!(saved.observe().revision, session.observe().revision + 1);
    assert_eq!(saved, app.world().resource::<Table>().session);
}

#[test]
fn opening_investigation_is_announced_and_turn_selection_names_the_remaining_investigators() {
    let mut session = Session::with_seed(89);
    for id in ["lead:tony", "confirm-mulligan", "confirm-mulligan"] {
        session.choose(session.observe().revision, id).unwrap();
    }
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session, directory.path().join("opening.json")),
        false,
    );
    activate_choice(&mut app, "confirm-mulligan");
    let mut seen = false;
    for _ in 0..300 {
        draws::tick(&mut app, 0.1);
        if let Some(phase) = phase_title(&mut app) {
            assert_eq!(phase, "Investigation");
            seen = true;
        }
        if seen && !app.world().resource::<Table>().is_presenting() {
            break;
        }
    }
    assert!(seen);
    let session = app.world().resource::<Table>().session.clone();
    assert_eq!(session.observe().play.as_ref().unwrap().round, 1);
    assert_eq!(session.observe().play.as_ref().unwrap().agenda_doom, 0);
    let mut reopened = host_with_table(
        Table::new(session, directory.path().join("selection.json")),
        false,
    );
    let text = rendered_text(&mut reopened);
    assert!(
        text.contains("Choose the next investigator") && text.contains("Tony, Patrice, Jacqueline"),
        "{text}"
    );
    assert!(!text.contains("Setup · no investigator"));
}

fn phase_title(app: &mut App) -> Option<String> {
    let world = app.world_mut();
    world.query::<&Text>().iter(world).find_map(|text| {
        matches!(
            text.0.as_str(),
            "Investigation" | "Enemy" | "Upkeep" | "Mythos"
        )
        .then(|| text.0.clone())
    })
}

#[test]
fn entered_phases_are_announced_before_processing_and_block_background_choices() {
    for reduced in [false, true] {
        let mut session = Session::with_seed(89);
        for id in [
            "lead:tony",
            "confirm-mulligan",
            "confirm-mulligan",
            "confirm-mulligan",
            "turn:tony",
            "turn:end",
            "turn:jacqueline",
            "turn:end",
            "turn:patrice",
        ] {
            session.choose(session.observe().revision, id).unwrap();
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("announcements.json");
        let mut table = Table::new(session, path.clone());
        table.reduced_motion = reduced;
        let mut app = host_with_table(table, false);
        activate_choice(&mut app, "turn:end");
        assert_eq!(phase_title(&mut app).as_deref(), Some("Enemy"));
        let committed = app.world().resource::<Table>().session.clone();
        let epoch = app.world().resource::<Table>().epoch;
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::Choose(
                epoch,
                committed.observe().revision,
                "enemy:skip-empty".into(),
            ));
        draws::tick(&mut app, 1.0);
        assert_eq!(phase_title(&mut app).as_deref(), Some("Enemy"));
        assert_eq!(app.world().resource::<Table>().session, committed);
        assert_eq!(continuity::load_session(&path).unwrap(), committed);

        let mut seen = vec!["Enemy".to_owned()];
        for _ in 0..160 {
            draws::tick(&mut app, 0.1);
            if let Some(title) = phase_title(&mut app)
                && seen.last() != Some(&title)
            {
                seen.push(title);
            }
            let view = app.world().resource::<Table>().session.observe();
            if !app.world().resource::<Table>().is_presenting()
                && view
                    .choices
                    .iter()
                    .any(|choice| choice.id == "phase-window:pass")
            {
                // This seed has a real fast-card opportunity in the Enemy
                // player window. Its explicit pass must remain a human choice.
                assert!(
                    view.choices
                        .iter()
                        .filter(|choice| choice.id != "undo")
                        .count()
                        > 1
                );
                activate_choice(&mut app, "phase-window:pass");
            }
            if view
                .choices
                .iter()
                .any(|choice| choice.id.starts_with("upkeep:patrice-discard:"))
            {
                break;
            }
        }
        let table = app.world().resource::<Table>();
        assert_eq!(
            seen,
            ["Enemy", "Upkeep"],
            "reduced={reduced}; live={:?}; rendered={:?}; announcement={:?}; notice={}",
            table.session.observe().play.as_ref().map(|play| play.phase),
            table.view().play.as_ref().map(|play| play.phase),
            table
                .presentation
                .announcement()
                .map(|announcement| (announcement.phase, announcement.elapsed)),
            table.notice
        );
        let session = app.world().resource::<Table>().session.clone();
        assert_eq!(session.observe().play.unwrap().phase, Phase::Upkeep);
        drop(app);
        let mut reopened = host_with_table(Table::open_existing(path).unwrap(), false);
        assert_eq!(
            phase_title(&mut reopened),
            None,
            "Reopen does not replay an entered phase"
        );
        assert_eq!(reopened.world().resource::<Table>().session, session);
    }
}

#[test]
fn reopening_the_last_patrice_discard_finishes_ordering_and_refills_only_once() {
    let (_, mut session, _) = round_review_points().pop().unwrap();
    // Recreate a pre-fix save with the final card still waiting, using ordinary
    // driver choices rather than privileged state construction.
    for _ in 0..5 {
        let view = session.observe();
        let choices = view
            .choices
            .iter()
            .filter(|choice| choice.id.starts_with("upkeep:patrice-discard:"))
            .collect::<Vec<_>>();
        assert!(!choices.is_empty());
        if choices.len() == 1 {
            break;
        }
        session.choose(view.revision, &choices[0].id).unwrap();
    }
    let before = session.observe();
    assert_eq!(
        before
            .choices
            .iter()
            .filter(|choice| choice.id.starts_with("upkeep:patrice-discard:"))
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["upkeep:patrice-discard:111"]
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("last-discard.json");
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(Table::open_existing(path.clone()).unwrap(), false);
    settle_automatic_resolution(&mut app);
    let after = app.world().resource::<Table>().session.clone();
    let view = after.observe();
    assert!(
        view.choices
            .iter()
            .all(|choice| !choice.id.starts_with("upkeep:patrice-discard:"))
    );
    let patrice = &view.play.as_ref().unwrap().players[1];
    assert_eq!(
        patrice.deck_count,
        before.play.as_ref().unwrap().players[1].deck_count - 5
    );
    assert_eq!(
        patrice
            .discard
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        [107, 135, 119, 121, 111]
    );
    assert_eq!(continuity::load_session(&path).unwrap(), after);
    drop(app);
    let mut reopened = host_with_table(Table::open_existing(path).unwrap(), false);
    settle_automatic_resolution(&mut reopened);
    assert_eq!(reopened.world().resource::<Table>().session, after);
}

#[test]
fn automatic_round_preserves_discard_order_and_reopens_into_a_real_mythos_choice() {
    let mut session = Session::with_seed(89);
    for id in [
        "lead:tony",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
        "turn:tony",
        "turn:end",
        "turn:jacqueline",
        "turn:end",
        "turn:patrice",
    ] {
        session.choose(session.observe().revision, id).unwrap();
    }
    let before = session.observe().play.unwrap();
    assert_eq!(before.phase, Phase::Investigation);
    assert_eq!(before.round, 1);
    assert_eq!(before.agenda_doom, 0);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("round.json");
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(Table::new(session, path.clone()), false);
    activate_choice(&mut app, "turn:end");

    let mut discarded = Vec::new();
    let mut reopened = false;
    for _ in 0..32 {
        settle_automatic_resolution(&mut app);
        let view = app.world().resource::<Table>().session.observe();
        if view.play.as_ref().unwrap().phase == Phase::Mythos
            && view.play.as_ref().unwrap().encounter_deck_count < 26
        {
            break;
        }
        let choice = view
            .choices
            .iter()
            .find(|choice| choice.id.starts_with("upkeep:patrice-discard:"))
            .or_else(|| {
                view.choices.iter().find(|choice| {
                    choice.id == "phase-window:pass"
                        || choice.id == "search:decline-hunt"
                        || (choice.id.starts_with("bounty:") && choice.id.ends_with(":0"))
                })
            })
            .unwrap_or_else(|| {
                panic!(
                    "Bookkeeping must advance to a real choice: {:?}",
                    view.choices
                )
            });
        if let Some(id) = choice.id.strip_prefix("upkeep:patrice-discard:") {
            assert!(
                view.choices
                    .iter()
                    .filter(|choice| choice.id.starts_with("upkeep:patrice-discard:"))
                    .count()
                    > 1,
                "The last remaining card has no discard-order decision: {}",
                choice.label
            );
            discarded.push(id.parse::<u16>().unwrap());
        }
        activate_choice(&mut app, &choice.id);
        if !reopened && !discarded.is_empty() {
            let saved = continuity::load_session(&path).unwrap();
            assert_eq!(saved, app.world().resource::<Table>().session);
            drop(app);
            app = host_with_table(Table::open_existing(path.clone()).unwrap(), false);
            assert_eq!(saved, app.world().resource::<Table>().session);
            reopened = true;
        }
    }
    let session = &app.world().resource::<Table>().session;
    let view = session.observe();
    let play = view.play.as_ref().unwrap();
    assert!(reopened);
    assert_eq!(play.phase, Phase::Mythos);
    assert_eq!(play.round, 2);
    // This seeded round draws Ancient Evils before the next encounter choice:
    // one framework doom plus its printed additional doom (original 01092).
    assert!(
        view.history
            .iter()
            .any(|entry| entry.contains("Ancient Evils"))
    );
    assert_eq!(play.agenda_doom, 2);
    assert!(
        play.encounter_deck_count < 26,
        "The encounter draw is automatic"
    );
    assert_eq!(play.players[0].deck_count, before.players[0].deck_count - 1);
    assert_eq!(play.players[1].deck_count, before.players[1].deck_count - 5);
    assert_eq!(play.players[2].deck_count, before.players[2].deck_count - 1);
    assert!(play.players.iter().all(|player| player.resources == 6));
    assert_eq!(play.actions_remaining, [3; 3]);
    assert_eq!(discarded, [107, 135, 119, 121]);
    assert_eq!(
        &play.players[1]
            .discard
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>()[..5],
        [107, 135, 119, 121, 111],
        "Rise to the Occasion is placed last without another ordering click"
    );
    assert!(view.choices.iter().any(|choice| choice.id != "undo"));
    assert_eq!(&continuity::load_session(&path).unwrap(), session);
}
