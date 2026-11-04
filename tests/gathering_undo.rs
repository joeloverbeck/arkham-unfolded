use arkham_unfolded_experiment::gathering::{BackpackFixture, HostileFixture, Session, continuity};

fn choose(session: &mut Session, id: &str) {
    let before = session.observe();
    assert!(
        before.choices.iter().any(|choice| choice.id == id),
        "Missing {id}: {:?}",
        before.choices
    );
    session.choose(before.revision, id).unwrap();
}
fn can_undo(session: &Session) -> bool {
    session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "undo")
}

#[test]
fn undo_restores_known_action_costs_but_keeps_a_monotonic_auditable_record() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Backpack(BackpackFixture::Search));
    let original = session.observe();
    choose(&mut session, "action:ordinary:resource");
    assert!(
        can_undo(&session),
        "A known resource action reveals no hidden information"
    );
    let paid = session.observe();
    assert_eq!(paid.play.as_ref().unwrap().players[0].resources, 11);
    choose(&mut session, "undo");
    let restored = session.observe();
    assert_eq!(restored.revision, original.revision + 2);
    assert_eq!(restored.play, original.play);
    assert!(restored.history.starts_with(&paid.history));
    assert!(restored.history.last().unwrap().contains("Undo"));
    assert!(!can_undo(&session));
    assert!(
        session
            .choose(paid.revision, "action:ordinary:resource")
            .is_err()
    );
    session.validate().unwrap();
}

#[test]
fn mulligan_selection_can_undo_but_opening_and_replacement_information_cannot() {
    let mut session = Session::with_seed(89);
    choose(&mut session, "lead:tony");
    assert!(!can_undo(&session));
    let first = session.observe().investigators[0].hand[0].id;
    choose(&mut session, &format!("mulligan:{first}"));
    assert!(can_undo(&session));
    choose(&mut session, "undo");
    assert!(
        session.observe().investigators[0]
            .selected_mulligan
            .is_empty()
    );
    assert!(!can_undo(&session));
    choose(&mut session, &format!("mulligan:{first}"));
    choose(&mut session, "confirm-mulligan");
    assert!(!can_undo(&session));
    let after = session.clone();
    assert!(session.choose(session.observe().revision, "undo").is_err());
    assert_eq!(session, after);
}

#[test]
fn a_draw_blocks_undo_even_when_the_deck_was_already_shuffled_and_rng_did_not_change() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Backpack(BackpackFixture::Search));
    choose(&mut session, "action:ordinary:resource");
    let random = serde_json::to_value(&session).unwrap()["runtime"]["random"].clone();
    let before = session.observe().play.unwrap();
    choose(&mut session, "action:ordinary:draw");
    let after = session.observe().play.unwrap();
    assert_eq!(
        serde_json::to_value(&session).unwrap()["runtime"]["random"],
        random
    );
    assert_eq!(
        after.players[0].deck_count,
        before.players[0].deck_count - 1
    );
    assert_eq!(
        after.players[0].hand.len(),
        before.players[0].hand.len() + 1
    );
    assert!(
        !can_undo(&session),
        "Neither the draw nor the preceding resource choice can be rewound after seeing the card"
    );
}

#[test]
fn search_exposure_and_search_shuffle_each_seal_earlier_choices() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Backpack(BackpackFixture::Search));
    let pack = session.observe().play.unwrap().players[0]
        .hand
        .iter()
        .find(|card| card.code == "04037")
        .unwrap()
        .id;
    choose(&mut session, &format!("action:ordinary:play:{pack}"));
    assert!(
        can_undo(&session),
        "Paid entry is reversible before the deck is searched"
    );
    let deck_count = session.observe().play.unwrap().players[0].deck_count;
    let random = serde_json::to_value(&session).unwrap()["runtime"]["random"].clone();
    choose(&mut session, "backpack:search");
    assert_eq!(
        session.observe().play.as_ref().unwrap().players[0].deck_count,
        deck_count
    );
    assert_eq!(
        serde_json::to_value(&session).unwrap()["runtime"]["random"],
        random
    );
    assert!(
        !can_undo(&session),
        "Looking reveals information without changing the deck or RNG"
    );
    let card = session.observe().play.unwrap().searched_player[0].id;
    choose(&mut session, &format!("backpack:select:{card}"));
    assert!(can_undo(&session));
    choose(&mut session, "undo");
    assert!(!can_undo(&session));
    assert!(
        session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == format!("backpack:select:{card}")
                && choice.label.starts_with("Select"))
    );
    choose(&mut session, "backpack:attach");
    assert!(
        !can_undo(&session),
        "The required shuffle is a conservative random-outcome barrier even with zero attachments"
    );
}

fn reach_tokens(session: &mut Session) {
    for _ in 0..5 {
        if session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "test:reveal")
        {
            return;
        }
        let id = if session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "commit:confirm")
        {
            "commit:confirm"
        } else {
            "window:pass"
        };
        choose(session, id);
    }
    panic!("Expected actual token declaration window");
}

#[test]
fn fixture_token_reveal_is_a_barrier_and_known_selection_undo_cannot_cross_it() {
    let mut session = Session::with_hostile_fixture(HostileFixture::JacquelineCancellation);
    reach_tokens(&mut session);
    choose(&mut session, "tokens:jacqueline");
    assert!(can_undo(&session));
    let random = serde_json::to_value(&session).unwrap()["runtime"]["random"].clone();
    choose(&mut session, "test:reveal");
    assert_eq!(
        serde_json::to_value(&session).unwrap()["runtime"]["random"],
        random,
        "The fixture does not draw using ordinary RNG"
    );
    assert!(!can_undo(&session));
    let revealed = session.observe().play.unwrap().tokens.unwrap().revealed;
    assert_eq!(revealed.len(), 3);
    choose(&mut session, "tokens:cancel:0");
    assert!(can_undo(&session));
    choose(&mut session, "undo");
    assert_eq!(
        session.observe().play.unwrap().tokens.unwrap().revealed,
        revealed
    );
    assert!(!can_undo(&session));
    let preserved = session.clone();
    assert!(session.choose(session.observe().revision, "undo").is_err());
    assert_eq!(session, preserved);
}

#[test]
fn undo_commits_reopen_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("live.json");
    let unusable = dir.path().join("directory");
    std::fs::create_dir(&unusable).unwrap();
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Backpack(BackpackFixture::Search));
    continuity::save_session(&path, &session).unwrap();
    let revision = session.observe().revision;
    continuity::commit_choice(&path, &mut session, revision, "action:ordinary:resource").unwrap();
    let paid = session.clone();
    let bytes = std::fs::read(&path).unwrap();
    let revision = session.observe().revision;
    assert!(continuity::commit_choice(&unusable, &mut session, revision, "undo").is_err());
    assert_eq!(session, paid);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    continuity::commit_choice(&path, &mut session, revision, "undo").unwrap();
    assert_eq!(continuity::load_session(&path).unwrap(), session);
    assert_eq!(
        continuity::load_session(continuity::previous_path(&path)).unwrap(),
        paid
    );
    assert!(
        continuity::save_session(&path, &paid).is_err(),
        "Undo remains a later audited continuation, not permission to overwrite history"
    );
}

#[test]
fn repeated_safe_undo_cycles_use_normalized_actions_and_keep_all_evidence() {
    let started = std::time::Instant::now();
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Backpack(BackpackFixture::Search));
    let initial = session.observe().play;
    for _ in 0..64 {
        choose(&mut session, "action:ordinary:resource");
        choose(&mut session, "undo");
    }
    assert_eq!(session.observe().revision, 128);
    assert_eq!(session.observe().play, initial);
    session.validate().unwrap();
    eprintln!(
        "64 safe Undo cycles plus Session validation: {:?}",
        started.elapsed()
    );
}

#[test]
fn repeated_undo_and_a_rechosen_action_restore_the_correct_live_prefix() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Backpack(BackpackFixture::Search));
    let initial = session.observe().play;
    choose(&mut session, "action:ordinary:resource");
    let first = session.observe().play;
    choose(&mut session, "action:ordinary:resource");
    choose(&mut session, "undo");
    assert_eq!(session.observe().play, first);
    choose(&mut session, "action:ordinary:resource");
    choose(&mut session, "undo");
    assert_eq!(session.observe().play, first);
    choose(&mut session, "undo");
    assert_eq!(session.observe().play, initial);
    assert!(!can_undo(&session));
    session.validate().unwrap();
}

#[test]
fn entering_an_unrevealed_room_blocks_undo_at_the_actual_reveal() {
    use arkham_unfolded_experiment::gathering::TonyEffectFixture;
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::GregoryCigarette));
    assert!(
        !session
            .observe()
            .play
            .unwrap()
            .scenario
            .locations
            .iter()
            .find(|location| location.code == "01113")
            .unwrap()
            .revealed
    );
    choose(&mut session, "action:ordinary:move:01113");
    for _ in 0..12 {
        let view = session.observe();
        if view
            .play
            .as_ref()
            .unwrap()
            .scenario
            .locations
            .iter()
            .find(|location| location.code == "01113")
            .unwrap()
            .revealed
        {
            assert!(
                !can_undo(&session),
                "New room information seals the move, including its paid opportunity sequence"
            );
            return;
        }
        let id = view
            .choices
            .iter()
            .find(|choice| choice.id.starts_with("opportunity:"))
            .map(|choice| choice.id.clone())
            .or_else(|| {
                [
                    "harm:damage:investigator",
                    "harm:horror:investigator",
                    "harm:confirm",
                ]
                .into_iter()
                .find(|id| view.choices.iter().any(|choice| choice.id == *id))
                .map(str::to_string)
            })
            .expect("Actual opportunity or harm continuation before entry");
        choose(&mut session, &id);
    }
    panic!("The actual move did not reveal Attic");
}

#[test]
fn act_advancement_and_terminal_story_information_seal_earlier_choices() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    choose(&mut session, "round-end:advance-act");
    for who in 0..3 {
        for _ in 0..3 {
            choose(&mut session, &format!("act:contribute:{who}"));
        }
    }
    assert_eq!(session.observe().play.unwrap().act, 3);
    assert!(
        !can_undo(&session),
        "Act reverse and Parlor reveal cannot be undone"
    );
    choose(&mut session, "bounty:400:0");
    assert!(can_undo(&session));
    choose(&mut session, "undo");
    assert!(
        !can_undo(&session),
        "Undoing the subsequent known choice cannot cross the story boundary"
    );
    choose(&mut session, "bounty:400:0");
    choose(&mut session, "round-end:ghoul-doom");
    choose(&mut session, "mythos:begin");
    assert!(session.observe().play.unwrap().scenario.outcome.is_some());
    assert!(
        session
            .observe()
            .prompt
            .contains("recorded Gathering resolution")
    );
    assert!(!can_undo(&session));
}
