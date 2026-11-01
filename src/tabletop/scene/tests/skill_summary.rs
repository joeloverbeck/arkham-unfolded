use super::*;

fn assert_summary(app: &mut App, expected: &str) {
    let expected = readable_text(expected.into());
    let world = app.world_mut();
    assert!(
        world
            .query::<&Text>()
            .iter(world)
            .any(|text| text.0.contains(&expected))
    );
}

fn choose(app: &mut App, id: &str) {
    let revision = app.world().resource::<Tabletop>().view().revision;
    act(app, Action::Choose(revision, id.into()));
}

#[test]
fn test_summary_distinguishes_pending_additional_tokens_from_final_value() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = app_for_session(
        Session::new(Fixture::curse()),
        directory.path().join("summary.json"),
    );
    choose(&mut app, "commit-promise");
    assert_summary(
        &mut app,
        "base Willpower 5 · cards +4 · chaos pending · current value 9 · difficulty 3 · Pending",
    );
    choose(&mut app, "reveal");
    assert_summary(
        &mut app,
        "base Willpower 5 · cards +4 · chaos -2 (more pending) · current value 7 · difficulty 3 · Pending",
    );
    choose(&mut app, "use-jacqueline");
    choose(&mut app, "cancel-1-2");
    assert_summary(
        &mut app,
        "base Willpower 5 · cards +4 · chaos -4 · final value 5 · difficulty 3 · Success",
    );
    act(&mut app, Action::Reopen);
    assert_summary(
        &mut app,
        "base Willpower 5 · cards +4 · chaos -4 · final value 5 · difficulty 3 · Success",
    );
}

#[test]
fn ending_an_unfinished_test_does_not_claim_a_final_value_or_success() {
    for (choices, expected) in [
        (
            vec!["end"],
            "base Willpower 5 · cards +0 · chaos pending · current value 5 · difficulty 3 · Ended before completion",
        ),
        (
            vec!["commit-promise", "reveal", "end"],
            "base Willpower 5 · cards +4 · chaos -2 (more pending) · current value 7 · difficulty 3 · Ended before completion",
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let mut app = app_for_session(
            Session::new(Fixture::curse()),
            directory.path().join("abandoned.json"),
        );
        for choice in choices {
            choose(&mut app, choice);
        }
        assert_summary(&mut app, expected);
        act(&mut app, Action::Reopen);
        assert_summary(&mut app, expected);
    }
}
