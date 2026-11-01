use super::*;

#[test]
fn test_summary_separates_base_cards_and_chaos_through_completion_and_reopen() {
    for (token, chaos, final_value, outcome) in [
        (crate::TokenFace::Zero, "+0", 4, "Success"),
        (crate::TokenFace::MinusFour, "-4", 0, "Failure"),
        (crate::TokenFace::AutoFail, "auto-fail", 0, "Failure"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let (mut app, _) = super::selection_lifecycle::production_app(
            directory.path().join("summary.json"),
            RenderHealth::ready_for_test(),
        );
        app.world_mut().resource_mut::<Table>().session = Session::with_draws(vec![token]);
        choose(&mut app, "select:tony");
        choose(&mut app, "investigate");
        let card = app
            .world()
            .resource::<Table>()
            .view()
            .investigator(Investigator::Tony)
            .hand
            .iter()
            .find(|card| card.title == "On the Hunt")
            .unwrap()
            .id;
        choose(&mut app, &format!("commit:{card}"));
        let pending = crate::tabletop::readable_text("base Intellect 3 · cards +1 · chaos pending · current value 4 · difficulty 2 · Pending".into());
        assert!(
            visible_text(&mut app)
                .iter()
                .any(|text| text.contains(&pending))
        );
        choose(&mut app, "reveal");
        assert!(
            visible_text(&mut app)
                .iter()
                .any(|text| text.contains(&pending))
        );
        choose(&mut app, "resolve:0");
        // Legacy continuations may still expose their recorded finish decision.
        if app
            .world()
            .resource::<Table>()
            .view()
            .choices
            .iter()
            .any(|choice| choice.id == "finish-test")
        {
            choose(&mut app, "finish-test");
        }
        let expected = crate::tabletop::readable_text(format!(
            "base Intellect 3 · cards +1 · chaos {chaos} · final value {final_value} · difficulty 2 · {outcome}"
        ));
        assert!(
            visible_text(&mut app)
                .iter()
                .any(|text| text.contains(&expected))
        );
        act(&mut app, Action::Reopen);
        assert!(
            visible_text(&mut app)
                .iter()
                .any(|text| text.contains(&expected))
        );
    }
}
