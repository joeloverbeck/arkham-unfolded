use super::*;
use crate::gathering::{HostileFixture, Resolution, Session, continuity::Replay};

fn choose(session: &mut Session, id: &str) {
    let view = session.observe_for_play();
    assert!(
        view.choices.iter().any(|choice| choice.id == id),
        "Missing {id}: {:?}",
        view.choices
    );
    session.choose(view.revision, id).unwrap();
}

fn paid_act_two() -> Session {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    choose(&mut session, "round-end:advance-act");
    for investigator in 0..3 {
        for _ in 0..3 {
            choose(&mut session, &format!("act:contribute:{investigator}"));
        }
    }
    choose(&mut session, "bounty:400:0");
    choose(&mut session, "round-end:ghoul-doom");
    session
}

#[test]
fn agenda_three_no_resolution_reveals_only_at_its_public_doom_removal() {
    let mut session = paid_act_two();
    let before = session.observe_for_play();
    choose(&mut session, "mythos:begin");
    let after = session.observe_for_play();
    assert_eq!(
        after
            .play
            .as_ref()
            .unwrap()
            .scenario
            .outcome
            .as_ref()
            .unwrap()
            .resolution,
        Resolution::NoResolution
    );
    let recording = session.recording();
    let steps: Vec<_> = recording
        .presentation_moments
        .iter()
        .filter(|step| step.revision == after.revision)
        .collect();
    let flipped = steps
        .windows(2)
        .find_map(|pair| {
            (pair[0].play.agenda == 3
                && pair[0].play.agenda_doom > 0
                && pair[1].play.agenda == 3
                && pair[1].play.agenda_doom == 0)
                .then_some(pair[1].sequence)
        })
        .expect("Actual advancement records doom before and after its removal");
    assert!(!Faces::for_view(&recording, &before, None).is_reverse("01107"));
    assert!(Faces::for_view(&recording, &after, None).is_reverse("01107"));
    assert!(
        !Faces::for_view(&recording, &after, None).is_reverse("01110"),
        "Defeat does not reveal Act 3's successful ending"
    );

    let mut replay = Replay::new(recording).unwrap();
    let mut checked_before = false;
    let mut checked_after = false;
    loop {
        let view = replay.current();
        let sequence = replay.current_presentation_sequence();
        let faces = Faces::for_view(replay.recording(), view, sequence);
        if view.revision == after.revision {
            let expected = sequence.is_none_or(|sequence| sequence >= flipped);
            assert_eq!(faces.is_reverse("01107"), expected, "sequence {sequence:?}");
            checked_before |= !expected;
            checked_after |= expected;
        } else {
            assert!(
                !faces.is_reverse("01107"),
                "Future presentation cannot leak into an earlier revision"
            );
        }
        if !replay.next() {
            break;
        }
    }
    assert!(checked_before && checked_after);
}

#[test]
fn advancing_act_two_retains_its_reverse_without_revealing_the_next_act() {
    let session = paid_act_two();
    let recording = session.recording();
    let advanced = recording
        .presentation_moments
        .iter()
        .find(|step| step.play.act == 3)
        .expect("Act 2's paid advancement records the revealed Parlor and Priest");
    let boundary = (advanced.revision, advanced.sequence);
    let mut replay = Replay::new(recording).unwrap();
    let mut saw_before = false;
    let mut saw_after = false;
    loop {
        let view = replay.current();
        let sequence = replay.current_presentation_sequence();
        let faces = Faces::for_view(replay.recording(), view, sequence);
        let expected = (view.revision, sequence.unwrap_or(u32::MAX)) >= boundary;
        assert_eq!(faces.is_reverse("01109"), expected);
        assert!(!faces.is_reverse("01110"));
        assert!(
            faces.is_reverse("01108"),
            "The already completed first Act remains known"
        );
        assert_eq!(
            faces
                .known_reverses(ScenarioBookSide::Act)
                .map(StoryCard::code)
                .collect::<Vec<_>>(),
            if expected {
                vec!["01108", "01109"]
            } else {
                vec!["01108"]
            }
        );
        saw_before |= !expected;
        saw_after |= expected;
        if !replay.next() {
            break;
        }
    }
    assert!(saw_before && saw_after);
}

fn defeat_priest(session: &mut Session) {
    choose(session, "action:ordinary:fight:400");
    for _ in 0..8 {
        let view = session.observe_for_play();
        if view
            .choices
            .iter()
            .any(|choice| choice.id == "resolution:burn")
        {
            return;
        }
        let id = [
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "lucky:decline",
        ]
        .into_iter()
        .find(|id| view.choices.iter().any(|choice| choice.id == *id))
        .expect("The basic fight can reach its revealed result");
        choose(session, id);
    }
    panic!("Priest defeat did not offer the Act 3 ending");
}

#[test]
fn act_three_reverse_stays_hidden_before_the_recorded_priest_defeat() {
    for ending in ["resolution:burn", "resolution:refuse"] {
        let mut session = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
        defeat_priest(&mut session);
        let choice_revision = session.observe_for_play().revision;
        choose(&mut session, ending);
        let recording = session.recording();
        let defeated = recording
            .presentation_moments
            .iter()
            .find(|step| {
                step.play
                    .scenario
                    .victory
                    .iter()
                    .any(|card| card.code == "01116")
            })
            .expect("The Priest's defeat has a visible victory-display moment");
        assert_eq!(defeated.revision, choice_revision);
        let boundary = (defeated.revision, defeated.sequence);
        let mut replay = Replay::new(recording).unwrap();
        let mut saw_same_revision_before = false;
        let mut saw_after = false;
        loop {
            let view = replay.current();
            let sequence = replay.current_presentation_sequence();
            let expected = (view.revision, sequence.unwrap_or(u32::MAX)) >= boundary;
            let faces = Faces::for_view(replay.recording(), view, sequence);
            assert_eq!(
                faces.is_reverse("01110"),
                expected,
                "{ending}: revision{} sequence{sequence:?}",
                view.revision
            );
            saw_same_revision_before |= view.revision == choice_revision && !expected;
            saw_after |= expected;
            if !replay.next() {
                break;
            }
        }
        assert!(
            saw_same_revision_before,
            "The final decision in this revision already offers the ending; earlier presentations must not borrow it"
        );
        assert!(saw_after);
    }
}

#[test]
fn zero_doom_without_an_advancement_never_reveals_the_current_agenda() {
    let session = Session::with_hostile_fixture(HostileFixture::GatheringGhoulMovement);
    let view = session.observe_for_play();
    assert_eq!(view.play.as_ref().unwrap().agenda_doom, 0);
    let faces = Faces::for_view(&session.recording(), &view, None);
    assert_eq!(
        faces
            .known_reverses(ScenarioBookSide::Agenda)
            .map(StoryCard::code)
            .collect::<Vec<_>>(),
        ["01105", "01106"]
    );
    assert!(!faces.is_reverse("01107"));
    assert!(!faces.is_reverse("01111"));
    assert!(StoryCard::from_code("01104").is_none());
    assert!(StoryCard::from_code("card-01105-back").is_none());
}

#[test]
fn an_explicit_agenda_reverse_choice_is_known_without_a_prior_transient() {
    let session = Session::with_hostile_fixture(HostileFixture::GatheringAgendaTwoNoGhoul);
    let view = session.observe_for_play();
    let recording = session.recording();
    assert!(recording.presentation_moments.is_empty());
    assert!(
        view.choices
            .iter()
            .any(|choice| choice.id == "agenda:search")
    );
    let faces = Faces::for_view(&recording, &view, None);
    assert!(faces.is_reverse("01106"));
    assert!(!faces.is_reverse("01107"));
}
