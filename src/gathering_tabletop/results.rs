//! Reading a recorded result never runs a rules decision. Optional effects use
//! the ordinary revision-bound choices, after the result has been acknowledged.
use super::*;
use crate::gathering::{Session, TestResult, TestView};
use crate::tabletop_ui as screen;

#[derive(Default)]
pub(super) struct Reading {
    acknowledged: usize,
    pending: Option<TestResult>,
    pending_from_test: bool,
    awaiting_cleanup: Option<Investigator>,
}

impl Reading {
    pub(super) fn for_session(session: &Session) -> Self {
        let mut reading = Self {
            acknowledged: session.live_state().completed_before,
            ..default()
        };
        if let Some(play) = session.observe().play
            && play.test.is_none()
            && play.pending.discard.is_some()
            && let Some(report) = session.pending_test_view().as_ref().and_then(result)
        {
            reading.acknowledged = play.completed_tests.len();
            reading.pending = Some(report);
            reading.pending_from_test = true;
        }
        reading
    }

    pub(super) fn at(view: Option<&Observation>) -> Self {
        Self {
            acknowledged: view
                .and_then(|view| view.play.as_ref())
                .map_or(0, |play| play.completed_tests.len()),
            ..default()
        }
    }

    pub(super) fn synchronize(&mut self, view: &Observation) {
        if self.pending.is_some() {
            return;
        }
        let Some(play) = &view.play else {
            return;
        };
        if let Some(who) = self.awaiting_cleanup {
            let stored = play.completed_tests.get(self.acknowledged - 1);
            if stored.is_some_and(|result| result.tester == who) {
                self.awaiting_cleanup = None;
            } else if (play.players[who.index()].defeated
                || play.players[who.index()].resigned
                || play.scenario.outcome.is_some())
                && play.test.as_ref().is_none_or(|test| test.tester != who)
            {
                // Lethal ST7 consequences can abandon cleanup. Ordinary
                // committed-card discard ordering can also hide the active
                // test, so disappearance alone is not abandonment evidence.
                self.acknowledged -= 1;
                self.awaiting_cleanup = None;
            }
        }
        self.pending = play.completed_tests.get(self.acknowledged).cloned();
        self.pending_from_test = false;
        if self.pending.is_none() && self.acknowledged == play.completed_tests.len() {
            self.pending = play
                .test
                .as_ref()
                .filter(|test| test.resolved)
                .and_then(result);
            self.pending_from_test = self.pending.is_some();
        }
    }

    pub(super) fn current(&self) -> Option<&TestResult> {
        self.pending.as_ref()
    }

    pub(super) fn advance(&mut self) {
        if let Some(report) = self.pending.take() {
            self.acknowledged += 1;
            self.awaiting_cleanup = self.pending_from_test.then_some(report.tester);
        }
    }
}

fn result(test: &TestView) -> Option<TestResult> {
    Some(TestResult {
        tester: test.tester,
        source: test.source_card.as_ref().map_or_else(
            || format!("{:?} test", test.skill),
            |card| card.title.clone(),
        ),
        total: test.total?,
        difficulty: test.difficulty,
        success: test.would_succeed?,
        failure_margin: test.failure_margin?,
        breakdown: test.breakdown.clone(),
    })
}

/// These windows have an explicit rules-level decline. Other choices (such as
/// assigning damage) remain in their existing consequence flow.
pub(super) fn opportunities(view: &Observation) -> bool {
    view.choices.iter().any(|choice| {
        is_decline(choice)
            // Small Favor can also defeat an enemy, without a skill test.
            // Keep that existing non-test decision out of the result panel.
            && (choice.id != "defeat:done"
                || view.play.as_ref().is_some_and(|play| play.test.is_some()))
    })
}

fn opportunity_label(choice: &crate::Choice, view: &Observation) -> String {
    if choice.id == "defeat:done" {
        return if view
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("defeat:bleeds:"))
        {
            "Decline If it bleeds...".into()
        } else {
            "Finish enemy-defeat reactions".into()
        };
    }
    if let Some(location) = choice
        .id
        .strip_prefix("spell-move:")
        .filter(|value| *value != "decline")
    {
        return format!(
            "Move to {} (Mists of R'lyeh)",
            play::location_name(location)
        );
    }
    if let Some(owner) = choice.id.strip_prefix("tony:quick-thinking:") {
        let name = match owner {
            "tony" => "Tony",
            "patrice" => "Patrice",
            "jacqueline" => "Jacqueline",
            _ => owner,
        };
        return format!("Take {name}'s extra action (Quick Thinking)");
    }
    if matches!(choice.id.as_str(), "after-test:pass" | "success:pass") {
        let uses: Vec<_> = view
            .choices
            .iter()
            .filter(|other| other.id != choice.id && other.id != "undo")
            .collect();
        if uses.len() == 1 && uses[0].id.starts_with("tony:quick-thinking:") {
            return "Decline Quick Thinking".into();
        }
        return "Decline all listed effects".into();
    }
    controls::display_history(view, &choice.label)
}

pub(super) fn render(
    commands: &mut Commands,
    table: &Table,
    view: &Observation,
    art: &art::Art,
    size: Vec2,
) {
    let pending = table.results.current();
    let current = view.play.as_ref().and_then(|play| play.test.as_ref());
    let provisional = current.is_some_and(|test| !test.resolved);
    let report = pending
        .cloned()
        .or_else(|| current.and_then(result))
        .or_else(|| view.play.as_ref()?.completed_tests.last().cloned());
    let heading = report.as_ref().map_or_else(
        || "Test opportunities".to_owned(),
        |report| {
            if provisional && pending.is_none() {
                format!("Provisional result · {}", outcome(report))
            } else {
                outcome(report)
            }
        },
    );
    screen::story_reading(
        commands,
        Panel,
        ScrollablePanel { inspection: true },
        size,
        false,
        "Skill test result",
        |panel| {
            panel.spawn(art.text(&heading, 28., GOLD));
            if let Some(report) = &report {
                panel.spawn(art.text(
                    format!(
                        "{} · {}\nTotal {}  /  Difficulty {}",
                        report.tester.name(),
                        report.source,
                        report.total,
                        report.difficulty
                    ),
                    20.,
                    PAPER,
                ));
            }
        },
        |_, _| {},
        |body| {
            if let Some(report) = &report {
                details(body, art, report);
            }
        },
        |footer| {
            if pending.is_none() {
                footer.spawn(art.text(
                    "Use an available effect, or explicitly decline.",
                    18.,
                    GOLD,
                ));
                footer
                    .spawn((
                        Node {
                            max_height: px(size.y * 0.28),
                            overflow: Overflow::scroll_y(),
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 0.,
                            padding: UiRect::all(px(6)),
                            row_gap: px(8),
                            ..default()
                        },
                        ScrollPosition::default(),
                        ScrollablePanel { inspection: true },
                    ))
                    .observe(screen::scroll_panel)
                    .with_children(|options| {
                        for (index, choice) in view
                            .choices
                            .iter()
                            .filter(|choice| choice.id != "undo" && !is_decline(choice))
                            .enumerate()
                        {
                            let button = ui::button(
                                art,
                                options,
                                &opportunity_label(choice, view),
                                Action::ResultChoice(table.epoch, view.revision, choice.id.clone()),
                            );
                            if index == 0 {
                                options.commands().entity(button).insert(AutoFocus);
                            }
                        }
                    });
            }
            screen::controls(footer, |row| {
                let first = if pending.is_some() {
                    Some(ui::control(
                        art,
                        row,
                        "Continue",
                        Action::ContinueResult(table.epoch),
                    ))
                } else {
                    view.choices
                        .iter()
                        .find(|choice| is_decline(choice))
                        .and_then(|choice| {
                            let button = ui::control(
                                art,
                                row,
                                &opportunity_label(choice, view),
                                Action::ResultChoice(table.epoch, view.revision, choice.id.clone()),
                            );
                            (!view
                                .choices
                                .iter()
                                .any(|other| other.id != "undo" && !is_decline(other)))
                            .then_some(button)
                        })
                };
                if let Some(first) = first {
                    row.commands().entity(first).insert(AutoFocus);
                }
            });
            if !table.notice.is_empty() {
                footer.spawn(art.text(&table.notice, 16., GOLD));
            }
        },
    );
}

fn is_decline(choice: &crate::Choice) -> bool {
    matches!(
        choice.id.as_str(),
        "after-test:pass"
            | "success:pass"
            | "lucky:decline"
            | "patrice-shuffle:decline"
            | "spell-move:decline"
            | "lita:decline-damage"
            | "defeat:done"
    )
}

fn outcome(report: &TestResult) -> String {
    if report.success {
        format!("Succeeded by {}", report.total - report.difficulty)
    } else {
        format!("Failed by {}", report.failure_margin)
    }
}

pub(super) fn details(body: &mut ChildSpawnerCommands, art: &art::Art, report: &TestResult) {
    let detail = &report.breakdown;
    body.spawn(art.text(
        format!("Base {:?}  {}", detail.skill, detail.base),
        21.,
        PAPER,
    ));
    for (heading, rows) in [
        ("Committed cards", &detail.cards),
        ("Abilities and bonuses", &detail.bonuses),
    ] {
        body.spawn(art.text(heading, 20., GOLD));
        if rows.is_empty() {
            body.spawn(art.text("None", 18., MUTED));
        }
        for row in rows {
            body.spawn(art.text(
                format!("{} · {}  {:+}", row.label, row.owner.name(), row.amount),
                19.,
                PAPER,
            ));
        }
    }
    body.spawn(art.text("Chaos tokens", 20., GOLD));
    for token in &detail.tokens {
        token_row(body, art, token.face, &format!("{:+}", token.amount));
    }
    for (heading, faces) in [("Ignored", &detail.ignored), ("Canceled", &detail.canceled)] {
        for face in faces {
            token_row(body, art, *face, &format!("{heading} · no contribution"));
        }
    }
    if let Some(reason) = &detail.outcome_reason {
        body.spawn(art.text(reason, 19., GOLD));
    }
    let cards: i32 = detail.cards.iter().map(|row| row.amount).sum();
    let bonuses: i32 = detail.bonuses.iter().map(|row| row.amount).sum();
    let chaos = detail.token_total;
    body.spawn(art.text(
        format!(
            "{} base {:+} cards {:+} bonuses {:+} chaos = {}\nFinal total {} vs difficulty {} · {}",
            detail.base,
            cards,
            bonuses,
            chaos,
            detail.base + cards + bonuses + chaos,
            report.total,
            report.difficulty,
            outcome(report)
        ),
        20.,
        PAPER,
    ));
    if detail.base + cards + bonuses + chaos < 0 {
        body.spawn(art.text("Skill totals cannot be lower than 0.", 18., MUTED));
    }
}

fn token_row(body: &mut ChildSpawnerCommands, art: &art::Art, face: crate::TokenFace, value: &str) {
    body.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: px(16),
        flex_shrink: 0.,
        ..default()
    })
    .with_children(|row| {
        if let Some(image) = art.image(crate::tabletop_tokens::image_role(face)) {
            row.spawn((
                ImageNode::new(image),
                Node {
                    width: px(60),
                    height: px(60),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        }
        row.spawn(art.text(format!("{}  {value}", face.label()), 20., PAPER));
    });
}
