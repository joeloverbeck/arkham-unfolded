//! Presentation-only steps within a committed player choice.
//!
//! These views are never saved as game state. The final observation remains the
//! authority; recorded harm supplies the intermediate, non-interactive recap.

use super::{Harm, HarmRecipient, HarmSource, Observation};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationStage {
    pub view: Observation,
    pub harm: Option<Harm>,
}

/// Each recorded application is one stage, including combined damage/horror.
/// The final stage is always the committed observation. Old observations without
/// harm facts retain their aggregate presentation rather than inventing events.
pub fn stages(before: &Observation, after: &Observation) -> Vec<PresentationStage> {
    let mut stages = Vec::new();
    let mut view = before.clone();
    view.revision = after.revision;
    // Costs precede opportunity attacks. Later card draws, test results, bounty
    // payouts and choices remain concealed until the settled observation.
    view.ordinary_actions = after.ordinary_actions;
    view.bonus_action = after.bonus_action;
    view.choices.clear();
    view.can_undo = false;
    for harm in after.harm.iter().flatten() {
        match harm.source {
            HarmSource::BasicFight | HarmSource::Cultist => {
                // These consequences follow the test outcome. Keep its token
                // visible until test end, but show the recorded result now.
                view.test = after.test.clone();
            }
            HarmSource::InternalInjury => {
                // Injury is the final harm event, after the action and test
                // cleanup. Show their committed consequences now (including a
                // draw, clue or bounty), retaining only the harm already shown.
                let prior_harm = (view.damage, view.horror, view.ghoul.damage);
                let history = std::mem::take(&mut view.history);
                view = after.clone();
                (view.damage, view.horror, view.ghoul.damage) = prior_harm;
                view.history = history;
                view.choices.clear();
                view.can_undo = false;
            }
            HarmSource::OpportunityAttack | HarmSource::Tablet => {}
        }
        match harm.recipient {
            HarmRecipient::Tony => {
                view.damage += harm.damage;
                view.horror += harm.horror;
                if let Some(tony) = view.investigators.first_mut() {
                    tony.horror = view.horror;
                }
            }
            HarmRecipient::Ghoul => view.ghoul.damage += harm.damage,
        }
        view.prompt = format!("{} — {}", harm.source.label(), harm.recipient.label());
        view.harm = Some(vec![*harm]);
        // Preserve the prior explanation, without leaking a later draw/result.
        view.history.push(format!(
            "{}: {} receives {} damage and {} horror.",
            harm.source.label(),
            harm.recipient.label(),
            harm.damage,
            harm.horror
        ));
        stages.push(PresentationStage {
            view: view.clone(),
            harm: Some(*harm),
        });
    }
    stages.push(PresentationStage {
        view: after.clone(),
        harm: None,
    });
    stages
}
