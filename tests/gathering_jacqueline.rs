use arkham_unfolded_experiment::gathering::{Observation, Session, continuity};

fn choose(session: &mut Session, id: &str) {
    session.choose(session.observe().revision, id).unwrap();
}

fn reveal(session: &mut Session) {
    for _ in 0..8 {
        let view = session.observe();
        if view.choices.iter().any(|choice| choice.id == "test:reveal") {
            return;
        }
        advance(session, &view);
    }
    panic!("The ordinary investigation did not reach its reveal");
}

// A deliberately modest player policy, not a rules oracle. Expected reaction
// availability comes from Jacqueline's printed once-per-round limit.
fn advance(session: &mut Session, view: &Observation) {
    let preferred = [
        "tokens:confirm",
        "phase-window:pass",
        "window:pass",
        "test-card:pass",
        "commit:confirm",
        "test:reveal",
        "lucky:decline",
        "success:pass",
        "after-test:pass",
        "harm:damage:investigator",
        "harm:horror:investigator",
        "harm:confirm",
        "turn:end",
        "enemy:attacks",
        "round:end",
        "round-end:decline-act",
        "mythos:begin",
        "revelation:resolve",
        "investigation:begin",
    ];
    let choice = preferred
        .into_iter()
        .find_map(|id| view.choices.iter().find(|choice| choice.id == id))
        .or_else(|| {
            view.choices.iter().find(|choice| {
                if let Some(index) = choice.id.strip_prefix("tokens:cancel:") {
                    return !view
                        .play
                        .as_ref()
                        .unwrap()
                        .tokens
                        .as_ref()
                        .unwrap()
                        .selected
                        .contains(&index.parse().unwrap());
                }
                choice.id.contains("decline")
                    || choice.id.ends_with(":pass")
                    || [
                        "turn:",
                        "upkeep:",
                        "enemy:",
                        "mythos:",
                        "result:",
                        "turn-end:",
                        "round-end:",
                        "revelation-order:",
                        "discard-card:",
                    ]
                    .iter()
                    .any(|prefix| choice.id.starts_with(prefix))
                    || (choice.id.starts_with("bounty:") && choice.id.ends_with(":0"))
            })
        })
        .unwrap_or_else(|| panic!("Unexpected player decision: {:?}", view.choices));
    choose(session, &choice.id);
}

#[test]
fn jacqueline_renews_at_the_next_round_after_real_play_and_reopen() {
    let mut session = Session::with_seed(89);
    choose(&mut session, "lead:tony");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    choose(&mut session, "turn:tony");
    choose(&mut session, "action:ordinary:investigate");
    reveal(&mut session);
    choose(&mut session, "tokens:jacqueline");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("used-reaction.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    for _ in 0..40 {
        let view = session.observe();
        if view.choices.iter().any(|choice| choice.id == "turn:end") {
            break;
        }
        advance(&mut session, &view);
    }
    choose(&mut session, "action:ordinary:investigate");
    reveal(&mut session);
    assert_eq!(session.observe().play.as_ref().unwrap().round, 1);
    assert!(
        !session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("tokens:jacqueline")),
        "The same round must still refuse a second ordinary use"
    );
    for _ in 0..200 {
        let view = session.observe();
        if view.play.as_ref().unwrap().round == 2
            && view.choices.iter().any(|choice| choice.id == "test:reveal")
        {
            continuity::save_session(&path, &session).unwrap();
            let reopened = continuity::load_session(&path).unwrap();
            assert_eq!(reopened.observe(), view);
            assert!(
                view.choices
                    .iter()
                    .any(|choice| choice.id == "tokens:jacqueline"),
                "Jacqueline must be available again at a new round's same-location reveal: {:?}",
                view.choices
            );
            return;
        }
        assert!(view.play.as_ref().unwrap().round <= 2);
        advance(&mut session, &view);
    }
    panic!("Ordinary play did not reach a second-round reveal");
}
