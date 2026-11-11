use super::*;

fn choose(session: &mut Session, id: &str) {
    let revision = session.observe().revision;
    session.choose(revision, id).unwrap();
}

/// The Barrier's group clue payment, offered as one listed choice, paid with the
/// Hallway's nine clues as the direct choice naming that allocation.
fn pay_hallway(session: &mut Session) {
    let view = session.observe();
    assert!(
        view.choices
            .iter()
            .any(|choice| choice.id == "act:pay:<contributions>"),
        "The group clue payment is offered: {:?}",
        view.choices
    );
    session.choose(view.revision, "act:pay:3:3:3").unwrap();
}

#[test]
fn investigator_menu_keeps_lead_selection_explicit_and_uses_the_public_choice() {
    let session = Session::with_seed(19);
    let view = session.observe();
    let source = Source::Investigator(Investigator::Tony);
    let choices = source_choices(&view, &source);
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].id, "lead:tony");
    assert!(global_choices(&view).is_empty());
    assert!(pending_choices(&view).is_empty());

    let mut context = Context::default();
    context.sync(0, view.revision);
    assert!(matches!(
        context.select(
            &view,
            Selection {
                source,
                at: Vec3::ZERO,
                title: "Tony Morgan".into(),
                inspect_code: "06003".into(),
            },
            true,
        ),
        Some(SelectionEffect::Menu)
    ));
    assert!(context.is_open());
    assert_eq!(session.observe(), view, "opening a menu makes no decision");
}

#[test]
fn ordinary_first_action_is_on_its_investigator_and_preserves_the_original_choice_id() {
    let mut session = Session::with_seed(19);
    choose(&mut session, "lead:tony");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    choose(&mut session, "turn:patrice");
    let view = session.observe();
    let choices = source_choices(&view, &Source::Investigator(Investigator::Patrice));
    let resource = choices
        .iter()
        .find(|choice| choice.id == "action:ordinary:resource")
        .expect("the investigator offers the public resource action");
    assert!(source_choices(&view, &Source::Investigator(Investigator::Tony)).is_empty());
    assert!(!global_choices(&view).iter().any(|c| c.id == resource.id));
    choose(&mut session, &resource.id);
    let play = session.observe().play.unwrap();
    assert_eq!(play.players[Investigator::Patrice.index()].resources, 6);
    assert_eq!(play.actions_remaining[Investigator::Patrice.index()], 2);
}

#[test]
fn playing_an_asset_is_offered_by_its_physical_hand_card() {
    use crate::gathering::{AssetFixture, HostileFixture};
    let session =
        Session::with_hostile_fixture(HostileFixture::Assets(AssetFixture::RobesOpportunity));
    let view = session.observe();
    let card = view.play.as_ref().unwrap().players[Investigator::Jacqueline.index()]
        .hand
        .iter()
        .find(|card| card.code == "60408")
        .unwrap();
    let id = format!("action:ordinary:play:{}", card.id);
    assert!(
        view.choices.iter().any(|choice| choice.id == id),
        "fixture offers the asset play"
    );
    assert!(
        source_choices(&view, &Source::Card(card.id))
            .iter()
            .any(|choice| choice.id == id)
    );
    assert!(!global_choices(&view).iter().any(|choice| choice.id == id));
}

#[test]
fn unknown_choices_remain_explicit_and_are_never_guessed_from_the_label() {
    let mut view = Session::with_seed(19).observe();
    view.choices = vec![Choice {
        id: "future-rule:06003".into(),
        label: "Choose Tony Morgan as lead investigator".into(),
        cancel_faces: vec![],
    }];
    assert!(source_choices(&view, &Source::Investigator(Investigator::Tony)).is_empty());
    assert!(global_choices(&view).is_empty());
    assert_eq!(pending_choices(&view)[0].id, "future-rule:06003");
}

#[test]
fn result_opportunities_preserve_metadata_and_select_the_first_recognized_decline() {
    let mut view = Session::with_seed(19).observe();
    view.choices = vec![
        Choice {
            id: "defeat:done".into(),
            label: "Finish reactions".into(),
            cancel_faces: vec![crate::TokenFace::Skull],
        },
        Choice {
            id: "tony:quick-thinking:jacqueline".into(),
            label: "Original Quick Thinking wording".into(),
            cancel_faces: vec![crate::TokenFace::AutoFail],
        },
        Choice {
            id: "lucky:decline".into(),
            label: "Decline Lucky!".into(),
            cancel_faces: vec![],
        },
        Choice {
            id: "future:effect".into(),
            label: "An unrecognized effect".into(),
            cancel_faces: vec![crate::TokenFace::ElderSign],
        },
    ];
    let original = view.clone();
    let offered = result_opportunities(&view).unwrap();
    let uses = offered.uses();
    assert_eq!(uses.len(), 2);
    let mut expected = original.choices[1].clone();
    expected.label = "Take Jacqueline's extra action (Quick Thinking)".into();
    assert_eq!(uses[0], expected);
    assert_eq!(uses[1], original.choices[3]);
    let mut decline = original.choices[0].clone();
    decline.label = "Finish enemy-defeat reactions".into();
    assert_eq!(offered.decline(), Some(decline));
    assert_eq!(view, original);

    // A later Lucky! decline admits this observation without a current test,
    // but does not displace the first recognized footer choice.
    view.choices.remove(2);
    assert!(result_opportunities(&view).is_none());
    assert_eq!(pending_choices(&view)[0].id, "future:effect");
}

#[test]
fn only_enemy_defeat_requires_a_current_test_for_result_opportunities() {
    let mut view = Session::with_seed(19).observe();
    for id in [
        "after-test:pass",
        "success:pass",
        "lucky:decline",
        "patrice-shuffle:decline",
        "spell-move:decline",
        "lita:decline-damage",
    ] {
        view.choices = vec![Choice {
            id: id.into(),
            label: "Offered decline".into(),
            cancel_faces: vec![],
        }];
        assert_eq!(
            result_opportunities(&view).unwrap().decline().unwrap().id,
            id
        );
    }
    view.choices[0].id = "future:decline".into();
    assert!(result_opportunities(&view).is_none());
    assert_eq!(pending_choices(&view), view.choices);
}

fn public_fixtures() -> Vec<crate::gathering::HostileFixture> {
    use crate::gathering::{
        AssetFixture as A, BackpackFixture as B, DrawFixture as D, HarmFixture as H,
        HostileFixture as F, LifecycleFixture as L, SearchFixture as S, TestCardFixture as T,
        TokenFixture as K,
    };
    vec![
        F::PatriceCancellationVision,
        F::PatriceCancellationRottingRemains,
        F::GatheringRoundEnd,
        F::GatheringPriestDefeat,
        F::GatheringAgendaTwoNoGhoul,
        F::GatheringAncientEvilsTerminal,
        F::GatheringGhoulMovement,
        F::GatheringRoundEndVoices,
        F::GatheringLitaDamage,
        F::GatheringRetaliate,
        F::GatheringLitaParley,
        F::GatheringNonterminalDefeat,
        F::GatheringHunters,
        F::SimultaneousAssetDefeat,
        F::DissonantVoicesAssetPlay,
        F::JacquelineProtectedTokens,
        F::JacquelineCancellation,
        F::Assets(A::RobesOpportunity),
        F::Assets(A::SignMagickSlots),
        F::Assets(A::SwordCaneOpportunity),
        F::Assets(A::SwordCaneFrozen),
        F::Assets(A::PairedLongColts),
        F::Assets(A::UncageRobes),
        F::Draws(D::PatriceExhaustion),
        F::Draws(D::JacquelineWeaknessOrder),
        F::Search(S::HornetTargets),
        F::Search(S::HornetNoMatch),
        F::Search(S::HuntTargets),
        F::Search(S::HuntNoMatch),
        F::Search(S::TonyEvents),
        F::Search(S::GetOverHereFrozen),
        F::Harm(H::WardCancellation),
        F::Harm(H::WardWeakness),
        F::Harm(H::DenyAttack),
        F::Harm(H::DenyChronophobia),
        F::Harm(H::DenyAgenda),
        F::Harm(H::IdolAttack),
        F::TestCards(T::CrystalPrediction),
        F::TestCards(T::PrescientResolved),
        F::TestCards(T::PromiseCurse),
        F::TestCards(T::ResourcefulSuccess),
        F::TestCards(T::LookFound),
        F::TestCards(T::TonyElderSign),
        F::Tokens(K::DarkProphecy),
        F::Tokens(K::EyesKatarina),
        F::Tokens(K::EyesOlive),
        F::Tokens(K::VoiceRaCurse),
        F::Tokens(K::Premonition),
        F::Tokens(K::JacquelineOlive),
        F::Lifecycle(L::SimultaneousDefeat),
        F::Lifecycle(L::LeadDefeat),
        F::Lifecycle(L::SealedDefeat),
        F::Backpack(B::Search),
        F::Backpack(B::NoMatch),
        F::Backpack(B::ShortDeck),
        F::Backpack(B::PairedColts),
        F::Backpack(B::Chill),
        F::Backpack(B::Defeat),
    ]
}

#[test]
fn public_hostile_decisions_have_explicit_contextual_or_global_routes() {
    let mut missing = vec![];
    for fixture in public_fixtures() {
        let view = Session::with_hostile_fixture(fixture).observe();
        for choice in pending_choices(&view) {
            missing.push(format!("{fixture:?}: {}", choice.id));
        }
    }
    assert!(
        missing.is_empty(),
        "unrouted public fixture decisions: {missing:#?}"
    );
}

#[test]
fn public_continuations_keep_choices_routed_after_zone_and_window_changes() {
    let mut missing = std::collections::BTreeSet::new();
    for fixture in public_fixtures() {
        let mut session = Session::with_hostile_fixture(fixture);
        // Exercise the public choice protocol across subsequent windows and
        // zone moves. This is routing coverage, not a scenario outcome oracle.
        for step in 0..48 {
            let view = session.observe();
            for choice in pending_choices(&view) {
                missing.insert(format!("{fixture:?}: {}", choice.id));
            }
            if view.choices.is_empty() {
                break;
            }
            let selected = global_choices(&view)
                .into_iter()
                .find(|choice| choice.id != "undo")
                .unwrap_or_else(|| {
                    if view.choices[0].id.starts_with("tokens:") {
                        view.choices[step % view.choices.len()].clone()
                    } else {
                        view.choices[0].clone()
                    }
                });
            match session.choose(view.revision, &selected.id) {
                Ok(()) => {}
                // Privileged fixture streams prescribe particular token paths.
                // A different legal branch can exhaust that fixture's token
                // script; ordinary sessions do not have this boundary.
                Err(error) if error.0 == "Verification token absent from bag" => break,
                Err(error) => panic!("{fixture:?}: {}: {error}", selected.id),
            }
        }
    }
    assert!(
        missing.is_empty(),
        "unrouted public continuations: {missing:#?}"
    );
}

#[test]
fn visible_fog_and_sealed_premonition_remain_inspectable_physical_cards() {
    use crate::gathering::{CardView, HostileFixture, TokenFixture};
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::Premonition));
    choose(&mut session, "test:reveal");
    let mut view = session.observe();
    let play = view.play.as_mut().unwrap();
    let sealed = play.premonition.as_ref().unwrap().card.id;
    play.scenario.locations[0].fog = Some(CardView {
        id: 600,
        code: "01168".into(),
        title: "Obscuring Fog".into(),
        weakness: false,
    });
    for id in [600, sealed] {
        assert!(
            exists(&view, &Source::Card(id)),
            "visible copy {id} must allow inspection"
        );
    }
}

#[test]
fn mapped_public_decisions_have_a_rendered_source_in_the_production_scene() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = super::super::tests::host(directory.path().join("context.json"));
    let mut missing = vec![];
    for fixture in public_fixtures() {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.session = Session::with_hostile_fixture(fixture);
        table.epoch += 1;
        table.dirty = true;
        app.update();
        let world = app.world_mut();
        let rendered: Vec<_> = world
            .query::<&ContextTarget>()
            .iter(world)
            .map(|target| target.source.clone())
            .collect();
        let view = world.resource::<Table>().view();
        for choice in &view.choices {
            if let Route::Sources(sources) = route(&view, &choice.id)
                && !sources.iter().any(|source| rendered.contains(source))
            {
                missing.push(format!("{fixture:?}: {} ({sources:?})", choice.id));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "mapped choices without a clickable scene source: {missing:#?}"
    );
}

#[test]
fn card_identity_does_not_admit_hidden_or_other_copies() {
    use crate::gathering::{AssetFixture, HostileFixture};
    let view = Session::with_hostile_fixture(HostileFixture::Assets(AssetFixture::PairedLongColts))
        .observe();
    let cards: Vec<_> = view.play.as_ref().unwrap().players[0]
        .hand
        .iter()
        .filter(|card| card.code == "06011")
        .collect();
    assert_eq!(cards.len(), 2);
    let choices = source_choices(&view, &Source::Card(cards[0].id));
    assert!(!choices.is_empty());
    assert!(choices.iter().all(|choice| {
        let id = choice
            .id
            .strip_prefix("action:ordinary:")
            .unwrap_or(&choice.id);
        id.split(':').nth(1) == Some(cards[0].id.to_string().as_str())
    }));
    assert!(!exists(&view, &Source::Card(u16::MAX)));
    let mut context = Context::default();
    assert!(
        context
            .select(
                &view,
                Selection {
                    source: Source::Card(u16::MAX),
                    at: Vec3::ZERO,
                    title: "Hidden card".into(),
                    inspect_code: "06011".into(),
                },
                true
            )
            .is_none()
    );
}

#[test]
fn rendered_context_action_saves_only_after_explicit_activation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("context.json");
    let mut app = super::super::tests::host(path.clone());
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Select(
            0,
            0,
            Selection {
                source: Source::Investigator(Investigator::Tony),
                at: layout::investigator_card(0),
                title: "Tony Morgan".into(),
                inspect_code: "06003".into(),
            },
        ));
    app.update();
    assert_eq!(
        app.world().resource::<Table>().session.observe().revision,
        0
    );
    assert!(!path.exists(), "selecting a card does not create a save");
    let world = app.world_mut();
    let menu = world
        .query_filtered::<Entity, With<Menu>>()
        .single(world)
        .unwrap();
    let rows = world.get::<Children>(menu).unwrap();
    let button = rows
        .iter()
        .find(|entity| world.get::<Button>(*entity).is_some())
        .unwrap();
    world.trigger(Activate { entity: button });
    app.update();
    let saved = continuity::load_session(&path).unwrap();
    assert_eq!(saved.observe().lead, Some(Investigator::Tony));
    assert_eq!(saved.observe().revision, 1);
}

fn activate_source_choice(session: Session, source: Source, id: &str) -> Session {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source-choice.json");
    let before = session.observe();
    let label = feedback::menu::label(
        &source_choices(&before, &source)
            .into_iter()
            .find(|choice| choice.id == id)
            .unwrap()
            .label,
    );
    let mut expected = session.clone();
    choose(&mut expected, id);
    let mut app = super::super::tests::host(path.clone());
    // Match native startup's reconstruction of reading and presentation state.
    app.insert_resource(Table::new(session, path.clone()));
    app.update();
    super::super::tests::click_source(&mut app, source);
    assert_eq!(app.world().resource::<Table>().session.observe(), before);
    assert!(
        !path.exists(),
        "the actual card observer only opens its menu"
    );
    let world = app.world_mut();
    let menu = world
        .query_filtered::<Entity, With<Menu>>()
        .single(world)
        .unwrap();
    let row = world
        .get::<Children>(menu)
        .unwrap()
        .iter()
        .find(|entity| {
            world
                .get::<AccessibleLabel>(*entity)
                .is_some_and(|value| value.0 == label)
        })
        .expect("the clicked source's actual menu offers the requested legal choice");
    world.trigger(Activate { entity: row });
    app.update();
    let saved = continuity::load_session(path).unwrap();
    assert_eq!(
        saved.observe(),
        expected.observe(),
        "the menu must dispatch the exact original legal ID"
    );
    assert_eq!(saved.observe().revision, before.revision + 1);
    saved
}

#[test]
fn clicked_location_menu_starts_and_saves_the_real_investigation() {
    let mut session = Session::with_seed(19);
    choose(&mut session, "lead:tony");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    choose(&mut session, "turn:patrice");
    let saved = activate_source_choice(
        session,
        Source::Location("01111".into()),
        "action:ordinary:investigate",
    );
    let test = saved.observe().play.unwrap().test.unwrap();
    assert_eq!(test.tester, Investigator::Patrice);
    assert_eq!(test.difficulty, 2);
}

#[test]
fn clicked_enemy_menu_starts_and_saves_the_real_fight() {
    use crate::gathering::{AssetFixture, HostileFixture};
    let session =
        Session::with_hostile_fixture(HostileFixture::Assets(AssetFixture::RobesOpportunity));
    let enemy = session.observe().play.unwrap().scenario.enemies[0].card.id;
    let saved = activate_source_choice(
        session,
        Source::Card(enemy),
        &format!("action:ordinary:fight:{enemy}"),
    );
    assert_eq!(
        saved.observe().play.unwrap().test.unwrap().tester,
        Investigator::Jacqueline
    );
}

#[test]
fn clicked_hand_menu_persists_a_specific_physical_commitment() {
    use crate::gathering::HostileFixture;
    let mut session = Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision);
    let cancel = session
        .observe()
        .choices
        .into_iter()
        .find(|choice| choice.id.starts_with("test-of-will:"))
        .unwrap()
        .id;
    choose(&mut session, &cancel);
    choose(&mut session, "window:pass");
    let card = session.observe().play.unwrap().players[1]
        .hand
        .iter()
        .find(|card| card.code == "01093")
        .unwrap()
        .id;
    let saved = activate_source_choice(session, Source::Card(card), &format!("commit:{card}"));
    assert!(
        saved
            .observe()
            .play
            .unwrap()
            .test
            .unwrap()
            .selected
            .contains(&card)
    );
}

#[test]
fn clicked_physical_token_menu_persists_its_cancellation_selection() {
    use crate::gathering::HostileFixture;
    let mut session = Session::with_hostile_fixture(HostileFixture::JacquelineCancellation);
    for _ in 0..4 {
        let view = session.observe();
        if view.choices.iter().any(|choice| choice.id == "test:reveal") {
            break;
        }
        choose(
            &mut session,
            if view
                .choices
                .iter()
                .any(|choice| choice.id == "commit:confirm")
            {
                "commit:confirm"
            } else {
                "window:pass"
            },
        );
    }
    choose(&mut session, "tokens:jacqueline");
    choose(&mut session, "test:reveal");
    let saved = activate_source_choice(session, Source::Token(0), "tokens:cancel:0");
    assert_eq!(
        saved.observe().play.unwrap().tokens.unwrap().selected,
        vec![0]
    );
}

fn select_tony(app: &mut App) {
    let table = app.world().resource::<Table>();
    let (epoch, revision) = (table.epoch, table.view().revision);
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Select(
            epoch,
            revision,
            Selection {
                source: Source::Investigator(Investigator::Tony),
                at: layout::investigator_card(0),
                title: "Tony Morgan".into(),
                inspect_code: "06003".into(),
            },
        ));
    app.update();
}

#[test]
fn dismiss_and_reselect_rejects_a_queued_action_from_the_old_menu() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("context.json");
    let mut app = super::super::tests::host(path.clone());
    select_tony(&mut app);
    let old = app.world().resource::<Table>().context.generation();
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::DismissContext);
    app.update();
    select_tony(&mut app);
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::ContextChoice(old, 0, 0, "lead:tony".into()));
    app.update();
    assert_eq!(app.world().resource::<Table>().view().revision, 0);
    assert!(!path.exists());
    assert!(app.world().resource::<Table>().context.is_open());
}

#[test]
fn dismissed_inspection_callback_cannot_open_an_old_inspector() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = super::super::tests::host(directory.path().join("context.json"));
    select_tony(&mut app);
    let generation = app.world().resource::<Table>().context.generation();
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::DismissContext,
        Action::ContextInspect(generation, 0, 0, "Tony Morgan".into(), "06003".into()),
    ]);
    app.update();
    assert!(app.world().resource::<Table>().inspection.is_none());
    assert_eq!(app.world().resource::<Table>().view().revision, 0);
}

#[test]
fn health_denial_invalidates_old_menu_actions_after_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("context.json");
    let mut app = super::super::tests::host(path.clone());
    select_tony(&mut app);
    let generation = app.world().resource::<Table>().context.generation();
    app.insert_resource(RenderHealth::default());
    app.update();
    assert!(!app.world().resource::<Table>().context.is_open());
    app.insert_resource(RenderHealth::ready_for_test());
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::ContextChoice(generation, 0, 0, "lead:tony".into()));
    app.update();
    assert_eq!(app.world().resource::<Table>().view().revision, 0);
    assert!(!path.exists());
}

#[test]
fn same_revision_reopen_invalidates_contextual_callbacks() {
    {
        let transition = Action::Reopen;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("context.json");
        let mut app = super::super::tests::host(path.clone());
        continuity::save_session(&path, &app.world().resource::<Table>().session).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        select_tony(&mut app);
        let old = app.world().resource::<Table>().context.generation();
        app.world_mut().resource_mut::<Actions>().0.extend([
            transition,
            Action::ContextChoice(old, 0, 0, "lead:tony".into()),
        ]);
        app.update();
        assert_eq!(
            app.world().resource::<Table>().session.observe().revision,
            0
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(!app.world().resource::<Table>().context.is_open());
    }
}

#[test]
fn failed_contextual_save_preserves_the_open_choice_and_generation() {
    let directory = tempfile::tempdir().unwrap();
    let obstruction = directory.path().join("obstruction");
    std::fs::write(&obstruction, "unchanged").unwrap();
    let mut app = super::super::tests::host(obstruction.join("session.json"));
    select_tony(&mut app);
    let generation = app.world().resource::<Table>().context.generation();
    let before = app.world().resource::<Table>().view();
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::ContextChoice(generation, 0, 0, "lead:tony".into()));
    app.update();
    let table = app.world().resource::<Table>();
    assert_eq!(table.view(), before);
    assert!(table.context.accepts(generation));
    assert!(table.notice.contains("not committed"));
    assert_eq!(std::fs::read_to_string(obstruction).unwrap(), "unchanged");
}

#[test]
fn bag_and_token_read_only_inspection_uses_the_standard_reference() {
    let mut session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::JacquelineCancellation);
    for _ in 0..4 {
        let view = session.observe();
        if view.choices.iter().any(|choice| choice.id == "test:reveal") {
            break;
        }
        choose(
            &mut session,
            if view
                .choices
                .iter()
                .any(|choice| choice.id == "commit:confirm")
            {
                "commit:confirm"
            } else {
                "window:pass"
            },
        );
    }
    choose(&mut session, "tokens:jacqueline");
    choose(&mut session, "test:reveal");
    let view = session.observe();
    for source in [Source::Bag, Source::Token(0)] {
        let mut context = Context::default();
        let effect = context.select(
            &view,
            Selection {
                source,
                at: Vec3::ZERO,
                title: "Chaos bag".into(),
                inspect_code: "chaos-bag".into(),
            },
            false,
        );
        assert!(matches!(effect, Some(SelectionEffect::Inspect(_, code)) if code == "01104"));
    }
}

#[test]
fn room_opens_the_whole_public_enemy_group_once() {
    let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringHunters);
    let view = session.observe();
    let source = Source::Location("01112".into());
    let directory = tempfile::tempdir().unwrap();
    let mut app = super::super::tests::host(directory.path().join("enemy-group.json"));
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.session = session.clone();
        table.epoch += 1;
        table.dirty = true;
    }
    app.update();
    super::super::tests::click_source(&mut app, source.clone());
    let world = app.world_mut();
    let menu = world
        .query_filtered::<Entity, With<Menu>>()
        .single(world)
        .unwrap();
    let labels = world
        .get::<Children>(menu)
        .unwrap()
        .iter()
        .filter_map(|row| {
            world
                .get::<AccessibleLabel>(row)
                .map(|label| label.0.clone())
        })
        .collect::<Vec<_>>();
    assert!(
        labels.iter().any(
            |label| label.starts_with("Inspect Swarm of Rats · 1 of 2\n")
                && label.contains("Damage ")
                && label.contains("Doom ")
                && label.contains("Bounties ")
        ),
        "{labels:?}"
    );
    assert!(
        labels.iter().any(
            |label| label.starts_with("Inspect Swarm of Rats · 2 of 2\n")
                && label.contains("Damage ")
                && label.contains("Doom ")
                && label.contains("Bounties ")
        ),
        "{labels:?}"
    );
    assert!(
        labels
            .iter()
            .any(|label| label.starts_with("Inspect Ghoul Priest\n")),
        "{labels:?}"
    );
    let choices = source_choices(&view, &source);
    assert!(
        !choices.is_empty(),
        "The enemy-phase fixture offers a real Hunter choice"
    );
    let ids = choices
        .iter()
        .map(|choice| choice.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        ids.len(),
        choices.len(),
        "The original clicked enemy is not listed twice"
    );
    for choice in choices {
        assert_eq!(
            labels
                .iter()
                .filter(|label| **label == feedback::menu::label(&choice.label))
                .count(),
            1
        );
    }
    assert_eq!(
        app.world().resource::<Table>().session.clone(),
        session.clone()
    );
}

#[test]
fn displayed_choices_remove_copy_annotations_without_changing_recorded_choices() {
    let mut view = Session::with_seed(19).observe();
    // This annotation shape is emitted by spell_events::choices. The global
    // control path must preserve the same display rule as contextual actions.
    view.choices = vec![Choice {
        id: "window:pass".into(),
        label: "Pass; attack Ghoul Minion (copy 312) for 2 damage".into(),
        cancel_faces: vec![],
    }];
    let recorded = view.choices.clone();
    let shown = global_choices(&view);
    assert_eq!(shown[0].label, "Pass; attack Ghoul Minion for 2 damage");
    assert_eq!(shown[0].id, recorded[0].id);
    assert_eq!(shown[0].cancel_faces, recorded[0].cancel_faces);
    assert_eq!(view.choices, recorded);
}

#[test]
fn recorded_destination_labels_display_known_names_without_rewriting_the_record() {
    let view = Session::with_seed(19).observe();
    for (original, expected) in [
        (
            "Spend an action: Move to 01113",
            "Spend an action: Move to Attic",
        ),
        (
            "Move to 01114 after evading",
            "Move to Cellar after evading",
        ),
        (
            "Spawn Tony's Quarry at 01112",
            "Spawn Tony's Quarry at Hallway",
        ),
    ] {
        assert_eq!(display_history(&view, original), expected);
    }
}

#[test]
fn a_single_enemy_inspect_button_omits_the_repeated_public_state() {
    let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tony(
        crate::gathering::TonyEffectFixture::WatchSuccess,
    ));
    let enemy = session.observe().play.unwrap().scenario.enemies[0].card.id;
    let directory = tempfile::tempdir().unwrap();
    let mut app = super::super::tests::host(directory.path().join("enemy.json"));
    app.insert_resource(Table::new(session, directory.path().join("enemy.json")));
    app.update();
    super::super::tests::click_source(&mut app, Source::Card(enemy));
    assert!(super::super::tests::choice_button(&mut app, "Inspect").is_some());
}

#[test]
fn enemy_inspection_keeps_each_public_physical_state_without_showing_its_id() {
    let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringHunters);
    let view = session.observe();
    let expected = view
        .play
        .as_ref()
        .unwrap()
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.engaged.is_none() && enemy.location == "01112")
        .unwrap()
        .clone();
    let directory = tempfile::tempdir().unwrap();
    let mut app = super::super::tests::host(directory.path().join("enemy-inspection.json"));
    app.insert_resource(Table::new(
        session.clone(),
        directory.path().join("enemy-inspection.json"),
    ));
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    let role: &'static str = Box::leak(format!("card-{}", expected.card.code).into_boxed_str());
    app.insert_resource(art::Art::with_images_for_test([(role, image.clone())]));
    app.update();
    super::super::tests::click_source(&mut app, Source::Location("01112".into()));
    let key = format!("enemy:{}", expected.card.id);
    let table = app.world().resource::<Table>();
    let generation = table.context.generation();
    let epoch = table.epoch;
    let revision = table.view().revision;
    let selection = table.context.selected.as_ref().unwrap();
    let targets = inspections(&table.view(), selection);
    let target = targets.iter().find(|target| target.code == key).unwrap();
    assert!(
        target
            .details
            .as_ref()
            .unwrap()
            .contains(&format!("Damage {}", expected.damage))
    );
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::ContextInspect(
            generation,
            epoch,
            revision,
            expected.card.title.clone(),
            key.clone(),
        ));
    app.update();
    assert_eq!(
        app.world()
            .resource::<Table>()
            .inspection
            .as_ref()
            .unwrap()
            .1,
        key
    );
    let world = app.world_mut();
    assert!(
        world
            .query::<&ImageNode>()
            .iter(world)
            .any(|node| node.image == image)
    );
    let text = super::super::tests::rendered_text(&mut app);
    for detail in [
        format!("Damage {}", expected.damage),
        format!("Doom {}", expected.doom),
        format!("Bounties {}", expected.bounties),
    ] {
        assert!(text.contains(&detail), "{text}");
    }
    assert!(!text.contains(&key));
    assert_eq!(
        app.world().resource::<Table>().session.clone(),
        session.clone()
    );
}

#[test]
fn actual_end_turn_and_bounty_history_use_public_names_instead_of_physical_ids() {
    use crate::gathering::{HostileFixture, PeterFixture};
    let mut forced =
        Session::with_hostile_fixture(HostileFixture::Peter(PeterFixture::ForcedBeforeReaction));
    // Frozen in Fear's is the only end-of-turn effect, so the session begins
    // it itself; its history entry still names the card, not its identity.
    choose(&mut forced, "turn:end");
    let view = forced.observe();
    let raw = view
        .history
        .iter()
        .find(|line| line.starts_with("Automatic: Resolve end-of-turn effect "))
        .unwrap();
    let id = raw.rsplit(' ').next().unwrap();
    let shown = display_history(&view, raw);
    assert!(shown.contains("Frozen in Fear"), "{shown}");
    assert!(!shown.contains(id));

    let mut bounty = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    pay_hallway(&mut bounty);
    choose(&mut bounty, "bounty:400:1");
    let view = bounty.observe();
    let raw = view
        .history
        .iter()
        .find(|line| line.contains("bounties from Bounty Contracts to enemy 400"))
        .unwrap();
    let shown = display_history(&view, raw);
    assert!(shown.contains("Ghoul Priest"));
    assert!(!shown.contains("400"));
    assert!(raw.contains("400"));
}
