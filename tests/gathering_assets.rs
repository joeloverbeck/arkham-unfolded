use arkham_unfolded_experiment::gathering::{AssetFixture, HostileFixture, Session, continuity};

fn choose(session: &mut Session, id: &str) {
    session
        .choose(session.observe().revision, id)
        .unwrap_or_else(|error| panic!("{id}: {error}; choices: {:?}", session.observe().choices));
}

fn completed_setup() -> Session {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringFromSetup);
    choose(&mut session, "lead:tony");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    session
}

#[test]
fn robes_discount_is_optional_paid_before_opportunity_and_not_repeated_after_reopen() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Assets(AssetFixture::RobesOpportunity));
    let before = session.observe();
    let jacqueline = &before.play.as_ref().unwrap().players[2];
    let robes = jacqueline
        .assets
        .iter()
        .find(|asset| asset.card.code == "60412")
        .unwrap()
        .card
        .id;
    let spell = jacqueline
        .hand
        .iter()
        .find(|card| card.code == "60408")
        .unwrap()
        .clone();
    let ordinary = format!("action:ordinary:play:{}", spell.id);
    let reduced = format!("{ordinary}:robes:{robes}");
    assert!(before.choices.iter().any(|choice| choice.id == ordinary));
    assert!(
        before.choices.iter().any(|choice| choice.id == reduced),
        "Original Robes may exhaust to reduce Clairvoyance from 4 to 3"
    );
    let mut declined = session.clone();
    choose(&mut declined, &ordinary);
    let declined = declined.observe().play.unwrap();
    assert_eq!(declined.players[2].resources, 1);
    assert!(
        declined.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.id == robes)
            .unwrap()
            .ready
    );

    choose(&mut session, &reduced);
    let paid = session.observe();
    let player = &paid.play.as_ref().unwrap().players[2];
    assert_eq!(player.resources, 2);
    assert!(
        !player
            .assets
            .iter()
            .find(|asset| asset.card.id == robes)
            .unwrap()
            .ready
    );
    assert!(!player.hand.iter().any(|card| card.id == spell.id));
    assert!(!player.assets.iter().any(|asset| asset.card.id == spell.id));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("robes-paid.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(session.observe(), paid);
    let unchanged = session.clone();
    assert!(session.choose(paid.revision, &reduced).is_err());
    assert_eq!(session, unchanged);
    let attack = paid
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("opportunity:"))
        .unwrap()
        .id
        .clone();
    choose(&mut session, &attack);
    for _ in 0..2 {
        choose(&mut session, "harm:damage:investigator");
    }
    // With no horror soak, the remaining horror has no player allocation choice.
    let after = session.observe().play.unwrap();
    let player = &after.players[2];
    assert_eq!(
        player.resources, 2,
        "Resuming the paid Spell must not pay again"
    );
    assert_eq!(player.damage, jacqueline.damage + 2);
    assert_eq!(player.horror, jacqueline.horror + 2);
    assert_eq!(
        player
            .assets
            .iter()
            .find(|asset| asset.card == spell)
            .unwrap()
            .uses,
        3
    );
    assert_eq!(after.actions_remaining[2], 2);
    let eyes = player
        .hand
        .iter()
        .find(|card| card.code == "11067")
        .unwrap()
        .id;
    let reuse = format!("action:ordinary:play:{eyes}:robes:{robes}");
    let observation = session.observe();
    assert!(
        !observation.choices.iter().any(|choice| choice.id == reuse),
        "Exhausted Robes cannot make the second three-cost Spell affordable from two resources"
    );
    let unchanged = session.clone();
    assert!(session.choose(observation.revision, &reuse).is_err());
    assert_eq!(session, unchanged);
}

#[test]
fn sign_magick_fast_play_avoids_opportunity_and_opens_a_third_spell_slot() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Assets(AssetFixture::SignMagickSlots));
    let before = session.observe();
    let player = &before.play.as_ref().unwrap().players[2];
    let sign = player
        .hand
        .iter()
        .find(|card| card.code == "05112")
        .unwrap()
        .clone();
    let eyes = player
        .hand
        .iter()
        .find(|card| card.code == "11067")
        .unwrap()
        .clone();
    let third_spell = format!("action:ordinary:play:{}", eyes.id);
    assert!(
        !before.choices.iter().any(|choice| choice.id == third_spell),
        "The two printed arcane slots are occupied"
    );
    let ordinary_sign = format!("action:ordinary:play:{}", sign.id);
    assert!(
        !before
            .choices
            .iter()
            .any(|choice| choice.id == ordinary_sign)
    );
    let fast = format!("fast-asset:{}", sign.id);
    assert!(
        before.choices.iter().any(|choice| choice.id == fast),
        "Sign Magick is Fast on its owner's turn"
    );
    let mut other_turn = session.clone();
    choose(&mut other_turn, "turn:end");
    choose(&mut other_turn, "turn:tony");
    let outside_turn = other_turn.observe();
    assert!(!outside_turn.choices.iter().any(|choice| choice.id == fast));
    let unchanged = other_turn.clone();
    assert!(other_turn.choose(outside_turn.revision, &fast).is_err());
    assert_eq!(
        other_turn, unchanged,
        "A Fast asset cannot be played outside its owner's turn"
    );
    choose(&mut session, &fast);
    let after = session.observe();
    let play = after.play.as_ref().unwrap();
    assert_eq!(play.actions_remaining[2], 3);
    assert_eq!(
        play.players[2].resources, 3,
        "Original Sign Magick costs 3 resources"
    );
    assert_eq!(play.players[2].damage, player.damage);
    assert_eq!(play.players[2].horror, player.horror);
    assert!(
        play.players[2]
            .assets
            .iter()
            .any(|asset| asset.card == sign)
    );
    assert!(
        !after
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("opportunity:"))
    );
    assert!(
        after.choices.iter().any(|choice| choice.id == third_spell),
        "The new restricted slot holds this Spell without discarding either existing Spell"
    );
    choose(&mut session, &third_spell);
    let pending = session.observe();
    let attack = pending
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("opportunity:"))
        .unwrap()
        .id
        .clone();
    choose(&mut session, &attack);
    let after = session.observe().play.unwrap();
    assert_eq!(after.actions_remaining[2], 2);
    assert_eq!(after.players[2].resources, 0);
    assert!(
        after.players[2]
            .assets
            .iter()
            .any(|asset| asset.card == eyes)
    );
    assert!(after.players[2].discard.is_empty());
}

#[test]
fn sword_cane_entry_is_optional_free_ready_and_resumes_before_later_paid_activation() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Assets(AssetFixture::SwordCaneOpportunity));
    let before = session.observe();
    let player = &before.play.as_ref().unwrap().players[2];
    let cane = player
        .hand
        .iter()
        .find(|card| card.code == "07029")
        .unwrap()
        .clone();
    let play = format!("action:ordinary:play:{}", cane.id);
    assert!(
        before.choices.iter().any(|choice| choice.id == play),
        "Sword Cane must have its actual entry reaction available through play"
    );
    choose(&mut session, &play);
    let entry = session.observe();
    let current = entry.play.as_ref().unwrap();
    assert_eq!(current.players[2].resources, 3);
    assert_eq!(current.players[2].damage, player.damage);
    assert_eq!(current.players[2].horror, player.horror);
    assert_eq!(current.actions_remaining[2], 2);
    assert!(
        !entry
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("opportunity:"))
    );
    let decline = format!("entry:{}:decline", cane.id);
    assert!(entry.choices.iter().any(|choice| choice.id == decline));
    let mut declined = session.clone();
    choose(&mut declined, &decline);
    assert!(declined.observe().play.unwrap().test.is_none());

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("cane-entry.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(session.observe(), entry);
    let free = format!("entry:{}:evade:cane:{}:400:willpower", cane.id, cane.id);
    assert!(entry.choices.iter().any(|choice| choice.id == free));
    assert!(
        entry
            .choices
            .iter()
            .any(|choice| choice.id
                == format!("entry:{}:evade:cane:{}:400:agility", cane.id, cane.id))
    );
    choose(&mut session, &free);
    let testing = session.observe().play.unwrap();
    assert_eq!(testing.test.as_ref().unwrap().base_skill, 5);
    assert_eq!(testing.test.as_ref().unwrap().difficulty, 4);
    assert_eq!(testing.actions_remaining[2], 2);
    assert!(
        testing.players[2]
            .assets
            .iter()
            .find(|asset| asset.card == cane)
            .unwrap()
            .ready,
        "Free entry activation waives both action and exhaust costs"
    );
    for _ in 0..4 {
        let choices = session.observe().choices;
        if choices.iter().any(|choice| choice.id == "test:reveal") {
            break;
        }
        choose(
            &mut session,
            if choices.iter().any(|choice| choice.id == "commit:confirm") {
                "commit:confirm"
            } else {
                "window:pass"
            },
        );
    }
    choose(&mut session, "test:reveal");
    let after = session.observe().play.unwrap();
    let priest = after
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == 400)
        .unwrap();
    assert!(priest.exhausted);
    assert!(priest.engaged.is_none());
    let normal = format!("action:ordinary:fight:cane:{}:400:willpower", cane.id);
    choose(&mut session, &normal);
    let paid = session.observe().play.unwrap();
    assert_eq!(paid.actions_remaining[2], 1);
    assert!(
        !paid.players[2]
            .assets
            .iter()
            .find(|asset| asset.card == cane)
            .unwrap()
            .ready,
        "A later ordinary activation pays the printed exhaust cost"
    );
}

#[test]
fn sword_cane_waives_its_own_costs_but_pays_both_frozen_in_fear_surcharges() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Assets(AssetFixture::SwordCaneFrozen));
    let before = session.observe().play.unwrap();
    let cane = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "07029")
        .unwrap()
        .id;
    assert_eq!(
        before.players[2]
            .threats
            .iter()
            .filter(|card| card.code == "01164")
            .count(),
        2
    );
    choose(&mut session, &format!("action:ordinary:play:{cane}"));
    assert_eq!(session.observe().play.unwrap().actions_remaining[2], 2);
    choose(
        &mut session,
        &format!("entry:{cane}:evade:cane:{cane}:400:willpower"),
    );
    let pending = session.observe().play.unwrap();
    assert_eq!(
        pending.actions_remaining[2], 0,
        "The official Frozen FAQ taxes free bold actions and stacks copies; FAQ2.24 preserves external additional costs"
    );
    assert!(
        pending.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.id == cane)
            .unwrap()
            .ready
    );
    assert_eq!(pending.test.unwrap().base_skill, 5);
}

#[test]
fn paired_long_colts_reopen_at_optional_free_play_and_pay_only_the_first_play() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Assets(AssetFixture::PairedLongColts));
    let before = session.observe();
    let colts: Vec<_> = before.play.as_ref().unwrap().players[0]
        .hand
        .iter()
        .filter(|card| card.code == "06011")
        .cloned()
        .collect();
    assert_eq!(colts.len(), 2);
    let play = format!("action:ordinary:play:{}", colts[0].id);
    assert!(
        before.choices.iter().any(|choice| choice.id == play),
        "The actual selected signature must be playable with its entry reaction"
    );
    choose(&mut session, &play);
    let paid = session.observe();
    assert_eq!(paid.play.as_ref().unwrap().players[0].resources, 2);
    let opportunity = paid
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("opportunity:"))
        .unwrap()
        .id
        .clone();
    choose(&mut session, &opportunity);
    let entry = session.observe();
    let first = &entry.play.as_ref().unwrap().players[0];
    assert_eq!(
        first.damage,
        before.play.as_ref().unwrap().players[0].damage + 2
    );
    assert_eq!(
        first.horror,
        before.play.as_ref().unwrap().players[0].horror + 2
    );
    assert!(
        first
            .assets
            .iter()
            .any(|asset| asset.card == colts[0] && asset.uses == 3)
    );
    let decline = format!("entry:{}:decline", colts[0].id);
    let mut declined = session.clone();
    choose(&mut declined, &decline);
    assert!(
        declined.observe().play.unwrap().players[0]
            .hand
            .iter()
            .any(|card| card == &colts[1])
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("paired-colt-entry.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(session.observe(), entry);
    choose(
        &mut session,
        &format!("entry:{}:play-colt:{}", colts[0].id, colts[1].id),
    );
    let after = session.observe().play.unwrap();
    assert_eq!(after.players[0].resources, 2);
    assert_eq!(after.actions_remaining[0], 2);
    assert_eq!(
        after.players[0].damage, first.damage,
        "A card play from this reaction is not a new Play action or opportunity attack"
    );
    assert_eq!(after.players[0].horror, first.horror);
    for colt in colts {
        assert!(
            after.players[0]
                .assets
                .iter()
                .any(|asset| asset.card == colt && asset.uses == 3)
        );
        assert!(!after.players[0].hand.iter().any(|card| card.id == colt.id));
    }
    session.validate().unwrap();
}

#[test]
fn uncage_selects_and_pays_its_spell_after_opportunity_with_optional_robes_and_saved_limbo() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Assets(AssetFixture::UncageRobes));
    let before = session.observe();
    let player = &before.play.as_ref().unwrap().players[2];
    let event = player
        .hand
        .iter()
        .find(|card| card.code == "03033")
        .unwrap()
        .clone();
    let spell = player
        .hand
        .iter()
        .find(|card| card.code == "60408")
        .unwrap()
        .clone();
    let robes = player
        .assets
        .iter()
        .find(|asset| asset.card.code == "60412")
        .unwrap()
        .card
        .id;
    let play = format!("action:ordinary:play:tony:{}:03033", event.id);
    assert!(
        before.choices.iter().any(|choice| choice.id == play),
        "Uncage plus Robes makes the actual four-cost Spell affordable with zero resources"
    );
    choose(&mut session, &play);
    let paid = session.observe();
    let player = &paid.play.as_ref().unwrap().players[2];
    assert!(!player.hand.iter().any(|card| card.id == event.id));
    assert!(!player.discard.iter().any(|card| card.id == event.id));
    assert!(
        player.hand.iter().any(|card| card.id == spell.id),
        "The nested Spell has not been selected or paid before the outer event's opportunity attack"
    );
    assert!(
        player
            .assets
            .iter()
            .find(|asset| asset.card.id == robes)
            .unwrap()
            .ready
    );
    let attack = paid
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("opportunity:"))
        .unwrap()
        .id
        .clone();
    choose(&mut session, &attack);
    choose(&mut session, "harm:damage:investigator");
    choose(&mut session, "harm:damage:investigator");
    let targets = session.observe();
    let full = format!("uncage:{}", spell.id);
    let reduced = format!("{full}:robes:{robes}");
    assert!(!targets.choices.iter().any(|choice| choice.id == full));
    assert!(targets.choices.iter().any(|choice| choice.id == reduced));
    let player = &targets.play.as_ref().unwrap().players[2];
    let sign = player
        .hand
        .iter()
        .find(|card| card.code == "05112")
        .unwrap()
        .clone();
    let cane = player
        .hand
        .iter()
        .find(|card| card.code == "07029")
        .unwrap();
    let ineligible = format!("uncage:{}", cane.id);
    assert!(
        !targets.choices.iter().any(|choice| choice.id == ineligible),
        "Sword Cane is neither Spell nor Ritual"
    );
    let mut refused = session.clone();
    assert!(refused.choose(targets.revision, &ineligible).is_err());
    assert_eq!(refused, session);
    let mut ritual = session.clone();
    choose(&mut ritual, &format!("uncage:{}", sign.id));
    choose(&mut ritual, "search:finish-event");
    let ritual = ritual.observe().play.unwrap();
    assert!(
        ritual.players[2]
            .assets
            .iter()
            .any(|asset| asset.card == sign),
        "Sign Magick is a Ritual, so Uncage can play it without spending another action"
    );
    assert_eq!(ritual.players[2].resources, 0);
    assert_eq!(ritual.actions_remaining[2], 2);
    assert!(
        ritual.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.id == robes)
            .unwrap()
            .ready
    );
    assert!(
        !targets.play.as_ref().unwrap().players[2]
            .discard
            .iter()
            .any(|card| card.id == event.id)
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("uncage-target.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(session.observe(), targets);
    choose(&mut session, &reduced);
    choose(&mut session, "search:finish-event");
    let after = session.observe().play.unwrap();
    let player = &after.players[2];
    assert_eq!(player.resources, 0);
    assert_eq!(after.actions_remaining[2], 2);
    assert!(
        player
            .assets
            .iter()
            .any(|asset| asset.card == spell && asset.uses == 3)
    );
    assert!(
        !player
            .assets
            .iter()
            .find(|asset| asset.card.id == robes)
            .unwrap()
            .ready
    );
    assert_eq!(
        player
            .discard
            .iter()
            .filter(|card| card.id == event.id)
            .count(),
        1
    );
    assert_eq!(
        player.damage, 2,
        "Nested play does not provoke a second opportunity attack"
    );
    session.validate().unwrap();
}

#[test]
fn uncage_waits_for_voice_of_ra_tokens_before_finishing_the_outer_event() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Assets(AssetFixture::UncageRobes));
    let before = session.observe().play.unwrap();
    let event = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "03033")
        .unwrap()
        .clone();
    let voice = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "60416")
        .unwrap()
        .clone();
    let robes = before.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60412")
        .unwrap()
        .card
        .id;
    choose(
        &mut session,
        &format!("action:ordinary:play:tony:{}:03033", event.id),
    );
    let attack = session
        .observe()
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("opportunity:"))
        .unwrap()
        .id
        .clone();
    choose(&mut session, &attack);
    choose(&mut session, "harm:damage:investigator");
    choose(&mut session, "harm:damage:investigator");
    let target = format!("uncage:spell:{}", voice.id);
    assert!(
        session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == target),
        "Voice of Ra is a selected Spell event, eligible for Uncage"
    );
    choose(&mut session, &target);
    let revealing = session.observe();
    assert!(
        !revealing
            .choices
            .iter()
            .any(|choice| choice.id == "search:finish-event"),
        "The outer Uncage cannot finish while its nested non-test token reveal is pending"
    );
    assert!(
        revealing
            .choices
            .iter()
            .any(|choice| choice.id == "test:reveal")
    );
    let player = &revealing.play.as_ref().unwrap().players[2];
    assert_eq!(
        player.resources, 1,
        "Voice gives its first resource before revealing"
    );
    assert!(
        player
            .assets
            .iter()
            .find(|asset| asset.card.id == robes)
            .unwrap()
            .ready,
        "A zero-cost Spell cannot exhaust Robes for a redundant discount"
    );
    assert!(
        !player
            .discard
            .iter()
            .any(|card| card.id == event.id || card.id == voice.id)
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("uncage-voice.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(session.observe(), revealing);
    choose(&mut session, "test:reveal");
    let nested_complete = session.observe().play.unwrap();
    assert_eq!(
        nested_complete.players[2].resources, 5,
        "The fixed skull, +1, elder sign batch contributes two taboo symbols, for 1+4 resources"
    );
    assert!(
        nested_complete.players[2]
            .discard
            .iter()
            .any(|card| card.id == voice.id)
    );
    assert!(
        !nested_complete.players[2]
            .discard
            .iter()
            .any(|card| card.id == event.id)
    );
    choose(&mut session, "search:finish-event");
    let completed = session.observe().play.unwrap();
    assert_eq!(completed.actions_remaining[2], 2);
    assert_eq!(completed.players[2].resources, 5);
    assert_eq!(
        completed.players[2]
            .discard
            .iter()
            .filter(|card| card.id == event.id)
            .count(),
        1
    );
    assert_eq!(completed.bag_count, 16);
}

#[test]
fn dissonant_voices_blocks_asset_play_without_spending_its_costs() {
    let mut session = Session::with_hostile_fixture(HostileFixture::DissonantVoicesAssetPlay);
    let before = session.observe();
    let jacqueline = &before.play.as_ref().unwrap().players[2];
    assert!(jacqueline.threats.iter().any(|card| card.code == "01165"));
    let robes = jacqueline
        .hand
        .iter()
        .find(|card| card.code == "60412")
        .unwrap();
    let id = format!("action:ordinary:play:{}", robes.id);
    assert!(
        !before.choices.iter().any(|choice| choice.id == id),
        "Dissonant Voices prohibits playing assets, including affordable Robes"
    );
    let unchanged = session.clone();
    assert!(session.choose(before.revision, &id).is_err());
    assert_eq!(
        session, unchanged,
        "A prohibited play spends neither resources nor actions"
    );
    choose(&mut session, "action:ordinary:resource");
    let after = session.observe().play.unwrap();
    assert_eq!(after.players[2].resources, jacqueline.resources + 1);
    assert!(after.players[2].hand.iter().any(|card| card.id == robes.id));
}

#[test]
fn playing_an_asset_from_the_actual_opening_pays_once_and_preserves_custody() {
    let mut session = completed_setup();
    choose(&mut session, "turn:tony");
    let opening = session.observe().play.unwrap();
    let hard_knocks = opening.players[0]
        .hand
        .iter()
        .find(|card| card.code == "01049")
        .expect("Seed 89 opening includes Hard Knocks")
        .clone();
    let id = format!("action:ordinary:play:{}", hard_knocks.id);
    choose(&mut session, &id);
    let after = session.observe();
    let tony = &after.play.as_ref().unwrap().players[0];
    assert_eq!(
        tony.resources, 3,
        "Hard Knocks printed resource cost is 2; setup supplies 5"
    );
    assert!(!tony.hand.iter().any(|card| card.id == hard_knocks.id));
    let played = tony
        .assets
        .iter()
        .find(|asset| asset.card.id == hard_knocks.id)
        .unwrap();
    assert_eq!(played.card, hard_knocks);
    assert!(played.ready);
    assert_eq!(played.uses, 0);
    assert!(!tony.discard.iter().any(|card| card.id == hard_knocks.id));
    let original = session.clone();
    assert!(
        session.choose(after.revision, &id).is_err(),
        "The same physical hand card cannot be played again"
    );
    assert_eq!(session, original);
}

#[test]
fn a_full_ally_slot_requires_the_players_explicit_replacement_choice() {
    let mut session = completed_setup();
    choose(&mut session, "turn:jacqueline");
    let opening = session.observe().play.unwrap();
    let hand = &opening.players[2].hand;
    let olive = hand
        .iter()
        .find(|card| card.code == "04197")
        .unwrap()
        .clone();
    let katarina = hand
        .iter()
        .find(|card| card.code == "11069")
        .unwrap()
        .clone();
    choose(&mut session, &format!("action:ordinary:play:{}", olive.id));
    let after_olive = session.observe();
    let unqualified = format!("action:ordinary:play:{}", katarina.id);
    assert!(
        !after_olive
            .choices
            .iter()
            .any(|choice| choice.id == unqualified)
    );
    let original = session.clone();
    assert!(session.choose(after_olive.revision, &unqualified).is_err());
    assert_eq!(
        session, original,
        "Missing a required replacement must not spend resources/actions"
    );

    choose(
        &mut session,
        &format!("action:ordinary:play:{}:replace:{}", katarina.id, olive.id),
    );
    let after = session.observe().play.unwrap();
    let jacqueline = &after.players[2];
    assert_eq!(
        jacqueline.resources, 1,
        "Both allies have printed cost 2; setup supplies 5"
    );
    assert!(jacqueline.assets.iter().any(|asset| asset.card == katarina));
    assert!(
        !jacqueline
            .assets
            .iter()
            .any(|asset| asset.card.id == olive.id)
    );
    assert!(jacqueline.discard.iter().any(|card| card == &olive));
    assert!(
        !jacqueline
            .hand
            .iter()
            .any(|card| card.id == katarina.id || card.id == olive.id)
    );
}

#[test]
fn two_arcane_assets_fit_and_receive_their_printed_charges() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringFromSetup);
    choose(&mut session, "lead:patrice");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    choose(&mut session, "turn:patrice");
    let opening = session.observe().play.unwrap();
    let hand = &opening.players[1].hand;
    let mists = hand
        .iter()
        .find(|card| card.code == "04029")
        .unwrap()
        .clone();
    let flame = hand
        .iter()
        .find(|card| card.code == "60407")
        .unwrap()
        .clone();
    choose(&mut session, &format!("action:ordinary:play:{}", mists.id));
    choose(&mut session, &format!("action:ordinary:play:{}", flame.id));
    let after = session.observe().play.unwrap();
    let patrice = &after.players[1];
    assert_eq!(
        patrice.resources, 0,
        "Mists costs 2 and Azure Flame costs 3"
    );
    for card in [mists, flame] {
        let asset = patrice
            .assets
            .iter()
            .find(|asset| asset.card == card)
            .unwrap();
        assert_eq!(
            asset.uses, 4,
            "Both original spell identities enter with four charges"
        );
        assert!(asset.ready);
        assert_eq!((asset.damage, asset.horror), (0, 0));
    }
    assert!(
        patrice.discard.is_empty(),
        "Two arcane slots fit both spells without replacement"
    );
}

#[test]
fn asset_costs_and_limbo_precede_an_attack_of_opportunity() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
    let before = session.observe().play.unwrap();
    let mauser = before.players[0]
        .hand
        .iter()
        .find(|card| card.code == "60306")
        .unwrap()
        .clone();
    choose(&mut session, &format!("action:ordinary:play:{}", mauser.id));
    let pending = session.observe();
    let tony = &pending.play.as_ref().unwrap().players[0];
    assert_eq!(
        tony.resources, 1,
        "The original Mauser costs 4, paid before the attack"
    );
    assert!(!tony.hand.iter().any(|card| card.id == mauser.id));
    assert!(
        !tony.assets.iter().any(|asset| asset.card.id == mauser.id),
        "The paid asset has not entered play during the attack"
    );
    let attack = pending
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("opportunity:"))
        .unwrap()
        .id
        .clone();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("paid-asset.session.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(
        session.observe(),
        pending,
        "Reopening retains the paid card in limbo and the pending attack"
    );
    choose(&mut session, &attack);
    let after = session.observe().play.unwrap();
    let tony = &after.players[0];
    assert_eq!(
        (tony.damage, tony.horror, tony.resources),
        (
            before.players[0].damage + 2,
            before.players[0].horror + 2,
            1
        ),
        "The original Ghoul Priest deals 2 damage and 2 horror"
    );
    assert_eq!(
        tony.assets
            .iter()
            .find(|asset| asset.card == mauser)
            .unwrap()
            .uses,
        5
    );
    assert_eq!(after.actions_remaining[0], 2);
}
