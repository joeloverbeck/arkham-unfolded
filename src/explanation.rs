//! Recorded explanation for the scripted Rotting Remains experiment only.

use crate::TokenFace;
#[cfg(any(feature = "tabletop", test))]
use crate::{Observation, Stage, TokenStatus};

pub(crate) fn revealed_history(faces: impl Iterator<Item = TokenFace>) -> String {
    format!(
        "Revealed {}. [RR ST.3; 60401]",
        faces.map(TokenFace::label).collect::<Vec<_>>().join(", ")
    )
}

pub(crate) fn resolved_curse_history() -> &'static str {
    "The resolved curse contributes −2 and requires another reveal. The prior token group stays resolved. [Innsmouth insert p1]"
}

#[cfg(any(feature = "tabletop", test))]
pub(crate) struct ResultExplanation<'a> {
    pub resolved: Vec<TokenFace>,
    pub auto_fail_reason: Option<&'static str>,
    pub curse_reason: Option<&'static str>,
    pub additional_horror: u8,
    pub jacqueline_horror: Option<HorrorChange>,
    pub continuation: Vec<&'a str>,
}

#[cfg(any(feature = "tabletop", test))]
pub(crate) struct HorrorChange {
    pub previous: u8,
    pub total: u8,
    pub sanity: u8,
}

#[cfg(any(feature = "tabletop", test))]
impl ResultExplanation<'_> {
    pub fn failed(&self) -> bool {
        // Only Rotting Remains: this is not a general skill-test outcome model.
        self.additional_horror > 0
    }
}

#[cfg(any(feature = "tabletop", test))]
pub(crate) fn explain_result(view: &Observation) -> Option<ResultExplanation<'_>> {
    if view.stage != Stage::Finished {
        return None;
    }
    let resolved: Vec<_> = view
        .tokens
        .iter()
        .filter(|token| token.status == TokenStatus::Resolved)
        .map(|token| token.face)
        .collect();
    let auto_fail = resolved.contains(&TokenFace::AutoFail);
    let curse = resolved.contains(&TokenFace::Curse);
    let mut continuation = Vec::new();
    // Historical recognition stays explicit and separate from the current
    // formatter: changing new wording must not silently rewrite this contract.
    // These are the existing authored prefixes and citation delimiter, not a
    // natural-language parser. Unknown history contributes no continuation.
    if curse
        && let Some(index) = view
            .history
            .iter()
            .position(|entry| entry.starts_with("The resolved curse"))
    {
        for entry in view
            .history
            .iter()
            .skip(index + 1)
            .filter(|entry| entry.starts_with("Revealed "))
        {
            continuation.push(entry.split(" [").next().unwrap_or(entry));
        }
    }
    Some(ResultExplanation {
        resolved,
        auto_fail_reason: auto_fail.then_some("The uncanceled auto-fail makes the test fail with a final skill value of 0. Numerical modifiers do not change that result."),
        curse_reason: curse.then_some("The Curse was not canceled: it applies -2 AND requires another reveal. The additional token is separate from that -2 modifier. This experiment uses a prepared token order."),
        additional_horror: view.horror_dealt,
        jacqueline_horror: view.investigators.iter().find(|i| i.name == "Jacqueline Fine").map(|i| HorrorChange {
            previous: i.horror.saturating_sub(view.horror_dealt),
            total: i.horror,
            sanity: i.sanity,
        }),
        continuation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuity::{self};
    use crate::{Fixture, Session};

    #[test]
    fn unfinished_and_ended_observations_have_no_result_explanation() {
        let mut view = Session::new(Fixture::curse()).observe();
        for stage in [
            Stage::Commit,
            Stage::BeforeReveal,
            Stage::Cancellation,
            Stage::Ended,
        ] {
            view.stage = stage;
            assert!(explain_result(&view).is_none());
        }
    }

    #[test]
    fn missing_or_unrecognized_history_does_not_invent_a_continuation() {
        let mut session = Session::new(Fixture::curse());
        for id in ["commit-promise", "use-jacqueline", "cancel-1-2"] {
            session.choose(session.observe().revision, id).unwrap();
        }
        let mut view = session.observe();
        for history in [
            vec![],
            vec!["Unknown future history format."],
            vec!["Revealed −2. [RR ST.3; 60401]"],
            vec![
                "The resolved curse contributes −2 and requires another reveal. The prior token group stays resolved. [Innsmouth insert p1]",
                "Unknown future reveal format.",
            ],
        ] {
            view.history = history.into_iter().map(str::to_owned).collect();
            let before = view.clone();
            let explanation = explain_result(&view).unwrap();
            assert!(
                explanation.curse_reason.is_some(),
                "the recorded resolved token still has its explanation"
            );
            assert!(explanation.continuation.is_empty());
            assert_eq!(view, before, "raw history remains available and unchanged");
        }
    }

    #[test]
    fn resolved_effects_explain_horror_without_counting_canceled_tokens() {
        for (fixture, cancel, resolved, additional, auto_fail, curse) in [
            (
                Fixture::auto_fail(),
                "cancel-1-2",
                vec![TokenFace::AutoFail],
                3,
                true,
                false,
            ),
            (
                Fixture::auto_fail(),
                "cancel-0",
                vec![TokenFace::Curse, TokenFace::Zero, TokenFace::MinusTwo],
                0,
                false,
                true,
            ),
            (
                Fixture::curse(),
                "cancel-0-1",
                vec![TokenFace::Zero],
                0,
                false,
                false,
            ),
        ] {
            let mut session = Session::new(fixture);
            for id in ["commit-promise", "use-jacqueline", cancel] {
                session.choose(session.observe().revision, id).unwrap();
            }
            let view = session.observe();
            let explanation = explain_result(&view).unwrap();
            assert_eq!(explanation.resolved, resolved);
            assert_eq!(explanation.additional_horror, additional);
            assert_eq!(explanation.failed(), auto_fail);
            assert_eq!(explanation.auto_fail_reason.is_some(), auto_fail);
            assert_eq!(explanation.curse_reason.is_some(), curse);
            let horror = explanation.jacqueline_horror.unwrap();
            assert_eq!(horror.previous, 2);
            assert_eq!(horror.total, if auto_fail { 5 } else { 2 });
            assert_eq!(horror.sanity, 9);
            if auto_fail {
                assert_eq!(view.skill_value, 0);
                assert_eq!(
                    explanation.auto_fail_reason,
                    Some(
                        "The uncanceled auto-fail makes the test fail with a final skill value of 0. Numerical modifiers do not change that result."
                    )
                );
            }
            if curse {
                assert_eq!(
                    explanation.curse_reason,
                    Some(
                        "The Curse was not canceled: it applies -2 AND requires another reveal. The additional token is separate from that -2 modifier. This experiment uses a prepared token order."
                    )
                );
                assert_eq!(explanation.continuation, ["Revealed −2."]);
            } else {
                assert!(explanation.continuation.is_empty());
            }
        }
    }

    #[test]
    fn recorded_continuations_preserve_the_two_reveal_groupings() {
        for (_name, choices, expected) in [
            (
                "early",
                ["commit-promise", "use-jacqueline", "cancel-1-2"].as_slice(),
                "Revealed −2.",
            ),
            (
                "late",
                ["commit-promise", "reveal", "use-jacqueline", "cancel-1-2"].as_slice(),
                "Revealed −1, 0, −2.",
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("live.json");
            let mut live = Session::new(Fixture::curse());
            for &id in choices {
                let revision = live.observe().revision;
                continuity::commit_choice(&path, &mut live, revision, id).unwrap();
            }
            let reopened = continuity::load_session(&path).unwrap();
            for view in [live.observe(), reopened.observe()] {
                let explanation = explain_result(&view).unwrap();
                assert_eq!(explanation.continuation, [expected]);
                assert_eq!(explanation.additional_horror, 0);
                assert_eq!(view.skill_value, 5);
                assert_eq!(view.difficulty, 3);
                assert!(
                    view.history
                        .iter()
                        .any(|entry| entry == "Revealed −2. [RR ST.3; 60401]"
                            || entry == "Revealed −1, 0, −2. [RR ST.3; 60401]")
                );
            }
        }
    }
}
