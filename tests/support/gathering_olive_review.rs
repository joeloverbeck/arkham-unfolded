//! Ordinary decisions reproducing the reported Olive/Jacqueline test.
//! Built from a fixed seed; no personal save loading or privileged state edits.
use crate::gathering::Session;

pub fn checkpoints() -> Vec<(&'static str, Session)> {
    let mut session = Session::with_seed(10_785_006_410_663_402_492);
    for id in [
        "lead:tony",
        "mulligan:13",
        "confirm-mulligan",
        "mulligan:134",
        "mulligan:143",
        "confirm-mulligan",
        "confirm-mulligan",
        "turn:jacqueline",
        // Uncage's only eligible Spell is played and the event finishes by itself.
        "action:ordinary:play:tony:203:03033",
        "action:ordinary:play:209",
        "action:ordinary:investigate:spell:227",
        "window:pass",
        "commit:confirm",
        "window:pass",
    ] {
        session
            .choose(session.observe().revision, id)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    let mut result = vec![("olive-ready", session.clone())];
    for (id, name) in [
        ("tokens:olive:0:209", "olive-active"),
        ("tokens:jacqueline:at:0", "olive-jacqueline-plan"),
        ("test:reveal", "olive-five-tokens"),
        ("tokens:cancel:1", "olive-one-selected"),
        ("tokens:cancel:3", "olive-two-selected"),
        ("tokens:confirm", "olive-choose-two"),
        ("tokens:keep:0", "olive-first-kept"),
        ("tokens:keep:2", "olive-two-kept"),
        ("tokens:confirm", "olive-result"),
    ] {
        session.choose(session.observe().revision, id).unwrap();
        result.push((name, session.clone()));
    }
    let mut grouped = result[1].1.clone();
    for id in [
        "tokens:jacqueline",
        "test:reveal",
        "tokens:keep:0",
        "tokens:keep:1",
        "tokens:confirm",
    ] {
        grouped.choose(grouped.observe().revision, id).unwrap();
    }
    result.push(("olive-group-cancellation", grouped));
    result
}
