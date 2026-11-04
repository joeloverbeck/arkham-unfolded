//! Paid boosts in the two player windows of the current skill test.
use super::{
    CardView, Investigator,
    runtime::{AssetState, Frame, Game, PendingRevelation, Skill, TestOperation, Window},
};
use crate::{Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct BoostState {
    masks_used: Vec<u16>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoostFixture {
    HardKnocks,
    SourMash,
    MaskRetry,
    MaskRazor,
}

pub(super) fn choices(game: &Game) -> Vec<Choice> {
    let Some(test) = &game.test else {
        return vec![];
    };
    if !matches!(test.window, Window::BeforeCommitment | Window::BeforeReveal) {
        return vec![];
    }
    let player = &game.players[test.tester.index()];
    if player.defeated || player.resigned {
        return vec![];
    }
    player.assets.iter().filter_map(|asset|{
        let label=match asset.card.code.as_str(){
            "01049" if player.resources>0 && matches!(test.skill,Skill::Combat|Skill::Agility)=>"Spend 1 resource on Hard Knocks for +1 skill",
            "10111" if asset.uses>0 && !test.boosts.masks_used.contains(&asset.card.id) && (matches!(test.skill,Skill::Willpower|Skill::Agility) || test.source_card.as_ref().is_some_and(|card|matches!(card.code.as_str(),"06117"|"06201"|"06164")))=>"Spend 1 Sparrow Mask offering for +2 skill (once this test)",
            "05117" if asset.ready && asset.uses>0 && test.skill==Skill::Willpower && matches!(test.operation,TestOperation::RottingRemains{..}|TestOperation::DiscardThreat{..}|TestOperation::CryptChill{..})=>"Exhaust Tennessee Sour Mash and spend 1 supply for +2 willpower on this treachery test",
            _=>return None,
        };
        Some(Choice{id:format!("boost:{}",asset.card.id),label:label.into(),cancel_faces:vec![]})
    }).collect()
}
pub(super) fn choose(game: &mut Game, id: &str) -> Result<(), RuleError> {
    if !choices(game).iter().any(|choice| choice.id == id) {
        return Err(RuleError(
            "That test boost is not currently available.".into(),
        ));
    }
    let id: u16 = id
        .strip_prefix("boost:")
        .and_then(|id| id.parse().ok())
        .ok_or_else(|| RuleError("Invalid boost identity.".into()))?;
    let test = game.test.as_mut().expect("Paid boost test");
    let player = &mut game.players[test.tester.index()];
    let asset = player
        .assets
        .iter_mut()
        .find(|asset| asset.card.id == id)
        .expect("Offered controlled asset");
    let previous_bonus = test.bonus;
    match asset.card.code.as_str() {
        "01049" => {
            player.resources -= 1;
            test.bonus += 1;
        }
        "10111" => {
            asset.uses -= 1;
            test.boosts.masks_used.push(id);
            test.bonus += 2;
        }
        "05117" => {
            asset.ready = false;
            asset.uses -= 1;
            test.bonus += 2;
        }
        _ => unreachable!("Offered boost card"),
    }
    test.record_bonus(&asset.card.title, test.tester, test.bonus - previous_bonus);
    game.history.push(format!(
        "{} pays {}'s ability costs for this skill test only. [{}]",
        test.tester.name(),
        asset.card.title,
        asset.card.code
    ));
    Ok(())
}

fn take(game: &mut Game, who: Investigator, code: &str) -> CardView {
    let player = &mut game.players[who.index()];
    if let Some(index) = player.hand.iter().position(|card| card.code == code) {
        player.hand.remove(index)
    } else {
        player.deck.remove(
            player
                .deck
                .iter()
                .position(|card| card.code == code)
                .expect("Selected boost card"),
        )
    }
}
pub(super) fn configure_fixture(game: &mut Game, case: BoostFixture) {
    if case == BoostFixture::MaskRazor {
        super::spell_events::configure_fixture(
            game,
            super::spell_events::SpellEventFixture::RazorWatcher,
        );
        let card = take(game, Investigator::Patrice, "10111");
        game.players[1].assets.push(AssetState {
            card,
            ready: true,
            uses: 2,
            damage: 0,
            horror: 0,
        });
        game.set_fixture_tokens(vec![TokenFace::MinusTwo]);
        return;
    }
    if case == BoostFixture::MaskRetry {
        super::retry::configure_fixture(game, super::retry::RetryFixture::ChargedFight);
        let card = take(game, Investigator::Patrice, "10111");
        game.players[1].assets.push(AssetState {
            card,
            ready: true,
            uses: 1,
            damage: 0,
            horror: 0,
        });
        game.set_fixture_tokens(vec![TokenFace::MinusFour, TokenFace::MinusOne]);
        return;
    }
    for frame in game.frames.drain(..) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        }
    }
    game.players[0].location = "01112".into();
    game.players[1].location = "01114".into();
    game.players[2].location = "01113".into();
    let code = if case == BoostFixture::HardKnocks {
        "01049"
    } else {
        "05117"
    };
    let card = take(game, Investigator::Tony, code);
    game.players[0].assets.push(AssetState {
        card,
        ready: true,
        uses: if case == BoostFixture::SourMash { 2 } else { 0 },
        damage: 0,
        horror: 0,
    });
    game.set_fixture_tokens(vec![if case == BoostFixture::HardKnocks {
        TokenFace::MinusTwo
    } else {
        TokenFace::MinusOne
    }]);
    if case == BoostFixture::HardKnocks {
        let id = super::scenario::advance_act_two(game).expect("Priest");
        let enemy = game
            .scenario
            .enemies
            .iter_mut()
            .find(|enemy| enemy.card.id == id)
            .unwrap();
        enemy.location = "01112".into();
        enemy.engaged = Some(Investigator::Tony);
        game.enable_fixture_turn(Investigator::Tony);
    } else {
        let index = game
            .encounter_deck
            .iter()
            .position(|card| card.code == "01163")
            .unwrap();
        let card = game.encounter_deck.remove(index);
        let encounter = card.id;
        game.frames.push(Frame::Revelation(PendingRevelation {
            card,
            drawn_by: Investigator::Tony,
            started: true,
            cancelled: false,
        }));
        game.begin_test(
            TestOperation::RottingRemains { encounter },
            Investigator::Tony,
            Skill::Willpower,
            3,
        )
        .expect("One treachery test");
    }
    game.history = vec![format!(
        "Privileged {case:?} paid-boost witness using selected assets and original rules. [01049/05117/10111]"
    )];
}
