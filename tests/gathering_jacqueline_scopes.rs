use arkham_unfolded_experiment::{
    TokenFace,
    gathering::{HostileFixture, Observation, Session, TokenFixture},
};

// Jacqueline's reaction adds two draws to the reveal she modifies, then cancels
// within it (60401). For a reveal with one plain draw, her whole-reveal scope and
// her first-draw scope modify the same draw. Commit 48f1389 proved, by exploring
// every legal continuation of both scopes, that they have identical
// consequences there, including under later Olive McBride, Dark Prophecy and
// Katarina Sojka declarations; the session now offers that scope once. Where
// the scopes differ, both stay offered and each label states its effect.

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
/// spends the given number of charges.
fn reach_declaration(session: &mut Session, charges: u8) {
    let eyes = format!("tokens:eyes:{charges}");
    for _ in 0..8 {
        let choices = session.observe().choices;
        if choices
            .iter()
            .any(|choice| choice.id.starts_with("tokens:jacqueline"))
        {
            return;
        }
        let next = [eyes.as_str(), "commit:confirm", "window:pass"]
            .into_iter()
            .find(|id| choices.iter().any(|choice| choice.id == *id))
            .unwrap_or_else(|| panic!("No route to the reveal: {choices:?}"));
        choose(session, next);
    }
    panic!("Jacqueline was never offered");
}

fn ids(view: &Observation) -> Vec<String> {
    view.choices
        .iter()
        .filter(|choice| choice.id != "undo")
        .map(|choice| choice.id.clone())
        .collect()
}

fn jacqueline(session: &Session) -> Vec<(String, String)> {
    session
        .observe()
        .choices
        .into_iter()
        .filter(|choice| choice.id.starts_with("tokens:jacqueline"))
        .map(|choice| (choice.id, choice.label))
        .collect()
}

fn offered(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(id, label)| ((*id).to_owned(), (*label).to_owned()))
        .collect()
}

const ONCE: &str = "Use Jacqueline — reveal 3 tokens, then cancel";
const ONCE_ARBITER: &str =
    "Use Jacqueline + Arbiter — reveal 3 tokens, then cancel; exhaust Arbiter, preserve round use";

fn assert_first_draw_refused(session: &mut Session) {
    let before = session.clone();
    let revision = session.observe().revision;
    assert!(session.choose(revision, "tokens:jacqueline:at:0").is_err());
    assert_eq!(*session, before);
}

#[test]
fn a_single_draw_reveal_offers_jacqueline_once_whether_or_not_olive_is_ready() {
    for fixture in [
        // Tony tests; Jacqueline assists at his location.
        HostileFixture::JacquelineCancellation,
        HostileFixture::Tokens(TokenFixture::JacquelineOlive),
        HostileFixture::Tokens(TokenFixture::ProphecyOlive),
        HostileFixture::Tokens(TokenFixture::DarkProphecy),
        HostileFixture::Tokens(TokenFixture::JacquelineKatarina),
        // Premonition's own one-token reveal.
        HostileFixture::Tokens(TokenFixture::Premonition),
    ] {
        let mut session = Session::with_hostile_fixture(fixture);
        reach_declaration(&mut session, 0);
        assert_eq!(
            jacqueline(&session),
            offered(&[("tokens:jacqueline", ONCE)]),
            "{fixture:?}"
        );
        assert_first_draw_refused(&mut session);
    }
}

#[test]
fn arbiter_is_the_only_variant_beside_the_single_jacqueline_choice() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::ArbiterCancellation));
    reach_declaration(&mut session, 0);
    let arbiter = format!("tokens:jacqueline:arbiter:{}", asset(&session, "60402"));
    assert_eq!(
        jacqueline(&session),
        offered(&[("tokens:jacqueline", ONCE), (&arbiter, ONCE_ARBITER)])
    );
    assert_first_draw_refused(&mut session);
    let revision = session.observe().revision;
    assert!(
        session
            .choose(
                revision,
                &format!("tokens:jacqueline:at:0{}", &arbiter[17..])
            )
            .is_err()
    );
}

#[test]
fn a_curse_reveal_offers_its_new_single_draw_once() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::ArbiterCurse));
    reach_declaration(&mut session, 0);
    let arbiter = format!("tokens:jacqueline:arbiter:{}", asset(&session, "60402"));
    for id in [
        arbiter.as_str(),
        "test:reveal",
        "tokens:cancel:1",
        "tokens:cancel:2",
        "tokens:confirm",
    ] {
        choose(&mut session, id);
    }
    // Arbiter preserved the round use for the curse's additional reveal.
    assert_eq!(
        jacqueline(&session),
        offered(&[("tokens:jacqueline", ONCE)])
    );
    assert_first_draw_refused(&mut session);
}

#[test]
fn voice_of_ra_offers_the_whole_reveal_and_each_draw_by_effect() {
    let session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::VoiceRaGuidance));
    assert_eq!(
        jacqueline(&session),
        offered(&[
            (
                "tokens:jacqueline",
                "Use Jacqueline — affect all 3 draws: add 2 tokens, then cancel"
            ),
            (
                "tokens:jacqueline:at:0",
                "Use Jacqueline — affect only draw 1: add 2 tokens, then cancel"
            ),
            (
                "tokens:jacqueline:at:1",
                "Use Jacqueline — affect only draw 2: add 2 tokens, then cancel"
            ),
            (
                "tokens:jacqueline:at:2",
                "Use Jacqueline — affect only draw 3: add 2 tokens, then cancel"
            ),
        ])
    );
}

#[test]
fn dark_future_offers_the_whole_reveal_and_each_of_its_five_draws() {
    let session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::NestedProphecyCleanup));
    let mut expected = vec![(
        "tokens:jacqueline".to_owned(),
        "Use Jacqueline — affect all 5 draws: add 2 tokens, then cancel".to_owned(),
    )];
    expected.extend((0..5).map(|draw| {
        (
            format!("tokens:jacqueline:at:{draw}"),
            format!(
                "Use Jacqueline — affect only draw {}: add 2 tokens, then cancel",
                draw + 1
            ),
        )
    }));
    assert_eq!(jacqueline(&session), expected);
}

#[test]
fn olive_declared_first_offers_jacqueline_around_her_on_her_draw_or_on_one_of_her_draws() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::JacquelineOlive));
    reach_declaration(&mut session, 0);
    let olive = asset(&session, "04197");
    choose(&mut session, &format!("tokens:olive:0:{olive}"));
    assert_eq!(
        jacqueline(&session),
        offered(&[
            (
                "tokens:jacqueline",
                "Use Jacqueline around Olive — choose Olive's 2 first; then cancel whole results"
            ),
            (
                "tokens:jacqueline:at:0",
                "Use Jacqueline on Olive's whole draw — add 2 tokens to Olive's 3; cancel first, then choose 2"
            ),
            (
                "tokens:jacqueline:at:0.0",
                "Use Jacqueline on Olive's draw 1 — cancel within this draw first"
            ),
            (
                "tokens:jacqueline:at:0.1",
                "Use Jacqueline on Olive's draw 2 — cancel within this draw first"
            ),
            (
                "tokens:jacqueline:at:0.2",
                "Use Jacqueline on Olive's draw 3 — cancel within this draw first"
            ),
        ])
    );
}

#[test]
fn eyes_offers_its_whole_request_and_each_draw_by_effect() {
    for (charges, expected) in [
        (
            1,
            [
                (
                    "tokens:jacqueline",
                    "Use Jacqueline — affect both draws: add 2 tokens as separate Eyes candidates, then cancel",
                ),
                (
                    "tokens:jacqueline:at:0",
                    "Use Jacqueline — affect only draw 1: add 2 tokens, then cancel; her group is one Eyes candidate",
                ),
                (
                    "tokens:jacqueline:at:1",
                    "Use Jacqueline — affect only draw 2: add 2 tokens, then cancel; her group is one Eyes candidate",
                ),
            ]
            .as_slice(),
        ),
        (
            0,
            [
                (
                    "tokens:jacqueline",
                    "Use Jacqueline — affect the whole reveal: add 2 tokens as separate Eyes candidates, then cancel",
                ),
                (
                    "tokens:jacqueline:at:0",
                    "Use Jacqueline — affect only draw 1: add 2 tokens, then cancel; her group is one Eyes candidate",
                ),
            ]
            .as_slice(),
        ),
    ] {
        let mut session =
            Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::EyesKatarina));
        reach_declaration(&mut session, charges);
        assert_eq!(jacqueline(&session), offered(expected), "{charges} charges");
    }
}

/// Eyes of the Dreamer chooses one candidate per position. Under the first-draw
/// scope Jacqueline's group replaces Eyes' only draw and is one candidate, so
/// every survivor of her cancellation resolves; under the whole-reveal scope her
/// two tokens are separate candidates and Eyes resolves one of them.
#[test]
fn eyes_without_charges_keeps_both_scopes_because_jacquelines_group_is_one_eyes_candidate() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::EyesKatarina));
    reach_declaration(&mut session, 0);
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
    assert_eq!(ids(&whole.observe()), ["tokens:keep:2", "tokens:keep:3"]);
    assert_eq!(ids(&first.observe()), ["tokens:position:0"]);
}
