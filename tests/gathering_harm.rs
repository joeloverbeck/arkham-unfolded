use arkham_unfolded_experiment::gathering::{HarmFixture, HostileFixture, Session, continuity};

fn choose(session: &mut Session, id: &str) {
    let revision = session.observe().revision;
    session
        .choose(revision, id)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
}

#[test]
fn ward_cancels_its_own_treachery_then_saves_at_the_actual_horror_assignment() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::WardCancellation));
    let before = session.observe();
    let player = &before.play.as_ref().unwrap().players[2];
    let ward = player
        .hand
        .iter()
        .find(|card| card.code == "01065")
        .unwrap()
        .clone();
    let robes = player
        .assets
        .iter()
        .find(|asset| asset.card.code == "60412")
        .unwrap()
        .card
        .id;
    let olive = player
        .assets
        .iter()
        .find(|asset| asset.card.code == "04197")
        .unwrap()
        .card
        .id;
    let id = format!("ward:{}:robes:{robes}", ward.id);
    assert!(
        player.hand.iter().any(|card| card.code == "05032"),
        "The actual Ward horror must not open Deny despite a real copy in hand"
    );
    assert!(
        before.choices.iter().any(|choice| choice.id == id),
        "Ward is a Spell and Robes makes its one-resource cost affordable"
    );
    choose(&mut session, &id);
    let pending = session.observe();
    let play = pending.play.as_ref().unwrap();
    assert!(
        play.test.is_none(),
        "Cancelled Rotting Remains never begins its test"
    );
    assert_eq!(play.players[2].resources, 0);
    assert!(
        !play.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.id == robes)
            .unwrap()
            .ready
    );
    assert!(!play.players[2].hand.iter().any(|card| card.id == ward.id));
    assert!(
        !play.players[2]
            .discard
            .iter()
            .any(|card| card.id == ward.id)
    );
    assert!(
        pending
            .choices
            .iter()
            .any(|choice| choice.id == "harm:horror:investigator")
    );
    assert!(
        pending
            .choices
            .iter()
            .any(|choice| choice.id == format!("harm:horror:{olive}"))
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ward-horror.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(session.observe(), pending);
    let unchanged = session.clone();
    assert!(session.choose(pending.revision, &id).is_err());
    assert_eq!(session, unchanged);
    choose(&mut session, &format!("harm:horror:{olive}"));
    let after = session.observe().play.unwrap();
    assert_eq!(after.players[2].horror, player.horror);
    assert_eq!(
        after.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.id == olive)
            .unwrap()
            .horror,
        1
    );
    assert_eq!(
        after.players[2]
            .discard
            .iter()
            .filter(|card| card.id == ward.id)
            .count(),
        1
    );
    assert!(
        after
            .encounter_discard
            .iter()
            .any(|card| card.code == "01163")
    );
    assert!(after.completed_tests.is_empty());
    session.validate().unwrap();
}

#[test]
fn original_ward_rejects_other_draws_weaknesses_enemies_and_dissonant_voices() {
    for case in [
        HarmFixture::WardOtherDraw,
        HarmFixture::WardWeakness,
        HarmFixture::WardEnemy,
        HarmFixture::WardVoices,
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::Harm(case));
        let observation = session.observe();
        let player = &observation.play.as_ref().unwrap().players[2];
        let ward = player
            .hand
            .iter()
            .find(|card| card.code == "01065")
            .unwrap()
            .id;
        let robes = player
            .assets
            .iter()
            .find(|asset| asset.card.code == "60412")
            .unwrap()
            .card
            .id;
        assert!(
            !observation
                .choices
                .iter()
                .any(|choice| choice.id.starts_with("ward:")),
            "{case:?}"
        );
        let unchanged = session.clone();
        assert!(
            session
                .choose(observation.revision, &format!("ward:{ward}:robes:{robes}"))
                .is_err()
        );
        assert_eq!(session, unchanged);
    }
}

#[test]
fn deny_ignores_one_assigned_attack_aspect_and_preserves_the_other_after_reopen() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::DenyAttack));
    let before = session.observe();
    let player = &before.play.as_ref().unwrap().players[2];
    let deny = player
        .hand
        .iter()
        .find(|card| card.code == "05032")
        .unwrap()
        .id;
    let olive = player
        .assets
        .iter()
        .find(|asset| asset.card.code == "04197")
        .unwrap()
        .card
        .id;
    assert!(
        !before
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("deny:")),
        "Prevention follows assignment"
    );
    choose(&mut session, &format!("harm:damage:{olive}"));
    choose(&mut session, "harm:horror:investigator");
    let assigned = session.observe();
    assert!(
        assigned
            .choices
            .iter()
            .any(|choice| choice.id == format!("deny:{deny}:damage")),
        "After assigning damage and horror, Deny can ignore the attack's entire damage aspect"
    );
    assert!(
        assigned
            .choices
            .iter()
            .any(|choice| choice.id == format!("deny:{deny}:horror"))
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("deny-assigned.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(session.observe(), assigned);
    choose(&mut session, &format!("deny:{deny}:damage"));
    let after = session.observe().play.unwrap();
    assert_eq!(after.players[2].horror, player.horror + 1);
    assert_eq!(
        after.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.id == olive)
            .unwrap()
            .damage,
        0
    );
    assert_eq!(after.players[2].resources, 0);
    assert_eq!(
        after.players[2]
            .discard
            .iter()
            .filter(|card| card.id == deny)
            .count(),
        1
    );
    session.validate().unwrap();
}

#[test]
fn deny_accepts_encounter_weakness_and_reference_harm_but_not_ward_horror() {
    for (case, aspect) in [
        (HarmFixture::DenyChronophobia, "horror"),
        (HarmFixture::DenyTablet, "damage"),
        (HarmFixture::DenyCultist, "horror"),
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::Harm(case));
        let before = session.observe();
        let player = &before.play.as_ref().unwrap().players[2];
        let deny = player
            .hand
            .iter()
            .find(|card| card.code == "05032")
            .unwrap()
            .id;
        choose(&mut session, &format!("harm:{aspect}:investigator"));
        choose(&mut session, &format!("deny:{deny}:{aspect}"));
        let after = session.observe().play.unwrap();
        assert_eq!(
            (after.players[2].damage, after.players[2].horror),
            (player.damage, player.horror)
        );
        assert!(after.players[2].discard.iter().any(|card| card.id == deny));
    }
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::DenySpellHorror));
    let before = session.observe();
    let player = &before.play.as_ref().unwrap().players[2];
    let deny = player
        .hand
        .iter()
        .find(|card| card.code == "05032")
        .unwrap()
        .id;
    let unchanged = session.clone();
    assert!(
        session
            .choose(before.revision, &format!("deny:{deny}:horror"))
            .is_err()
    );
    assert_eq!(session, unchanged);
    choose(&mut session, "harm:horror:investigator");
    let after = session.observe().play.unwrap();
    assert_eq!(after.players[2].horror, player.horror + 1);
    assert!(after.players[2].hand.iter().any(|card| card.id == deny));
}

#[test]
fn harm_origin_not_display_text_determines_deny_eligibility() {
    let mut encounter =
        Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::DenyTabletDisplayText));
    choose(&mut encounter, "harm:damage:investigator");
    let observation = encounter.observe();
    assert!(
        observation
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("deny:") && choice.id.ends_with(":damage")),
        "Changing the reference card's displayed source label cannot remove its encounter origin"
    );
    let mut player_effect =
        Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::DenyPlayerDisplayText));
    let previous_horror = player_effect.observe().play.unwrap().players[2].horror;
    choose(&mut player_effect, "harm:horror:investigator");
    let observation = player_effect.observe();
    assert!(
        !observation
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("deny:")),
        "An encounter-looking display label cannot make a player-card effect eligible"
    );
    assert_eq!(
        observation.play.unwrap().players[2].horror,
        previous_horror + 1
    );
}

#[test]
fn agenda_discard_allows_each_owner_to_ignore_before_random_selection() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::DenyAgenda));
    let before = session.observe().play.unwrap();
    let deny = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "05032")
        .unwrap()
        .id;
    choose(&mut session, "agenda:random-discard");
    let pending = session.observe();
    assert!(
        pending
            .choices
            .iter()
            .any(|choice| choice.id == format!("agenda-discard:deny:{deny}")),
        "Individual choices for one simultaneous effect follow player order, beginning with the lead"
    );
    assert!(
        !pending
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("agenda-discard:order:"))
    );
    assert_eq!(pending.play.as_ref().unwrap().agenda, 1);
    for (index, player) in before.players.iter().enumerate() {
        assert_eq!(
            pending.play.as_ref().unwrap().players[index].hand,
            player.hand
        );
    }
    let owner_window = session.observe();
    let id = format!("agenda-discard:deny:{deny}");
    assert!(owner_window.choices.iter().any(|choice| choice.id == id));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("agenda-deny.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(session.observe(), owner_window);
    choose(&mut session, &id);
    let next_owner = session.observe();
    assert_eq!(
        next_owner.play.as_ref().unwrap().players[0].hand,
        before.players[0].hand,
        "Tony's declined discard waits until Patrice has made her individual prevention choice"
    );
    assert_eq!(
        next_owner.play.as_ref().unwrap().players[1].hand,
        before.players[1].hand
    );
    for _ in 0..8 {
        let observation = session.observe();
        if observation.play.as_ref().unwrap().agenda == 2 {
            break;
        }
        let next = observation
            .choices
            .iter()
            .find(|choice| choice.id == "agenda-discard:resolve")
            .expect("Remaining investigator discard")
            .id
            .clone();
        choose(&mut session, &next);
    }
    let after = session.observe().play.unwrap();
    assert_eq!(after.agenda, 2);
    assert_eq!(
        after.players[0].hand.len(),
        before.players[0].hand.len() - 1
    );
    assert_eq!(
        after.players[1].hand.len(),
        before.players[1].hand.len() - 1
    );
    let retained: Vec<_> = before.players[2]
        .hand
        .iter()
        .filter(|card| card.id != deny)
        .cloned()
        .collect();
    assert_eq!(after.players[2].hand, retained);
    assert_eq!(
        after.players[2]
            .discard
            .iter()
            .filter(|card| card.id == deny)
            .count(),
        1
    );
    session.validate().unwrap();
}

#[test]
fn idol_pays_selected_nonweakness_cards_in_order_then_saves_cancelled_aspects() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::IdolAttack));
    let before = session.observe().play.unwrap();
    let player = &before.players[1];
    let idol = player
        .assets
        .iter()
        .find(|asset| asset.card.code == "09102")
        .unwrap()
        .card
        .id;
    let watcher = player
        .hand
        .iter()
        .find(|card| card.code == "06017")
        .unwrap()
        .id;
    let cards: Vec<_> = player
        .hand
        .iter()
        .filter(|card| !card.weakness)
        .take(2)
        .cloned()
        .collect();
    assert_eq!(cards.len(), 2);
    choose(&mut session, "harm:damage:investigator");
    let assigned = session.observe();
    assert!(
        assigned
            .choices
            .iter()
            .any(|choice| choice.id == format!("idol:{idol}")),
        "Ready Idol offers its optional prevention after assignment"
    );
    choose(&mut session, &format!("idol:{idol}"));
    let selecting = session.observe();
    assert!(
        !selecting
            .choices
            .iter()
            .any(|choice| choice.id == format!("idol-select:{watcher}"))
    );
    let unchanged = session.clone();
    assert!(
        session
            .choose(selecting.revision, &format!("idol-select:{watcher}"))
            .is_err()
    );
    assert_eq!(session, unchanged);
    choose(&mut session, &format!("idol-select:{}", cards[1].id));
    choose(&mut session, &format!("idol-select:{}", cards[0].id));
    choose(&mut session, "idol-pay");
    // Two discards cancel both assigned points, so no per-point choice remains:
    // the session cancels them all and records it.
    let paid = session.observe();
    assert!(
        paid.history.contains(
            &"Patrice Hathaway exhausts Idol of Xanatos and discards 2 chosen cards in the selected order. Cancel that much assigned damage/horror. [09102]"
                .to_owned()
        )
    );
    assert!(
        paid.history
            .contains(&"Automatic: Cancel all assigned harm (1 damage and 1 horror)".to_owned())
    );
    assert!(
        !paid
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("idol-prevent:"))
    );
    let paid = paid.play.unwrap();
    let patrice = &paid.players[1];
    assert!(
        !patrice
            .assets
            .iter()
            .find(|asset| asset.card.id == idol)
            .unwrap()
            .ready
    );
    assert_eq!(
        &patrice.discard[patrice.discard.len() - 2..],
        &[cards[1].clone(), cards[0].clone()]
    );
    assert!(patrice.hand.iter().any(|card| card.id == watcher));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("idol-cancel.json");
    continuity::save_session(&path, &session).unwrap();
    let reopened = continuity::load_session(&path).unwrap();
    assert_eq!(reopened.observe(), session.observe());
    let after = session.observe().play.unwrap();
    assert_eq!(
        (after.players[1].damage, after.players[1].horror),
        (player.damage, player.horror)
    );
    assert!(
        !after.players[1]
            .assets
            .iter()
            .find(|asset| asset.card.id == idol)
            .unwrap()
            .ready
    );
    assert_eq!(
        after.players[1]
            .discard
            .iter()
            .filter(|card| card.id == cards[0].id || card.id == cards[1].id)
            .count(),
        2
    );
    session.validate().unwrap();
}

#[test]
fn idol_can_cancel_before_costs_and_cannot_discard_four_cards_or_reuse_exhausted_asset() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::IdolAttack));
    let before = session.observe().play.unwrap();
    let player = &before.players[1];
    let idol = player
        .assets
        .iter()
        .find(|asset| asset.card.code == "09102")
        .unwrap()
        .card
        .id;
    let cards: Vec<_> = player
        .hand
        .iter()
        .filter(|card| !card.weakness)
        .take(4)
        .map(|card| card.id)
        .collect();
    assert_eq!(cards.len(), 4);
    choose(&mut session, "harm:damage:investigator");
    choose(&mut session, &format!("idol:{idol}"));
    for card in &cards[..3] {
        choose(&mut session, &format!("idol-select:{card}"));
    }
    let selecting = session.observe();
    let unchanged = session.clone();
    assert!(
        session
            .choose(selecting.revision, &format!("idol-select:{}", cards[3]))
            .is_err()
    );
    assert_eq!(session, unchanged);
    choose(&mut session, "idol-cancel");
    let declined = session.observe().play.unwrap();
    assert_eq!(declined.players[1].hand, player.hand);
    assert_eq!(declined.players[1].discard, player.discard);
    assert!(
        declined.players[1]
            .assets
            .iter()
            .find(|asset| asset.card.id == idol)
            .unwrap()
            .ready
    );
    choose(&mut session, &format!("idol:{idol}"));
    choose(&mut session, &format!("idol-select:{}", cards[0]));
    choose(&mut session, "idol-pay");
    assert!(
        session.observe().history.contains(
            &"Patrice Hathaway exhausts Idol of Xanatos and discards 1 chosen card in the selected order. Cancel that much assigned damage/horror. [09102]"
                .to_owned()
        ),
        "one discard is singular"
    );
    choose(&mut session, "idol-prevent:horror:investigator");
    let remaining = session.observe();
    assert!(
        !remaining
            .choices
            .iter()
            .any(|choice| choice.id == format!("idol:{idol}"))
    );
    if remaining
        .choices
        .iter()
        .any(|choice| choice.id == "harm:confirm")
    {
        choose(&mut session, "harm:confirm");
    }
    let after = session.observe().play.unwrap();
    assert_eq!(
        (after.players[1].damage, after.players[1].horror),
        (player.damage + 1, player.horror)
    );
    assert_eq!(after.players[1].hand.len(), player.hand.len() - 1);
}
