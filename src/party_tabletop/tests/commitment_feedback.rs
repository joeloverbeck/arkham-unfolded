use super::*;

fn toggle(context: &mut controls::Context, view: &Observation, id: u16) {
    context.select(view, true, view.revision, Source::Hand(id), Vec3::ZERO);
    context.toggle(view, true, context.menu_epoch(), view.revision, id, false);
}

#[test]
fn supporter_additions_and_actions_follow_the_selected_set_without_replacing_it() {
    let mut session = Session::with_draws(vec![crate::TokenFace::Zero]);
    for id in [
        "select:tony",
        "draw",
        "draw",
        "draw",
        "end-turn",
        "select:patrice",
        "investigate",
    ] {
        session.choose(session.observe().revision, id).unwrap();
    }
    let view = session.observe();
    let mut context = controls::Context::default();
    context.sync(view.revision, None);
    let cue = |context: &controls::Context, id| context.contribution(&view, true, Source::Hand(id));
    assert_eq!(cue(&context, 0).unwrap().bonus, 1);
    assert_eq!(cue(&context, 3).unwrap().bonus, 1);
    assert!(
        cue(&context, 1).is_none(),
        "Guts has no intellect contribution"
    );
    assert_eq!(
        cue(&context, 100).unwrap().bonus,
        4,
        "wild icons use the tested skill"
    );
    toggle(&mut context, &view, 0);
    assert!(
        cue(&context, 0).is_some(),
        "selected card remains removable"
    );
    assert!(cue(&context, 3).is_none(), "one card per supporter");
    assert!(
        cue(&context, 100).is_some(),
        "testing investigator remains eligible"
    );
    toggle(&mut context, &view, 3);
    assert!(context.is_pending(Source::Hand(0)));
    assert!(
        !context.is_pending(Source::Hand(3)),
        "rejected addition cannot replace prior choice"
    );
    assert!(context.confirmation_control(&view, false).is_some());
    context.inspect(&view, cards::index("card/03263").unwrap());
    context.dismiss();
    assert!(context.is_pending(Source::Hand(0)));
    toggle(&mut context, &view, 0);
    assert!(cue(&context, 3).is_some());
    toggle(&mut context, &view, 3);
    assert!(context.is_pending(Source::Hand(3)));
    assert!(
        context
            .contribution(&view, false, Source::Hand(3))
            .is_none(),
        "replay offers no commitment"
    );
    context.sync(view.revision + 1, None);
    assert!(!context.is_pending(Source::Hand(3)));
}

#[test]
fn production_adapter_marks_only_live_commitment_cards_and_clears_after_confirmation() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = render_app(dir.path().join("commitment-cues.json"));
    app.init_resource::<FeedbackSettings>()
        .add_systems(Update, synchronize_feedback.after(scene::synchronize));
    choose(&mut app, "select:patrice");
    choose(&mut app, "investigate");
    let world = app.world_mut();
    let bonuses: Vec<_> = world
        .query::<(&CardSource, &CardFeedback)>()
        .iter(world)
        .filter_map(|(source, mark)| {
            mark.commitment
                .map(|cue| (source.0, cue.bonus, cue.skill.name()))
        })
        .collect();
    assert_eq!(bonuses.len(), 3);
    assert!(bonuses.contains(&(Source::Hand(100), 4, "intellect")));
    choose(&mut app, "commit:100");
    let world = app.world_mut();
    assert!(
        world
            .query::<&CardFeedback>()
            .iter(world)
            .all(|mark| mark.commitment.is_none())
    );
    act(&mut app, Action::Replay);
    act(&mut app, Action::Back);
    let world = app.world_mut();
    assert!(
        world
            .query::<&CardFeedback>()
            .iter(world)
            .all(|mark| mark.commitment.is_none())
    );
}
