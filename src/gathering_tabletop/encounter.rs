//! An untimed reading stop over a committed encounter draw. Any still-pending
//! responses use the same legal choices and commit path as the tabletop.
use super::*;
use crate::gathering::{CardView, DrawCue, DrawSource};

#[derive(Clone, Copy)]
pub(super) enum Input {
    Continue,
    Rules,
}

struct Draw {
    revision: u64,
    sequence: Option<u32>,
    revelation: Reveal,
}

pub(super) struct Reveal {
    pub card: CardView,
    pub drawn_by: Investigator,
}

#[derive(Default)]
pub(super) struct Reading {
    waiting: Vec<Draw>,
    current: Option<Reveal>,
    rules: bool,
    elapsed: f32,
}

impl Reading {
    pub(super) fn for_session(session: &Session) -> Self {
        // A fresh process can re-read an unresolved draw. Started resolutions and
        // nested cancellation tests do not become new draws on reopening.
        let view = session.observe();
        if !view
            .choices
            .iter()
            .any(|choice| choice.id == "revelation:resolve")
        {
            return Self::default();
        }
        Self {
            current: view.play.and_then(|play| {
                play.pending_revelation
                    .filter(|revelation| {
                        !revelation.started && !revelation.cancelled && !revelation.card.weakness
                    })
                    .map(|revelation| Reveal {
                        card: revelation.card,
                        drawn_by: revelation.drawn_by,
                    })
            }),
            ..default()
        }
    }

    pub(super) fn committed(&mut self, session: &Session, cues: &[DrawCue]) {
        self.current = None;
        self.rules = false;
        self.waiting = cues
            .iter()
            .filter(|cue| cue.batch.source == DrawSource::Encounter)
            .filter_map(|cue| {
                let view = cue.sequence.map_or_else(
                    || Some(session.observe()),
                    |sequence| {
                        session
                            .presentation_after(cue.revision.saturating_sub(1))
                            .iter()
                            .find(|moment| moment.sequence == sequence)
                            .map(|moment| session.observe_presentation(moment))
                    },
                )?;
                let revelation = view.play?.pending_revelation?;
                cue.batch
                    .cards
                    .iter()
                    .any(|card| card.id == revelation.card.id)
                    .then_some(Draw {
                        revision: cue.revision,
                        sequence: cue.sequence,
                        revelation: Reveal {
                            card: revelation.card,
                            drawn_by: revelation.drawn_by,
                        },
                    })
            })
            .collect();
    }

    pub(super) fn arrive(&mut self, revision: u64, sequence: Option<u32>) {
        if self.current.is_none()
            && let Some(index) = self
                .waiting
                .iter()
                .position(|draw| draw.revision == revision && draw.sequence == sequence)
        {
            self.current = Some(self.waiting.remove(index).revelation);
            self.rules = false;
            self.elapsed = 0.;
        }
    }

    pub(super) fn current(&self) -> Option<&Reveal> {
        self.current.as_ref()
    }

    pub(super) fn decision(&self, session: &Session) -> Option<Observation> {
        let card = &self.current.as_ref()?.card;
        let view = session.observe();
        let pending = view.play.as_ref()?.pending_revelation.as_ref()?;
        (pending.card.id == card.id
            && !pending.started
            && !pending.cancelled
            && view
                .choices
                .iter()
                .any(|choice| choice.id == "revelation:resolve"))
        .then_some(view)
    }

    pub(super) fn accepts(&self, session: &Session, revision: u64, id: &str) -> bool {
        self.decision(session).is_some_and(|view| {
            view.revision == revision
                && id != "undo"
                && view.choices.iter().any(|choice| choice.id == id)
        })
    }

    /// The responses the human may choose while reading: with agent partners, only
    /// those of investigators the human controls (#205).
    fn responses<'a>(table: &Table, view: &'a Observation) -> Vec<&'a crate::Choice> {
        view.choices
            .iter()
            .filter(|choice| !matches!(choice.id.as_str(), "revelation:resolve" | "undo"))
            .filter(|choice| table.human_may(&choice.id))
            .collect()
    }

    pub(super) fn input(&mut self, input: Input) {
        match input {
            Input::Continue => self.current = None,
            Input::Rules => self.rules = !self.rules,
        }
    }
}

pub(super) fn render(commands: &mut Commands, table: &Table, art: &art::Art, size: Vec2) {
    let reading = table.presentation.encounter();
    let Some(revelation) = reading.current() else {
        return;
    };
    let card = &revelation.card;
    let decision = reading.decision(&table.session);
    let responses = decision
        .as_ref()
        .map(|view| Reading::responses(table, view))
        .unwrap_or_default();
    let title = crate::gathering::inspection::encounter_card_type(&card.code).map_or_else(
        || card.title.clone(),
        |kind| format!("{} · {kind}", card.title),
    );
    let width = (size.x * 0.94).min(650.);
    let response_height = responses.len() as f32 * 66.;
    let image_width = ((size.y * 0.9 - 250. - response_height) * 5. / 7.)
        .clamp(180., 430.)
        .min(width - 56.);
    let label = format!(
        "Encounter card: {}, drawn by {}",
        card.title,
        revelation.drawn_by.name()
    );
    let overlay = reading::panel(
        commands,
        size,
        width,
        &label,
        |header| {
            header.spawn((
                art.text("ENCOUNTER CARD", 17., GOLD),
                TextLayout::justify(Justify::Center),
            ));
            header.spawn((
                art.text(
                    format!("Drawn by {}", revelation.drawn_by.name()),
                    20.,
                    PAPER,
                ),
                TextLayout::justify(Justify::Center),
            ));
        },
        |body| {
            body.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                flex_shrink: 0.,
                ..default()
            })
            .with_children(|body| {
                if reading.rules {
                    if let Some(text) =
                        crate::gathering::inspection::scenario_card(&card.code, false)
                    {
                        body.spawn(art.text(text, 21., PAPER));
                    }
                } else if let Some(image) = art.card(&card.code, false) {
                    crate::tabletop_ui::card_image(body, image, false, image_width);
                }
            });
        },
        |footer| {
            footer.spawn((
                art.text(title, 23., PAPER),
                TextLayout::justify(Justify::Center),
            ));
            footer.spawn((
                art.text(
                    if decision.is_some() {
                        "Continue to resolve this encounter, or choose a response first."
                    } else {
                        "Read the card, then continue."
                    },
                    14.,
                    MUTED,
                ),
                TextLayout::justify(Justify::Center),
            ));
            if let Some(view) = &decision {
                for response in &responses {
                    ui::button(
                        art,
                        footer,
                        &response.label,
                        Action::EncounterChoice(table.epoch, view.revision, response.id.clone()),
                    );
                }
            }
            if !table.notice.is_empty() {
                footer.spawn(art.text(&table.notice, 14., PAPER));
            }
            footer
                .spawn(Node {
                    display: Display::Grid,
                    grid_template_columns: vec![RepeatedGridTrack::flex(2, 1.)],
                    column_gap: px(12),
                    row_gap: px(8),
                    ..default()
                })
                .with_children(|row| {
                    let continue_button = ui::button(
                        art,
                        row,
                        "Continue",
                        decision.as_ref().map_or(
                            Action::Encounter(table.epoch, Input::Continue),
                            |view| {
                                Action::EncounterChoice(
                                    table.epoch,
                                    view.revision,
                                    "revelation:resolve".into(),
                                )
                            },
                        ),
                    );
                    row.commands().entity(continue_button).insert(AutoFocus);
                    ui::button(
                        art,
                        row,
                        if reading.rules {
                            "Show card"
                        } else {
                            "Read rules"
                        },
                        Action::Encounter(table.epoch, Input::Rules),
                    );
                });
        },
    );
    surround(commands, overlay, size, width, reading.elapsed);
}

const SEGMENTS: usize = 28;

#[derive(Component)]
pub(super) struct Tendril {
    strand: usize,
    segment: usize,
    size: Vec2,
}

impl Tendril {
    fn point(&self, t: f32, elapsed: f32) -> Vec2 {
        let phase = elapsed * std::f32::consts::TAU / [10., 12., 8., 11.][self.strand]
            + self.strand as f32 * 1.7;
        let x = self.size.x * (0.12 + 0.54 * t + 0.18 * t * (t * 5.2 + phase).sin());
        let y = if self.strand < 2 {
            self.size.y * (1.04 - 0.75 * t)
        } else {
            self.size.y * (-0.05 + 0.51 * t)
        };
        Vec2::new(
            if self.strand.is_multiple_of(2) {
                x
            } else {
                self.size.x - x
            },
            y,
        )
    }

    fn pose(&self, elapsed: f32) -> (Node, UiTransform) {
        let t = self.segment as f32 / SEGMENTS as f32;
        let next = (self.segment + 1) as f32 / SEGMENTS as f32;
        let start = self.point(t, elapsed);
        let end = self.point(next, elapsed);
        let center = (start + end) * 0.5;
        let direction = end - start;
        let thickness = self.size.x.min(180.) * 0.26 * (1. - (t + next) * 0.5).powf(1.35) + 1.;
        let length = direction.length() + thickness;
        (
            Node {
                position_type: PositionType::Absolute,
                left: px(center.x - thickness * 0.5),
                top: px(center.y - length * 0.5),
                width: px(thickness),
                height: px(length),
                border_radius: BorderRadius::all(percent(50)),
                ..default()
            },
            UiTransform {
                rotation: Rot2::radians(-direction.x.atan2(direction.y)),
                ..default()
            },
        )
    }
}

fn surround(commands: &mut Commands, overlay: Entity, size: Vec2, panel_width: f32, elapsed: f32) {
    // Clip every contour to the side gutters. Neither motion nor resizing can
    // carry it across the card, heading, effective rules or controls.
    let gutter = ((size.x - panel_width) * 0.5 - 18.).max(0.);
    if gutter < 12. {
        return;
    }
    commands.entity(overlay).with_children(|overlay| {
        for right in [false, true] {
            overlay
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: if right { px(size.x - gutter) } else { px(0) },
                        top: px(0),
                        width: px(gutter),
                        height: px(size.y),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    GlobalZIndex(91),
                    Pickable::IGNORE,
                ))
                .with_children(|edge| {
                    for strand in [usize::from(right), 2 + usize::from(right)] {
                        for segment in 0..SEGMENTS {
                            let tendril = Tendril {
                                strand,
                                segment,
                                size: Vec2::new(gutter, size.y),
                            };
                            let (node, transform) = tendril.pose(elapsed);
                            edge.spawn((
                                Name::new("Encounter tendril"),
                                node,
                                transform,
                                tendril,
                                BackgroundColor(Color::srgb(0.035, 0.105, 0.115)),
                                Pickable::IGNORE,
                            ));
                        }
                    }
                });
        }
    });
}

/// Only decorative UI transforms change here; never rebuild the table or advance
/// its presentation/decision clock to animate a reading stop.
pub(super) fn animate(
    time: Res<PresentationTime>,
    health: Res<RenderHealth>,
    windows: Query<&Window>,
    mut table: ResMut<Table>,
    mut segments: Query<(&Tendril, &mut Node, &mut UiTransform)>,
    mut was_running: Local<bool>,
    mut redraw: ResMut<Redraw>,
) {
    let reading = table.presentation.encounter();
    let running = reading.current().is_some()
        && !table.reduced_motion
        && health.allows_play()
        && windows.iter().any(|window| window.focused);
    if running {
        redraw.request();
        if *was_running {
            let reading = table.presentation.encounter_mut();
            reading.elapsed = (reading.elapsed + time.delta_secs().min(0.05)) % 2640.;
        }
    }
    *was_running = running;
    let elapsed = table.presentation.encounter().elapsed;
    for (segment, mut node, mut transform) in &mut segments {
        let (next_node, next_transform) = segment.pose(elapsed);
        *node = next_node;
        *transform = next_transform;
    }
}
