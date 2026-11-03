use arkham_unfolded_experiment::gathering::{BackpackFixture, HostileFixture, Session, continuity};
fn choose(session: &mut Session, id: &str) {
    let view = session.observe();
    assert!(
        view.choices.iter().any(|choice| choice.id == id),
        "Missing {id}: {:?}",
        view.choices
    );
    session.choose(view.revision, id).unwrap();
}
fn start(case: BackpackFixture) -> (Session, u16) {
    let mut session = Session::with_hostile_fixture(HostileFixture::Backpack(case));
    let pack = session.observe().play.unwrap().players[0]
        .hand
        .iter()
        .find(|card| card.code == "04037")
        .unwrap()
        .id;
    choose(&mut session, &format!("action:ordinary:play:{pack}"));
    (session, pack)
}
#[test]
fn backpack_searches_the_actual_top_six_and_resumes_selected_facedown_custody() {
    let (mut session, pack) = start(BackpackFixture::Search);
    let entered = session.observe().play.unwrap();
    assert_eq!(entered.players[0].resources, 8);
    assert!(
        arkham_unfolded_experiment::gathering::inspection::card("04037")
            .unwrap()
            .contains("Body")
    );
    assert!(
        entered.searched_player.is_empty(),
        "Optional reaction has not exposed deck cards"
    );
    let deck_count = entered.players[0].deck_count;
    choose(&mut session, "backpack:search");
    let searched = session.observe().play.unwrap();
    assert_eq!(
        searched.players[0].deck_count, deck_count,
        "Search cards stay in their deck"
    );
    assert_eq!(
        searched
            .searched_player
            .iter()
            .map(|card| card.code.as_str())
            .collect::<Vec<_>>(),
        ["04107", "05117", "06011", "60306", "01089", "02038"]
    );
    let selected: Vec<_> = searched.searched_player.iter().take(3).cloned().collect();
    for card in &selected {
        choose(&mut session, &format!("backpack:select:{}", card.id));
    }
    assert!(
        !session.observe().choices.iter().any(
            |choice| choice.id == format!("backpack:select:{}", searched.searched_player[3].id)
        ),
        "At most three attachments"
    );
    for card in &searched.searched_player[4..] {
        assert!(
            !session
                .observe()
                .choices
                .iter()
                .any(|choice| choice.id == format!("backpack:select:{}", card.id))
        );
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("search.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    choose(&mut session, "backpack:attach");
    let after = session.observe().play.unwrap();
    assert!(after.searched_player.is_empty());
    assert_eq!(after.players[0].deck_count, deck_count - 3);
    assert_eq!(after.backpacks[0].asset, pack);
    assert_eq!(after.backpacks[0].cards, selected);
    assert!(
        !after.players[0]
            .hand
            .iter()
            .any(|card| selected.contains(card))
    );
    assert!(
        session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == format!("action:ordinary:play:{}", selected[0].id))
    );
    session.validate().unwrap();
}

#[test]
fn optional_search_no_match_and_short_deck_finish_without_inventing_cards() {
    let (mut declined, pack) = start(BackpackFixture::Search);
    let before = declined.observe().play.unwrap();
    choose(&mut declined, "backpack:decline");
    let after = declined.observe().play.unwrap();
    assert_eq!(after.players[0].deck_count, before.players[0].deck_count);
    assert!(after.players[0].discard.iter().any(|card| card.id == pack));
    assert!(after.searched_player.is_empty());
    let (mut no_match, pack) = start(BackpackFixture::NoMatch);
    choose(&mut no_match, "backpack:search");
    assert_eq!(no_match.observe().play.unwrap().searched_player.len(), 6);
    assert_eq!(no_match.observe().choices.len(), 1);
    choose(&mut no_match, "backpack:attach");
    let after = no_match.observe().play.unwrap();
    assert!(after.backpacks.is_empty());
    assert!(after.players[0].discard.iter().any(|card| card.id == pack));
    let (mut short, pack) = start(BackpackFixture::ShortDeck);
    choose(&mut short, "backpack:search");
    let top = short.observe().play.unwrap().searched_player;
    assert_eq!(
        top.iter()
            .map(|card| card.code.as_str())
            .collect::<Vec<_>>(),
        ["04107", "01089"]
    );
    choose(&mut short, &format!("backpack:select:{}", top[0].id));
    choose(&mut short, "backpack:attach");
    choose(&mut short, &format!("action:ordinary:play:{}", top[0].id));
    let after = short.observe().play.unwrap();
    assert!(after.backpacks.is_empty());
    assert!(after.players[0].discard.iter().any(|card| card.id == pack));
    assert!(
        after.players[0]
            .assets
            .iter()
            .any(|asset| asset.card == top[0])
    );
    assert!(!after.players[0].hand.iter().any(|card| card == &top[0]));
    assert_eq!(after.players[0].deck_count, 1);
    assert_eq!(after.players[0].resources, 6);
}

#[test]
fn completing_a_search_shuffles_even_when_the_owner_attaches_zero_cards() {
    let (mut declined, _) = start(BackpackFixture::Search);
    choose(&mut declined, "backpack:decline");
    choose(&mut declined, "action:ordinary:draw");
    let original_top = declined.observe().play.unwrap().players[0].hand[0].clone();
    assert_eq!(
        original_top.code, "04107",
        "The literal fixture's unshuffled top card"
    );

    let (mut searched, _) = start(BackpackFixture::Search);
    choose(&mut searched, "backpack:search");
    choose(&mut searched, "backpack:attach");
    choose(&mut searched, "action:ordinary:draw");
    let after = searched.observe().play.unwrap();
    assert!(
        !after.players[0].hand.contains(&original_top),
        "This fixed random witness changes the next actual draw: omitting the required shuffle must fail"
    );
    assert_eq!(
        after.players[0].deck_count,
        declined.observe().play.unwrap().players[0].deck_count
    );
}

#[test]
fn facedown_colts_share_the_real_paid_and_free_entry_play_paths() {
    let (mut session, pack) = start(BackpackFixture::PairedColts);
    choose(&mut session, "backpack:search");
    let colts: Vec<_> = session
        .observe()
        .play
        .unwrap()
        .searched_player
        .into_iter()
        .filter(|card| card.code == "06011")
        .collect();
    assert_eq!(colts.len(), 2);
    for card in &colts {
        choose(&mut session, &format!("backpack:select:{}", card.id));
    }
    choose(&mut session, "backpack:attach");
    assert_eq!(session.observe().play.unwrap().players[0].hand.len(), 0);
    choose(
        &mut session,
        &format!("action:ordinary:play:{}", colts[0].id),
    );
    let first = session.observe().play.unwrap();
    assert_eq!(first.players[0].resources, 5);
    assert_eq!(first.backpacks[0].cards, vec![colts[1].clone()]);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("attached-colt.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    choose(
        &mut session,
        &format!("entry:{}:play-colt:{}", colts[0].id, colts[1].id),
    );
    let after = session.observe().play.unwrap();
    assert_eq!(after.players[0].resources, 5);
    assert_eq!(after.actions_remaining[0], 1);
    assert!(after.backpacks.is_empty());
    assert!(after.players[0].discard.iter().any(|card| card.id == pack));
    assert_eq!(
        after.players[0]
            .assets
            .iter()
            .filter(|asset| asset.card.code == "06011" && asset.uses == 3)
            .count(),
        2
    );
    session.validate().unwrap();
}

#[test]
fn crypt_chill_discards_the_host_and_attachments_simultaneously_in_chosen_order() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Backpack(BackpackFixture::Chill));
    let initial = session.observe().play.unwrap();
    let pack = initial.backpacks[0].asset;
    let mut discarded: Vec<_> = initial.backpacks[0]
        .cards
        .iter()
        .map(|card| card.id)
        .collect();
    discarded.insert(1, pack);
    choose(&mut session, "revelation:resolve");
    for id in [
        "window:pass",
        "commit:confirm",
        "window:pass",
        "test:reveal",
    ] {
        if session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == id)
        {
            choose(&mut session, id);
            for attached in &initial.backpacks[0].cards {
                assert!(
                    !session
                        .observe()
                        .choices
                        .iter()
                        .any(|choice| choice.id == format!("commit:{}", attached.id)),
                    "Backpack grants play permission, not commitment permission (Sour Mash has a matching willpower icon)"
                );
            }
        }
    }
    choose(&mut session, &format!("chill:{pack}"));
    let pending = session.observe();
    assert!(pending.play.as_ref().unwrap().backpacks.is_empty());
    assert!(
        !pending.play.as_ref().unwrap().players[0]
            .assets
            .iter()
            .any(|asset| asset.card.id == pack)
    );
    assert_eq!(
        pending
            .choices
            .iter()
            .filter(|choice| choice.id.starts_with("discard-card:"))
            .count(),
        4
    );
    choose(&mut session, &format!("discard-card:{}", discarded[0]));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("host-discard.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    for id in &discarded[1..] {
        if session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == format!("discard-card:{id}"))
        {
            choose(&mut session, &format!("discard-card:{id}"));
        }
    }
    let after = session.observe().play.unwrap();
    assert_eq!(
        after.players[0]
            .discard
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        discarded
    );
    assert!(
        after
            .encounter_discard
            .iter()
            .any(|card| card.code == "01167")
    );
    let record = session.recording();
    continuity::Replay::new(record).unwrap();
}

#[test]
fn lethal_opportunity_keeps_the_paid_attachment_and_all_remaining_owned_cards() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Backpack(BackpackFixture::Defeat));
    let before = session.observe().play.unwrap();
    let pack = before.backpacks[0].asset;
    let attached = before.backpacks[0].cards.clone();
    let owned_count = before.players[0].deck_count
        + before.players[0].hand.len()
        + before.players[0].discard.len()
        + before.players[0].assets.len()
        + before.players[0].threats.len()
        + attached.len();
    let played = attached.iter().find(|card| card.code == "04107").unwrap();
    choose(&mut session, &format!("action:ordinary:play:{}", played.id));
    let paid = session.observe();
    assert_eq!(paid.play.as_ref().unwrap().players[0].resources, 8);
    assert_eq!(paid.play.as_ref().unwrap().backpacks[0].cards.len(), 2);
    assert!(
        !paid.play.as_ref().unwrap().players[0]
            .assets
            .iter()
            .any(|asset| asset.card.id == played.id)
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("paid-attached.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    let attack = session
        .observe()
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("opportunity:"))
        .unwrap()
        .id
        .clone();
    choose(&mut session, &attack);
    let after = session.observe().play.unwrap();
    assert!(after.players[0].defeated);
    assert!(after.backpacks.is_empty());
    for id in attached.iter().map(|card| card.id).chain([pack]) {
        assert_eq!(
            after.players[0]
                .removed
                .iter()
                .filter(|card| card.id == id)
                .count(),
            1,
            "Each owned physical card is removed exactly once"
        );
    }
    assert!(after.players[0].assets.is_empty());
    assert_eq!(
        after.players[0].removed.len(),
        owned_count,
        "Elimination preserves the complete fixed investigator inventory"
    );
    assert_eq!(after.players[0].resources, 0);
    session.validate().unwrap();
}
