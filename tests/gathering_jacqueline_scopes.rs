use arkham_unfolded_experiment::{
    TokenFace,
    gathering::{HostileFixture, Observation, Session, TokenFixture},
};

// Jacqueline's reaction adds two draws to the reveal she modifies, then cancels
// within it (60401). For a reveal with one position, her whole-reveal scope and
// her first-draw scope modify the same single draw. The proof compares every
// legal continuation of both scopes through the session: later declarations
// (Olive McBride, Dark Prophecy, Katarina Sojka) map draw P of the whole reveal
// to draw 0.P of the first-draw scope, and token choices name physical tokens.

fn choose(session: &mut Session, id: &str) {
    let view = session.observe();
    assert!(
        view.choices.iter().any(|choice| choice.id == id),
        "Missing {id}: {:?}",
        view.choices
    );
    session.choose(view.revision, id).unwrap();
}

fn asset(session: &Session, code: &str) -> u16 {
    session.observe().play.unwrap().players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == code)
        .unwrap()
        .card
        .id
}

/// Passes the empty windows before a reveal and, for Eyes of the Dreamer,
/// spends no charges, so the reveal has exactly one position.
fn reach_declaration(session: &mut Session) {
    for _ in 0..8 {
        let choices = session.observe().choices;
        if choices
            .iter()
            .any(|choice| choice.id.starts_with("tokens:jacqueline"))
        {
            return;
        }
        let next = ["tokens:eyes:0", "commit:confirm", "window:pass"]
            .into_iter()
            .find(|id| choices.iter().any(|choice| choice.id == *id))
            .unwrap_or_else(|| panic!("No route to the reveal: {choices:?}"));
        choose(session, next);
    }
    panic!("Jacqueline was never offered");
}

/// The identity a whole-reveal declaration has under the first-draw scope.
fn nested(id: &str) -> String {
    ["tokens:olive:", "tokens:prophecy:", "tokens:katarina:"]
        .into_iter()
        .find_map(|prefix| {
            id.strip_prefix(prefix)
                .map(|rest| format!("{prefix}0.{rest}"))
        })
        .unwrap_or_else(|| id.to_owned())
}

fn ids(view: &Observation, map: fn(&str) -> String) -> Vec<String> {
    view.choices
        .iter()
        .filter(|choice| choice.id != "undo")
        .map(|choice| map(&choice.id))
        .collect()
}

/// Everything a player can observe about the consequences, excluding wording
/// that names the scope's position (labels and history).
fn assert_same_consequences(whole: &Session, first: &Session, trail: &[String]) {
    let (a, b) = (whole.observe(), first.observe());
    assert_eq!(ids(&a, nested), ids(&b, str::to_owned), "after {trail:?}");
    let (a, b) = (a.play.unwrap(), b.play.unwrap());
    assert_eq!(a.players, b.players, "after {trail:?}");
    assert_eq!(a.test, b.test, "after {trail:?}");
    assert_eq!(a.completed_tests, b.completed_tests, "after {trail:?}");
    assert_eq!(a.bag_count, b.bag_count, "after {trail:?}");
    assert_eq!(a.curse_pool, b.curse_pool, "after {trail:?}");
    assert_eq!(a.premonition, b.premonition, "after {trail:?}");
    assert_eq!(a.actions_remaining, b.actions_remaining, "after {trail:?}");
    match (a.tokens, b.tokens) {
        (Some(a), Some(b)) => {
            assert_eq!(a.stage, b.stage, "after {trail:?}");
            assert_eq!(a.revealed, b.revealed, "after {trail:?}");
            assert_eq!(a.selected, b.selected, "after {trail:?}");
            assert_eq!(a.ignored, b.ignored, "after {trail:?}");
            assert_eq!(a.cancelled, b.cancelled, "after {trail:?}");
            assert_eq!(a.fresh_reveal_count(), b.fresh_reveal_count());
        }
        (a, b) => assert_eq!(a.is_none(), b.is_none(), "after {trail:?}"),
    }
}

/// Selecting a token or group toggles it; the proof explores each selection
/// set once (ascending physical order) and never unselects.
fn explores(view: &Observation, id: &str) -> bool {
    if id == "undo" {
        return false;
    }
    let Some(tokens) = view.play.as_ref().and_then(|play| play.tokens.as_ref()) else {
        return true;
    };
    let members = tokens.choice_members(id);
    if members.is_empty() || id.starts_with("tokens:effect:") {
        return true;
    }
    let highest = tokens.selected.iter().max();
    members
        .iter()
        .all(|index| !tokens.selected.contains(index) && highest.is_none_or(|top| index > top))
}

/// Explores every continuation until no reveal decision remains.
/// Returns the number of complete continuations compared.
fn explore(whole: &Session, first: &Session, trail: &mut Vec<String>) -> usize {
    assert_same_consequences(whole, first, trail);
    assert!(trail.len() < 40, "Unbounded continuation: {trail:?}");
    let view = whole.observe();
    let finished = view.play.as_ref().is_some_and(|play| play.tokens.is_none())
        || !view
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("tokens:") || choice.id == "test:reveal");
    if finished {
        return 1;
    }
    let mut leaves = 0;
    for choice in &view.choices {
        if !explores(&view, &choice.id) {
            continue;
        }
        let (mut a, mut b) = (whole.clone(), first.clone());
        choose(&mut a, &choice.id);
        choose(&mut b, &nested(&choice.id));
        trail.push(choice.id.clone());
        leaves += explore(&a, &b, trail);
        trail.pop();
    }
    leaves
}

fn prove(fixture: HostileFixture, arbiter: bool) -> usize {
    let mut session = Session::with_hostile_fixture(fixture);
    reach_declaration(&mut session);
    let cost = if arbiter {
        format!(":arbiter:{}", asset(&session, "60402"))
    } else {
        String::new()
    };
    let (mut whole, mut first) = (session.clone(), session);
    choose(&mut whole, &format!("tokens:jacqueline{cost}"));
    choose(&mut first, &format!("tokens:jacqueline:at:0{cost}"));
    explore(&whole, &first, &mut vec![])
}

#[test]
fn single_draw_jacqueline_scopes_have_identical_consequences_under_every_later_declaration() {
    let mut compared = 0;
    for (fixture, arbiter) in [
        (HostileFixture::JacquelineCancellation, false),
        (HostileFixture::JacquelineProtectedTokens, false),
        (
            HostileFixture::Tokens(TokenFixture::ArbiterCancellation),
            true,
        ),
        (HostileFixture::Tokens(TokenFixture::JacquelineOlive), false),
        (
            HostileFixture::Tokens(TokenFixture::JacquelineOliveProtected),
            false,
        ),
        (HostileFixture::Tokens(TokenFixture::ProphecyOlive), false),
        (
            HostileFixture::Tokens(TokenFixture::ProphecyOliveOtherSymbol),
            false,
        ),
        (HostileFixture::Tokens(TokenFixture::DarkProphecy), false),
        (
            HostileFixture::Tokens(TokenFixture::JacquelineKatarina),
            false,
        ),
        (HostileFixture::Tokens(TokenFixture::Premonition), false),
    ] {
        compared += prove(fixture, arbiter);
    }
    assert!(compared > 100, "Only {compared} continuations compared");
}

/// Eyes of the Dreamer chooses one candidate per position. Under the first-draw
/// scope Jacqueline's group replaces Eyes' only draw and is one candidate, so
/// every survivor of her cancellation resolves; under the whole-reveal scope her
/// two tokens are separate candidates and Eyes resolves one of them.
#[test]
fn eyes_without_charges_keeps_both_scopes_because_jacquelines_group_is_one_eyes_candidate() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::EyesKatarina));
    reach_declaration(&mut session);
    let katarina = asset(&session, "11069");
    let (mut whole, mut first) = (session.clone(), session);
    for (session, scope, position) in [
        (&mut whole, "tokens:jacqueline", "0"),
        (&mut first, "tokens:jacqueline:at:0", "0.0"),
    ] {
        choose(session, scope);
        choose(session, &format!("tokens:katarina:{position}:{katarina}"));
        choose(session, "test:reveal");
        // Katarina ignores -1 and -2 and stops at skull; Jacqueline adds elder
        // sign and one more token. Dark Future protects both symbols, so only
        // the last token can be cancelled.
        let tokens = session.observe().play.unwrap().tokens.unwrap();
        assert_eq!(
            tokens.revealed[..4],
            [
                TokenFace::MinusOne,
                TokenFace::MinusTwo,
                TokenFace::Skull,
                TokenFace::ElderSign
            ]
        );
        choose(session, "tokens:cancel:4");
        choose(session, "tokens:confirm");
    }
    assert_eq!(
        ids(&whole.observe(), str::to_owned),
        ["tokens:keep:2", "tokens:keep:3"]
    );
    assert_eq!(ids(&first.observe(), str::to_owned), ["tokens:position:0"]);
}
