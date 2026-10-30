use arkham_unfolded_experiment::{Fixture, Session, Stage, TokenFace, TokenStatus};

fn choose(game: &mut Session, id: &str) {
    game.choose(game.observe().revision, id).unwrap();
}

fn cancel(game: &mut Session, faces: &[TokenFace]) {
    let choice = game
        .observe()
        .choices
        .into_iter()
        .find(|choice| choice.cancel_faces == faces)
        .unwrap();
    choose(game, &choice.id);
}

fn at_cancellation(fixture: Fixture) -> Session {
    let mut game = Session::new(fixture);
    choose(&mut game, "commit-promise");
    choose(&mut game, "use-jacqueline");
    game
}

#[test]
fn canceling_the_curse_keeps_it_in_the_bag_without_an_extra_draw() {
    let mut game = Session::new(Fixture::curse());
    game.choose(game.observe().revision, "commit-promise")
        .unwrap();
    game.choose(game.observe().revision, "use-jacqueline")
        .unwrap();
    let choice = game
        .observe()
        .choices
        .into_iter()
        .find(|choice| choice.cancel_faces == vec![TokenFace::Curse, TokenFace::MinusOne])
        .unwrap();
    game.choose(game.observe().revision, &choice.id).unwrap();
    let result = game.observe();
    assert_eq!(result.stage, Stage::Finished);
    assert_eq!(result.skill_value, 9);
    assert_eq!(result.horror_dealt, 0);
    assert_eq!(result.curses_in_bag, 1);
    assert_eq!(result.tokens.len(), 3);
}

// Expected values are worked from the original cards, RR ST.2–8, FAQ2.5
// §2.5/§2.9 and Innsmouth p1, not calculated from an implementation trace.
#[test]
fn resolving_the_curse_draws_again_and_returns_it_to_the_pool() {
    let mut game = at_cancellation(Fixture::curse());
    cancel(&mut game, &[TokenFace::MinusOne, TokenFace::Zero]);
    assert_eq!(game.observe().stage, Stage::Finished);
    assert!(
        !game
            .observe()
            .choices
            .iter()
            .any(|c| c.id == "use-jacqueline")
    );
    let result = game.observe();
    assert_eq!((result.skill_value, result.horror_dealt), (5, 0));
    assert_eq!((result.curses_in_bag, result.curses_in_pool), (0, 10));
    assert_eq!(result.tokens.len(), 4);
    assert_eq!(result.bag_count, 16);
}

#[test]
fn canceling_auto_fail_alone_resolves_both_other_tokens() {
    let mut game = at_cancellation(Fixture::auto_fail());
    assert_eq!(
        game.observe()
            .choices
            .iter()
            .filter(|c| !c.cancel_faces.is_empty())
            .count(),
        2
    );
    cancel(&mut game, &[TokenFace::AutoFail]);
    assert_eq!(game.observe().stage, Stage::Finished);
    let result = game.observe();
    assert_eq!((result.skill_value, result.horror_dealt), (5, 0));
    assert_eq!(
        result
            .tokens
            .iter()
            .filter(|t| t.status == TokenStatus::Canceled)
            .count(),
        1
    );
    assert_eq!(result.curses_in_bag, 0);
}

#[test]
fn a_legal_adverse_choice_deals_three_horror_in_addition_to_starting_trauma() {
    let mut game = at_cancellation(Fixture::auto_fail());
    cancel(&mut game, &[TokenFace::Curse, TokenFace::Zero]);
    let result = game.observe();
    assert_eq!((result.skill_value, result.horror_dealt), (0, 3));
    assert_eq!(result.investigators[2].horror, 5);
    assert_eq!(result.curses_in_bag, 1);
}

#[test]
fn all_three_non_autofail_cancellation_pairs_are_available() {
    let mut game = at_cancellation(Fixture::curse());
    assert_eq!(
        game.observe()
            .choices
            .iter()
            .filter(|c| !c.cancel_faces.is_empty())
            .count(),
        3
    );
    cancel(&mut game, &[TokenFace::Curse, TokenFace::Zero]);
    assert_eq!(game.observe().skill_value, 8);
}

#[test]
fn declining_initial_use_preserves_jacqueline_for_the_curse_continuation() {
    let mut game = Session::new(Fixture::curse());
    choose(&mut game, "commit-promise");
    choose(&mut game, "reveal");
    assert!(!game.observe().jacqueline_used);
    assert_eq!(game.observe().tokens[0].status, TokenStatus::Resolved);
    choose(&mut game, "use-jacqueline");
    cancel(&mut game, &[TokenFace::MinusOne, TokenFace::MinusTwo]);
    let result = game.observe();
    assert_eq!((result.skill_value, result.horror_dealt), (7, 0));
    assert_eq!(result.tokens[0].status, TokenStatus::Resolved);
    assert_eq!((result.curses_in_bag, result.curses_in_pool), (0, 10));
}

#[test]
fn declining_both_reactions_still_resolves_the_skill_test() {
    let mut game = Session::new(Fixture::curse());
    choose(&mut game, "commit-promise");
    choose(&mut game, "reveal");
    choose(&mut game, "reveal");
    assert_eq!(
        (game.observe().skill_value, game.observe().horror_dealt),
        (6, 0)
    );
    assert!(!game.observe().jacqueline_used);
}

#[test]
fn declining_promise_does_not_create_or_draw_a_curse() {
    let mut game = Session::new(Fixture::curse());
    choose(&mut game, "decline-promise");
    choose(&mut game, "use-jacqueline");
    assert!(
        !game
            .observe()
            .tokens
            .iter()
            .any(|t| t.face == TokenFace::Curse)
    );
    cancel(&mut game, &[TokenFace::MinusOne, TokenFace::Zero]);
    assert_eq!(
        (game.observe().skill_value, game.observe().bag_count),
        (3, 16)
    );
    assert_eq!(game.observe().curses_in_pool, 10);
}

#[test]
fn no_eligible_contributor_skips_commitment_but_preserves_jacquelines_reaction() {
    let mut game = Session::new(Fixture::curse().with_patrice_elsewhere());
    let view = game.observe();
    assert_eq!(view.stage, Stage::BeforeReveal);
    assert_eq!(view.revision, 0);
    assert_eq!(view.bag_count, 16);
    assert!(view.tokens.is_empty());
    assert!(!view.promise_committed);
    assert_eq!(
        view.choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["use-jacqueline", "reveal", "end"]
    );
    let before = game.clone();
    assert!(game.choose(0, "decline-promise").is_err());
    assert_eq!(game, before);
    choose(&mut game, "use-jacqueline");
    assert_eq!(game.observe().stage, Stage::Cancellation);
    assert_eq!(game.observe().tokens.len(), 3);
    assert_eq!(game.observe().curses_in_bag, 0);
    assert!(game.observe().jacqueline_used);

    let used = Session::new(
        Fixture::curse()
            .with_patrice_elsewhere()
            .with_jacqueline_used(),
    );
    assert_eq!(
        used.observe()
            .choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["reveal", "end"]
    );
}

#[test]
fn remote_commits_stale_choices_excess_commits_and_reuse_are_refused_without_mutation() {
    let mut remote = Session::new(Fixture::curse().with_patrice_elsewhere());
    let before = remote.clone();
    assert!(remote.choose(0, "commit-promise").is_err());
    assert_eq!(remote, before);

    let mut game = Session::new(Fixture::curse());
    choose(&mut game, "commit-promise");
    let before = game.clone();
    assert!(game.choose(0, "use-jacqueline").is_err());
    assert!(
        game.choose(game.observe().revision, "commit-promise")
            .is_err()
    );
    assert_eq!(game, before);
    choose(&mut game, "use-jacqueline");
    let before = game.clone();
    assert!(game.choose(game.observe().revision, "cancel-0").is_err());
    assert_eq!(game, before);
    cancel(&mut game, &[TokenFace::MinusOne, TokenFace::Zero]);
    let before = game.clone();
    assert!(
        game.choose(game.observe().revision, "use-jacqueline")
            .is_err()
    );
    assert_eq!(game, before);

    let mut already_used = Session::new(Fixture::curse().with_jacqueline_used());
    choose(&mut already_used, "commit-promise");
    let before = already_used.clone();
    assert!(
        already_used
            .choose(already_used.observe().revision, "use-jacqueline")
            .is_err()
    );
    assert_eq!(already_used, before);
}

#[test]
fn player_observations_do_not_expose_future_scripted_draws() {
    let curse = Session::new(Fixture::curse()).observe();
    let auto_fail = Session::new(Fixture::auto_fail()).observe();
    assert_eq!(curse, auto_fail);
}

#[test]
fn player_can_terminate_at_a_pending_cancellation_without_a_fabricated_outcome() {
    let mut game = at_cancellation(Fixture::curse());
    choose(&mut game, "end");
    assert_eq!(game.observe().stage, Stage::Ended);
    assert!(game.observe().choices.is_empty());
    assert_eq!(game.observe().horror_dealt, 0);
    assert_eq!(game.recording().moments.last(), Some(&game.observe()));
}
