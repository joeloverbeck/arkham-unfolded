use super::*;

fn cancellation_view() -> Observation {
    let mut session = Session::new(Fixture::curse());
    session.choose(0, "commit-promise").unwrap();
    session.choose(1, "use-jacqueline").unwrap();
    session.observe()
}

#[test]
fn staged_cancellation_survives_dismissal_but_only_the_current_callback_is_admitted() {
    let view = cancellation_view();
    let mut context = Context::default();
    context.select(&view, None, Source::Token(0), Vec2::ZERO);
    let old = context.generation();
    assert!(
        context
            .offered_choice(&view, None, old, "cancel-1-2")
            .is_none()
    );
    context.prepare_cancellation(&view, None, old, "cancel-1-2");
    context.dismiss();
    context.select(&view, None, Source::Token(0), Vec2::new(20.0, 30.0));
    assert!(
        context
            .offered_choice(&view, None, old, "cancel-1-2")
            .is_none()
    );
    assert_eq!(
        context
            .offered_choice(&view, None, context.generation(), "cancel-1-2")
            .map(|choice| choice.id.as_str()),
        Some("cancel-1-2"),
        "the selected token need not belong to the complete cancellation set"
    );
}

#[test]
fn source_less_choices_need_live_offers_and_generation_but_no_selection_snapshot() {
    let view = Session::new(Fixture::curse()).observe();
    let mut context = Context::default();
    let generation = context.generation();
    assert!(
        context
            .offered_source_less(&view, None, generation, "decline-promise")
            .is_some()
    );
    context.select(&view, None, Source::Card(1), Vec2::ZERO);
    context.dismiss();
    let generation = context.generation();
    let mut changed = view.clone();
    changed.prompt = "A changed display at the same revision".into();
    assert!(
        context
            .offered_source_less(&changed, None, generation, "decline-promise")
            .is_some(),
        "a stale selection snapshot is not a source-less admission condition"
    );
    for (replay, callback, id) in [
        (Some(0), generation, "decline-promise"),
        (None, generation - 1, "decline-promise"),
        (None, generation, "commit-promise"),
    ] {
        assert!(
            context
                .offered_source_less(&changed, replay, callback, id)
                .is_none()
        );
    }
    assert!(
        context
            .offered_source_less(&cancellation_view(), None, generation, "decline-promise")
            .is_none(),
        "source-less IDs must still be offered by the current observation"
    );
    assert!(context.synchronize(&changed, None));
    assert!(
        context
            .offered_source_less(&changed, None, generation, "decline-promise")
            .is_none(),
        "explicit synchronization still expires the old callback"
    );
}

#[test]
fn synchronization_compares_the_full_observation_and_replay_position() {
    let view = cancellation_view();
    let mut context = Context::default();
    context.select(&view, None, Source::Token(0), Vec2::ZERO);
    context.prepare_cancellation(&view, None, context.generation(), "cancel-1-2");
    assert!(!context.synchronize(&view, None));
    let mut changed = view.clone();
    changed.prompt = "A changed display at the same revision".into();
    assert!(
        context
            .offered_choice(&changed, None, context.generation(), "cancel-1-2")
            .is_none()
    );
    assert!(context.synchronize(&changed, None));
    context.select(&changed, None, Source::Token(0), Vec2::ZERO);
    assert!(
        context
            .offered_choice(&changed, None, context.generation(), "cancel-1-2")
            .is_none(),
        "synchronization discarded the old staged cancellation"
    );

    context.select(&view, Some(2), Source::Token(0), Vec2::ZERO);
    assert!(!context.synchronize(&view, Some(2)));
    assert!(context.synchronize(&view, Some(3)));
    assert_eq!(context.selected_source(), None);
}

#[test]
fn menu_closures_retain_cancellation_and_preserve_their_utility_effects() {
    let view = cancellation_view();
    let mut context = Context::default();
    context.select(&view, None, Source::Token(0), Vec2::ZERO);
    context.prepare_cancellation(&view, None, context.generation(), "cancel-1-2");
    context.toggle_utilities();
    assert!(context.utilities_open());
    for (close, utilities) in [
        (Context::close_menu as fn(&mut Context), true),
        (Context::toggle_utilities, false),
        (Context::dismiss, false),
    ] {
        context.select(&view, None, Source::Token(0), Vec2::ZERO);
        let generation = context.generation();
        close(&mut context);
        assert_eq!(context.utilities_open(), utilities);
        assert!(
            context
                .offered_choice(&view, None, generation, "cancel-1-2")
                .is_none()
        );
        context.select(&view, None, Source::Token(0), Vec2::ZERO);
        assert!(
            context
                .offered_choice(&view, None, context.generation(), "cancel-1-2")
                .is_some()
        );
    }
    assert!(!context.utilities_open());
    context.toggle_utilities();
    context.select(&view, None, Source::Token(0), Vec2::ZERO);
    assert!(context.camera_moved());
    assert!(
        context.utilities_open(),
        "selection and camera closure retain utilities"
    );
    let generation = context.generation();
    assert!(!context.camera_moved());
    assert!(
        context
            .offered_source_less(&view, None, generation, "end")
            .is_some()
    );
    context.select(&view, None, Source::Token(0), Vec2::ZERO);
    assert!(
        context
            .offered_choice(&view, None, context.generation(), "cancel-1-2")
            .is_some()
    );
}

#[test]
fn preparation_requires_a_current_live_token_offer_but_not_an_open_menu() {
    let view = cancellation_view();
    let mut context = Context::default();
    context.select(&view, None, Source::Card(2), Vec2::ZERO);
    context.prepare_cancellation(&view, None, context.generation(), "cancel-1-2");
    assert_eq!(context.pending_cancellation(), None);
    context.select(&view, None, Source::Token(0), Vec2::ZERO);
    let generation = context.generation();
    for (callback, id) in [(generation - 1, "cancel-1-2"), (generation, "cancel-1")] {
        context.prepare_cancellation(&view, None, callback, id);
        assert_eq!(context.pending_cancellation(), None);
    }
    context.close_menu();
    context.prepare_cancellation(&view, None, context.generation(), "cancel-1-2");
    assert_eq!(context.pending_cancellation(), Some("cancel-1-2"));
    assert!(
        context
            .offered_choice(&view, None, context.generation(), "cancel-1-2")
            .is_none()
    );
    context.select(&view, None, Source::Token(0), Vec2::ZERO);
    assert!(
        context
            .offered_choice(&view, None, context.generation(), "cancel-1-2")
            .is_some()
    );
    assert!(
        context
            .offered_choice(&view, None, context.generation(), "cancel-0-1")
            .is_none()
    );

    context.select(&view, Some(2), Source::Token(0), Vec2::ZERO);
    context.prepare_cancellation(&view, Some(2), context.generation(), "cancel-1-2");
    assert_eq!(context.pending_cancellation(), None);
    assert!(
        context
            .offered_choice(&view, Some(2), context.generation(), "cancel-1-2")
            .is_none()
    );
}
