//! Reading a recorded result never runs a rules decision. Optional effects use
//! the ordinary revision-bound choices, after the result has been acknowledged.
use super::*;
use crate::gathering::{Session, TestAction, TestResult, TestView};
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

/// What a test attempts, for the decision display throughout it (#210):
/// `Tony Morgan fights Ghoul Minion`, `Tony Morgan tests Willpower for Crypt Chill`.
/// A weapon, spell or event that makes the test is named after it.
pub(super) fn attempt(test: &TestView) -> String {
    let who = test.tester.name();
    let target = test.target.as_ref().map(|target| target.title.as_str());
    let named = match (test.operation, target) {
        (Some(TestAction::Fight), Some(enemy)) => format!("{who} fights {enemy}"),
        (Some(TestAction::Evade), Some(enemy)) => format!("{who} evades {enemy}"),
        (Some(TestAction::Investigate), Some(location)) => format!("{who} investigates {location}"),
        (Some(TestAction::Parley), Some(card)) => format!("{who} parleys with {card}"),
        (_, Some(card)) => format!("{who} tests {:?} for {card}", test.skill),
        (_, None) => format!("{who} tests {:?}", test.skill),
    };
    match made_with(test) {
        Some(source) => format!("{named} with {source}"),
        None => named,
    }
}

/// The result's context after the tester (#210): `Fight · Ghoul Minion`,
/// `Willpower test · Crypt Chill`, then any weapon, spell or event that made it.
fn named(test: &TestView) -> String {
    let action = match test.operation {
        Some(TestAction::Fight) => "Fight".to_owned(),
        Some(TestAction::Evade) => "Evade".to_owned(),
        Some(TestAction::Investigate) => "Investigate".to_owned(),
        Some(TestAction::Parley) => "Parley".to_owned(),
        Some(TestAction::Card) | None => format!("{:?} test", test.skill),
    };
    test.target
        .as_ref()
        .map(|target| target.title.as_str())
        .into_iter()
        .chain(made_with(test))
        .fold(action, |named, part| format!("{named} · {part}"))
}

/// The card that makes the test, when it is not the test's target.
fn made_with(test: &TestView) -> Option<&str> {
    let source = test.source_card.as_ref()?;
    (test.target.as_ref().map(|target| &target.code) != Some(&source.code))
        .then_some(source.title.as_str())
}

fn result(test: &TestView) -> Option<TestResult> {
    Some(TestResult {
        tester: test.tester,
        source: named(test),
        total: test.total?,
        difficulty: test.difficulty,
        success: test.would_succeed?,
        failure_margin: test.failure_margin?,
        breakdown: test.breakdown.clone(),
    })
}

pub(super) fn render(
    commands: &mut Commands,
    table: &Table,
    view: &Observation,
    art: &art::Art,
    size: Vec2,
) {
    let pending = table.presentation.result();
    let opportunities = controls::result_opportunities(view);
    let uses = opportunities
        .as_ref()
        .map_or_else(Vec::new, |offered| offered.uses());
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
    reading::panel(
        commands,
        size,
        (size.x * 0.94).min(760.),
        "Skill test result",
        |panel| {
            panel.spawn((
                art.text(&heading, 28., GOLD),
                Node {
                    flex_shrink: 0.,
                    ..default()
                },
            ));
            if let Some(report) = &report {
                panel.spawn((
                    art.text(
                        format!(
                            "Total {}  ·  Difficulty {}",
                            report.total, report.difficulty
                        ),
                        24.,
                        PAPER,
                    ),
                    Node {
                        flex_shrink: 0.,
                        ..default()
                    },
                ));
                panel.spawn((
                    art.text(
                        format!("{} · {}", report.tester.name(), report.source),
                        18.,
                        MUTED,
                    ),
                    Node {
                        flex_shrink: 0.,
                        ..default()
                    },
                ));
            }
        },
        |body| {
            if let Some(report) = &report {
                details(body, art, report);
            }
        },
        |footer| {
            if pending.is_none() {
                footer.spawn(
                    art.text(
                        if opportunities
                            .as_ref()
                            .is_some_and(|offered| offered.decline().is_none())
                        {
                            "Choose which result to apply next. All remaining results still apply."
                        } else {
                            "Use an available effect, or explicitly decline."
                        },
                        18.,
                        GOLD,
                    ),
                );
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
                        for (index, choice) in uses.iter().enumerate() {
                            let button = ui::button(
                                art,
                                options,
                                &choice.label,
                                Action::ResultChoice(table.epoch, view.revision, choice.id.clone()),
                            );
                            if index == 0 {
                                options.commands().entity(button).insert(AutoFocus);
                            }
                        }
                    });
            }
            footer
                .spawn(Node {
                    justify_content: JustifyContent::Center,
                    flex_wrap: FlexWrap::Wrap,
                    flex_shrink: 0.,
                    ..default()
                })
                .with_children(|row| {
                    let first = if pending.is_some() {
                        Some(ui::control(
                            art,
                            row,
                            "Continue",
                            Action::ContinueResult(table.epoch),
                        ))
                    } else {
                        opportunities.as_ref().and_then(|offered| {
                            let choice = offered.decline()?;
                            let button = ui::control(
                                art,
                                row,
                                &choice.label,
                                Action::ResultChoice(table.epoch, view.revision, choice.id.clone()),
                            );
                            uses.is_empty().then_some(button)
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

pub(super) fn outcome(report: &TestResult) -> String {
    if report
        .breakdown
        .tokens
        .iter()
        .any(|token| token.face == crate::TokenFace::AutoFail)
    {
        "Automatic failure".into()
    } else if report.success {
        format!("Succeeded by {}", report.total - report.difficulty)
    } else {
        format!("Failed by {}", report.failure_margin)
    }
}

pub(super) fn details(body: &mut ChildSpawnerCommands, art: &art::Art, report: &TestResult) {
    let detail = &report.breakdown;
    contribution(
        body,
        art,
        &format!("Base {:?}", detail.skill),
        &detail.base.to_string(),
    );
    for (heading, rows) in [
        ("Committed cards", &detail.cards),
        ("Abilities and bonuses", &detail.bonuses),
    ] {
        if rows.is_empty() {
            continue;
        }
        body.spawn(art.text(heading, 18., GOLD));
        for row in rows {
            contribution(
                body,
                art,
                &format!("{} · {}", row.label, row.owner.name()),
                &format!("{:+}", row.amount),
            );
        }
    }
    body.spawn(art.text("Chaos tokens", 18., GOLD));
    for token in &detail.tokens {
        use crate::TokenFace;
        let amount = format!("{:+}", token.amount).replace('-', "−");
        let label = match token.face {
            TokenFace::AutoFail => "Automatic failure".to_owned(),
            TokenFace::PlusOne
            | TokenFace::Zero
            | TokenFace::MinusOne
            | TokenFace::MinusTwo
            | TokenFace::MinusThree
            | TokenFace::MinusFour => {
                if token.face.modifier() == Some(token.amount) {
                    token.face.label().to_owned()
                } else {
                    format!("{} → {amount}", token.face.label())
                }
            }
            _ => format!("{} · {amount}", token.face.label()),
        };
        token_row(body, art, token.face, &label);
    }
    for (heading, faces) in [("Ignored", &detail.ignored), ("Canceled", &detail.canceled)] {
        for face in faces {
            token_row(
                body,
                art,
                *face,
                &format!("{} · {heading} · no contribution", face.label()),
            );
        }
    }
    if let Some(reason) = &detail.outcome_reason {
        body.spawn(art.text(reason, 19., GOLD));
    }
    let cards: i32 = detail.cards.iter().map(|row| row.amount).sum();
    let bonuses: i32 = detail.bonuses.iter().map(|row| row.amount).sum();
    let chaos = detail.token_total;
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
                    width: px(40),
                    height: px(40),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        }
        row.spawn(art.text(value, 20., PAPER));
    });
}

fn contribution(body: &mut ChildSpawnerCommands, art: &art::Art, label: &str, amount: &str) {
    body.spawn(Node {
        width: percent(100),
        column_gap: px(16),
        align_items: AlignItems::Center,
        flex_shrink: 0.,
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            art.text(label, 19., PAPER),
            Node {
                flex_grow: 1.,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            },
        ));
        row.spawn((
            art.text(amount, 21., PAPER),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ));
    });
}
