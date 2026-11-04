use super::*;
use crate::gathering::{HostileFixture, Resolution, Session};

fn choose(session: &mut Session, id: &str) {
    let view = session.observe();
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
    let before = session.observe();
    assert!(!current_faces(&session).is_reverse("01107"));
    choose(&mut session, "mythos:begin");
    let after = session.observe();
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
    let steps = session.presentation_after(before.revision);
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

    assert!(current_faces(&session).is_reverse("01107"));
    assert!(!current_faces(&session).is_reverse("01110"));
    for step in steps {
        let faces = Faces::from_codes(step.revealed_story.iter().map(String::as_str));
        assert_eq!(faces.is_reverse("01107"), step.sequence >= flipped);
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("story.json");
    crate::gathering::continuity::save_session(&path, &session).unwrap();
    let reopened = crate::gathering::continuity::load_session(path).unwrap();
    assert!(current_faces(&reopened).is_reverse("01107"));
    let mut reading = Reading::for_session(&reopened);
    reading.synchronize(&current_faces(&reopened));
    assert_eq!(reading.current(), Some(StoryCard::AgendaThree));
}

#[test]
fn advancing_act_two_retains_its_reverse_without_revealing_the_next_act() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    assert!(current_faces(&session).is_reverse("01108"));
    assert!(!current_faces(&session).is_reverse("01109"));
    choose(&mut session, "round-end:advance-act");
    for who in 0..3 {
        for _ in 0..3 {
            choose(&mut session, &format!("act:contribute:{who}"));
        }
    }
    let steps = session.presentation_after(0);
    assert!(steps.iter().any(|step| step.play.act == 3));
    for step in steps {
        let faces = Faces::from_codes(step.revealed_story.iter().map(String::as_str));
        assert_eq!(faces.is_reverse("01109"), step.play.act == 3);
        assert!(!faces.is_reverse("01110"));
    }
    assert!(current_faces(&session).is_reverse("01109"));
}

fn defeat_priest(session: &mut Session) {
    choose(session, "action:ordinary:fight:400");
    for _ in 0..8 {
        let view = session.observe();
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
        assert!(!current_faces(&session).is_reverse("01110"));
        defeat_priest(&mut session);
        let steps = session.presentation_after(0);
        assert!(
            steps
                .iter()
                .any(|step| !step.revealed_story.iter().any(|code| code == "01110"))
        );
        for step in steps {
            let defeated = step
                .play
                .scenario
                .victory
                .iter()
                .any(|card| card.code == "01116");
            assert_eq!(
                step.revealed_story.iter().any(|code| code == "01110"),
                defeated
            );
        }
        assert!(current_faces(&session).is_reverse("01110"));
        choose(&mut session, ending);
        assert!(current_faces(&session).is_reverse("01110"));
    }
}

#[test]
fn zero_doom_without_an_advancement_never_reveals_the_current_agenda() {
    let session = Session::with_hostile_fixture(HostileFixture::GatheringGhoulMovement);
    let view = session.observe();
    assert_eq!(view.play.as_ref().unwrap().agenda_doom, 0);
    let faces = current_faces(&session);
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
    let view = session.observe();
    assert!(session.presentation_after(0).is_empty());
    assert!(
        view.choices
            .iter()
            .any(|choice| choice.id == "agenda:search")
    );
    let faces = current_faces(&session);
    assert!(faces.is_reverse("01106"));
    assert!(!faces.is_reverse("01107"));
}

fn current_faces(session: &Session) -> Faces {
    Faces::from_codes(session.live_state().known_story.iter().map(String::as_str))
}
