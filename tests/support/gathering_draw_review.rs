//! Ordinary recorded prefixes shared by the native-review example and UI checks.
use crate::gathering::{DrawSource, Investigator, Session};

pub struct Checkpoint {
    pub name: &'static str,
    pub initial_view: &'static str,
    pub trigger: String,
    pub session: Session,
}

fn choose(session: &mut Session, id: &str) -> Result<(), String> {
    session
        .choose(session.observe().revision, id)
        .map_err(|error| error.to_string())
}

fn checkpoint(
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

// Follow this one ordinary first-round route, refusing any unexpected decision.
// A clone locates the decision immediately before the requested real draw; the
// exported session contains only the legal prefix, never an edited game state.
fn before_draw(
    session: &mut Session,
    source: DrawSource,
    name: &'static str,
    view: &'static str,
) -> Result<Checkpoint, String> {
    for _ in 0..48 {
        let observed = session.observe();
        let id = [
            "turn:tony",
            "turn:jacqueline",
            "turn:patrice",
            "turn:end",
            "phase-window:pass",
            "mythos:draw",
            "enemy:attacks",
            "enemy:skip-empty",
            "enemy:next-investigator",
            "enemy:attacks-complete",
            "enemy:end",
            "upkeep:ready",
            "upkeep:draw",
            "upkeep:draws-complete",
            "upkeep:patrice-refill",
            "upkeep:resources",
            "upkeep:next-hand",
            "upkeep:hand-limits-complete",
            "round:end",
            "mythos:begin",
            "mythos:encounters",
            "search:decline-hunt",
            "revelation:resolve",
        ]
        .into_iter()
        .find(|id| observed.choices.iter().any(|choice| choice.id == *id))
        .or_else(|| {
            observed
                .choices
                .iter()
                .find(|choice| choice.id.starts_with("upkeep:patrice-discard:"))
                .map(|choice| choice.id.as_str())
        })
        .ok_or_else(|| {
            format!(
                "Unexpected {name} preparation decision: {:?}",
                observed.choices
            )
        })?;
        let mut next = session.clone();
        let draws = next
            .choose_with_draws(observed.revision, id)
            .map_err(|error| error.to_string())?;
        let found = draws.iter().any(|cue| cue.batch.source == source);
        let before = found.then(|| checkpoint(name, view, id, session));
        *session = next;
        if let Some(before) = before {
            return Ok(before);
        }
    }
    Err(format!("{name} preparation exceeded 48 ordinary decisions"))
}

pub fn checkpoints() -> Result<Vec<Checkpoint>, String> {
    let mut session = Session::with_seed(89);
    let mut result = vec![checkpoint("opening", "tony", "lead:tony", &session)];
    choose(&mut session, "lead:tony")?;
    let mut mulligan = session.clone();
    for card in &session.observe().investigators[0].hand[..2] {
        choose(&mut mulligan, &format!("mulligan:{}", card.id))?;
    }
    result.push(checkpoint(
        "mulligan",
        "tony",
        "confirm-mulligan",
        &mulligan,
    ));
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan")?;
    }
    choose(&mut session, "turn:tony")?;
    result.push(checkpoint(
        "action",
        "tony",
        "action:ordinary:draw",
        &session,
    ));
    result.push(before_draw(
        &mut session,
        DrawSource::Investigator(Investigator::Tony),
        "upkeep",
        "tony",
    )?);
    result.push(before_draw(
        &mut session,
        DrawSource::Encounter,
        "encounter",
        "overview",
    )?);
    Ok(result)
}

/// Ordinary first encounter with Jacqueline leading, for the reading/reaction route.
#[cfg(test)]
pub fn encounter_checkpoint(seed: u64) -> Result<Checkpoint, String> {
    let mut session = Session::with_seed(seed);
    for id in [
        "lead:jacqueline",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
    ] {
        choose(&mut session, id)?;
    }
    before_draw(&mut session, DrawSource::Encounter, "encounter", "scenario")
}
