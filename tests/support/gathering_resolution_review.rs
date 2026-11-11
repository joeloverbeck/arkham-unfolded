//! Fixed ordinary prefixes for the resolution surfaces in docs/tabletop-presentation.md.
//! No fixture constructors, state edits, or seed search run in this helper.
use crate::gathering::Session;

pub struct Checkpoint {
    pub name: &'static str,
    pub initial_view: &'static str,
    pub trigger: String,
    pub session: Session,
}

fn choose(session: &mut Session, id: &str) -> Result<(), String> {
    session
        .choose(session.observe().revision, id)
        .map_err(|error| format!("{id}: {error}"))
}

fn start(seed: u64, turn: &str) -> Result<Session, String> {
    let mut session = Session::with_seed(seed);
    for id in [
        "lead:tony",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
        turn,
    ] {
        choose(&mut session, id)?;
    }
    Ok(session)
}

fn point(
    name: &'static str,
    initial_view: &'static str,
    trigger: &str,
    session: &Session,
) -> Checkpoint {
    Checkpoint {
        name,
        initial_view,
        trigger: trigger.into(),
        session: session.clone(),
    }
}

pub fn checkpoints() -> Result<Vec<Checkpoint>, String> {
    let mut result = vec![];
    let mut hornet = start(0, "turn:tony")?;
    result.push(point(
        "hornet",
        "tony",
        "action:ordinary:play:tony:30",
        &hornet,
    ));
    choose(&mut hornet, "action:ordinary:play:tony:30")?;
    result.push(point(
        "hornet-pending",
        "tony",
        "search:enemy:318:then",
        &hornet,
    ));

    let no_match = start(405, "turn:tony")?;
    let view = no_match.observe();
    let event = view.investigators[0]
        .hand
        .iter()
        .find(|card| card.code == "09068")
        .ok_or("Seed 405 must offer Kicking the Hornet's Nest")?;
    // Without an eligible enemy, the play's search finishes by itself.
    let play = format!("action:ordinary:play:tony:{}", event.id);
    result.push(point("no-match", "tony", &play, &no_match));

    let mut backpack = start(5, "turn:tony")?;
    choose(&mut backpack, "action:ordinary:play:11")?;
    result.push(point("backpack", "tony", "backpack:search", &backpack));

    let mut voice = start(3, "turn:jacqueline")?;
    choose(&mut voice, "action:ordinary:play:spell:232")?;
    result.push(point("voice-of-ra", "jacqueline", "test:reveal", &voice));

    let mut premonition = start(2, "turn:jacqueline")?;
    choose(&mut premonition, "premonition:210")?;
    result.push(point(
        "premonition",
        "jacqueline",
        "test:reveal",
        &premonition,
    ));

    let mut violin = start(1, "turn:patrice")?;
    choose(&mut violin, "action:ordinary:play:127")?;
    result.push(point(
        "exhaust-asset",
        "patrice",
        "violin:127:discard:141:target:patrice:resource",
        &violin,
    ));

    for id in [
        "search:enemy:318:then",
        "bounty:318:0",
        "action:ordinary:evade:318",
        "commit:7",
        "tony:watch:7:0",
        "commit:118",
        "commit:201",
        "commit:confirm",
        "test:reveal",
    ] {
        choose(&mut hornet, id)?;
        if id == "commit:7" {
            result.push(point("watch-wager", "test", "tony:watch:7:0", &hornet));
            let mut jacqueline = hornet.clone();
            for choice in [
                "tony:watch:7:0",
                "commit:confirm",
                "tokens:jacqueline",
                "test:reveal",
            ] {
                choose(&mut jacqueline, choice)?;
            }
            let trigger = jacqueline
                .observe()
                .choices
                .iter()
                .find(|choice| choice.id.starts_with("tokens:cancel:"))
                .ok_or("Jacqueline must offer cancellation")?
                .id
                .clone();
            result.push(point("jacqueline-three", "test", &trigger, &jacqueline));
        }
    }
    // Reopening retains the completed result reading before the owner examines
    // the evaded enemy; acknowledging it cannot repeat the evade.
    let evaded = hornet.observe().play.ok_or("Expected scenario play")?;
    if !evaded
        .scenario
        .enemies
        .iter()
        .any(|enemy| enemy.card.id == 318 && enemy.exhausted)
    {
        return Err("The recorded ordinary evasion must exhaust its enemy".into());
    }
    result.push(point("evaded-enemy", "scenario", "turn:end", &hornet));

    let mut keyring = start(0, "turn:patrice")?;
    choose(&mut keyring, "action:ordinary:play:141")?;
    result.push(point(
        "keyring-two",
        "patrice",
        "action:ordinary:investigate:spell:141",
        &keyring,
    ));

    // Only genuine decisions remain in this route: the session performs the
    // last turn's start, the round's administration and the mythos draws itself.
    let mut revelation = Session::with_seed(0);
    for id in [
        "lead:tony",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
        "turn:tony",
        "turn:end",
        "turn:patrice",
        "turn:end",
        "turn:end",
        "upkeep:patrice-discard:124",
        "upkeep:patrice-discard:143",
        "upkeep:patrice-discard:118",
    ] {
        if id == "upkeep:patrice-discard:124" {
            result.push(point("patrice-upkeep", "patrice", id, &revelation));
        }
        choose(&mut revelation, id)?;
    }
    // Placing the fourth card leaves the last one forced; the refill, the round
    // end and the mythos draws then follow within this decision.
    result.push(point(
        "revelation",
        "patrice",
        "upkeep:patrice-discard:123",
        &revelation,
    ));
    choose(&mut revelation, "upkeep:patrice-discard:123")?;
    let view = revelation.observe();
    if !view
        .play
        .as_ref()
        .is_some_and(|play| play.test.is_some() && play.pending_revelation.is_some())
    {
        return Err("The ordinary encounter must start its revelation test".into());
    }
    result.push(point(
        "revelation-pending",
        "test",
        "commit:confirm",
        &revelation,
    ));
    Ok(result)
}
