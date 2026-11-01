use super::*;
use crate::tabletop::readable_text;

#[test]
fn test_summary_keeps_base_and_commitment_separate_and_recovers_recorded_auto_fail() {
    for (token, chaos, total, outcome) in [
        (TokenFace::Zero, "+0", 4, "Success"),
        (TokenFace::AutoFail, "auto-fail", 0, "Failure"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let mut app = presentation_app(directory.path().join("summary.json"));
        app.world_mut().resource_mut::<Table>().session = Session::with_draws(vec![token]);
        choose(&mut app, "investigate");
        act(&mut app, Action::FinishAnimation);
        choose(&mut app, "commit:0");
        let pending = readable_text("base Intellect 3 · cards +1 · chaos pending · current value 4 · difficulty 2 · Pending".into());
        assert!(
            visible_text(&mut app)
                .iter()
                .any(|text| text.contains(&pending))
        );
        choose(&mut app, "reveal");
        choose(&mut app, "resolve");
        for _ in 0..4 {
            if !app.world().resource::<Table>().presenting() {
                break;
            }
            act(&mut app, Action::FinishAnimation);
        }
        let expected = readable_text(format!(
            "base Intellect 3 · cards +1 · chaos {chaos} · final value {total} · difficulty 2 · {outcome}"
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
        act(&mut app, Action::Replay);
        while app
            .world()
            .resource::<Table>()
            .replay
            .as_ref()
            .unwrap()
            .index()
            + 1
            < app
                .world()
                .resource::<Table>()
                .replay
                .as_ref()
                .unwrap()
                .len()
        {
            act(&mut app, Action::Next);
        }
        assert!(
            visible_text(&mut app)
                .iter()
                .any(|text| text.contains(&expected))
        );
    }
}
