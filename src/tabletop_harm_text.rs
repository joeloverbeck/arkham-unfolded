//! Damage, horror and explicit skill bonuses share a visual spelling across views.
//!
//! This is a display-only substitution of harm words and `+N skill` labels. Rules, saved
//! history, accessible labels and the absent-font fallback keep their names.

use crate::tabletop::symbols::Skill;
use bevy::{ecs::spawn::SpawnIter, prelude::*};

pub(crate) const DAMAGE_COLOR: Color = Color::srgb(1.0, 0.48, 0.38);
pub(crate) const HORROR_COLOR: Color = Color::srgb(0.48, 0.72, 1.0);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum HarmIcon {
    Damage,
    Horror,
}

impl HarmIcon {
    pub(crate) fn glyph(self) -> &'static str {
        match self {
            Self::Damage => "Q",
            Self::Horror => "R",
        }
    }

    pub(crate) fn color(self) -> Color {
        match self {
            Self::Damage => DAMAGE_COLOR,
            Self::Horror => HORROR_COLOR,
        }
    }
}

enum Run {
    Words(String),
    Icon(HarmIcon),
    Skill(Skill),
}

fn runs(value: &str) -> Vec<Run> {
    let mut result = Vec::new();
    let mut words_start = 0;
    let mut cursor = 0;
    for word in value.split_inclusive(|c: char| !c.is_alphanumeric() && c != '_') {
        let end = word
            .char_indices()
            .find(|(_, c)| !c.is_alphanumeric() && *c != '_')
            .map_or(word.len(), |(i, _)| i);
        let icon = match &word[..end] {
            word if word.eq_ignore_ascii_case("damage") => Some(Run::Icon(HarmIcon::Damage)),
            word if word.eq_ignore_ascii_case("horror") => Some(Run::Icon(HarmIcon::Horror)),
            word => {
                // Skill names become glyphs only in explicit numeric bonus labels.
                // Ordinary rules prose and card descriptions retain their words.
                let bonus = value[..cursor]
                    .split_whitespace()
                    .next_back()
                    .and_then(|previous| previous.trim_start_matches('(').strip_prefix('+'))
                    .is_some_and(|amount| {
                        !amount.is_empty() && amount.bytes().all(|c| c.is_ascii_digit())
                    });
                bonus.then(|| Skill::named(word)).flatten().map(Run::Skill)
            }
        };
        if let Some(icon) = icon {
            if words_start < cursor {
                result.push(Run::Words(value[words_start..cursor].to_owned()));
            }
            result.push(icon);
            words_start = cursor + end;
        }
        cursor += word.len();
    }
    if words_start < value.len() {
        result.push(Run::Words(value[words_start..].to_owned()));
    }
    result
}

/// Spawn rich text for a panel or a projected world label. No font means plain
/// English, including the original case and the exact amounts.
pub(crate) fn ui(
    value: impl Into<String>,
    size: f32,
    color: Color,
    font: &Option<Handle<Font>>,
) -> impl Bundle {
    let label = value.into();
    let value = crate::tabletop::readable_text(label.clone());
    let has_icon = runs(&value).iter().any(|run| !matches!(run, Run::Words(_)));
    let (root, spans) = if let Some(font) = font.as_ref().filter(|_| has_icon) {
        (String::new(), spans(&value, size, color, font))
    } else {
        (value, Vec::new())
    };
    (
        Text::new(root),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        AccessibleLabel::new(label),
        Children::spawn(SpawnIter(spans.into_iter())),
    )
}

/// The existing authored rich-text runs also pass their English words through
/// the same spelling, without nesting text spans or changing their order.
pub(crate) fn spans(
    value: &str,
    size: f32,
    color: Color,
    font: &Handle<Font>,
) -> Vec<(TextSpan, TextFont, TextColor)> {
    runs(value)
        .into_iter()
        .map(|run| match run {
            Run::Words(words) => (
                TextSpan::new(crate::tabletop::readable_text(words)),
                TextFont {
                    font_size: FontSize::Px(size),
                    ..default()
                },
                TextColor(color),
            ),
            Run::Icon(icon) => (
                // Keep an adjacent quantity attached across the font change.
                TextSpan::new(format!("\u{2060}{}\u{2060}", icon.glyph())),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(size * 1.25),
                    ..default()
                },
                TextColor(icon.color()),
            ),
            Run::Skill(skill) => (
                TextSpan::new(format!("\u{2060}{}\u{2060}", skill.glyph())),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(size * 1.25),
                    ..default()
                },
                TextColor(color),
            ),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_bonuses_use_all_four_glyphs_without_rewriting_skill_prose_or_wild_icons() {
        let mut world = World::new();
        let label = "Commit +1 willpower / +2 intellect / +3 combat / +4 agility. Printed: 4 wild icons; combat 5.";
        let entity = world
            .spawn(ui(label, 16.0, Color::WHITE, &Some(Handle::default())))
            .id();
        let visible: String = world
            .get::<Children>(entity)
            .unwrap()
            .iter()
            .map(|child| world.get::<TextSpan>(child).unwrap().0.as_str())
            .collect();
        assert_eq!(
            visible,
            "Commit +1 \u{2060}f\u{2060} / +2 \u{2060}g\u{2060} / +3 \u{2060}h\u{2060} / +4 \u{2060}i\u{2060}. Printed: 4 wild icons; combat 5."
        );
        assert_eq!(world.get::<AccessibleLabel>(entity).unwrap().0, label);
        let fallback = world.spawn(ui(label, 16.0, Color::WHITE, &None)).id();
        assert_eq!(world.get::<Text>(fallback).unwrap().0, label);
    }

    #[test]
    fn visible_prose_uses_both_colored_glyphs_without_changing_amounts() {
        let font = Handle::<Font>::default();
        let spans = spans(
            "+2 Damage and +1 horror. Direct DAMAGE: 1",
            20.0,
            Color::WHITE,
            &font,
        );
        let visible: String = spans.iter().map(|(span, _, _)| span.0.as_str()).collect();
        assert_eq!(
            visible,
            "+2 \u{2060}Q\u{2060} and +1 \u{2060}R\u{2060}. Direct \u{2060}Q\u{2060}: 1"
        );
        let icons: Vec<_> = spans
            .iter()
            .filter(|(span, _, _)| span.0.contains('\u{2060}'))
            .collect();
        assert_eq!(icons.len(), 3);
        assert_eq!(icons[0].2.0, DAMAGE_COLOR);
        assert_eq!(icons[1].2.0, HORROR_COLOR);
        assert_eq!(icons[2].2.0, DAMAGE_COLOR);
    }

    #[test]
    fn word_substitution_preserves_unrelated_words_and_record_identifiers() {
        let spans = spans(
            "undamaged horrors damage_dealt horror2 [Damage]",
            15.0,
            Color::WHITE,
            &Handle::default(),
        );
        let visible: String = spans.iter().map(|(span, _, _)| span.0.as_str()).collect();
        assert_eq!(
            visible,
            "undamaged horrors damage_dealt horror2 [\u{2060}Q\u{2060}]"
        );
    }

    #[test]
    fn spawned_ui_keeps_accessible_english_and_plain_fallback() {
        let mut world = World::new();
        let fallback = world
            .spawn(ui("Damage 2 / Horror 1", 16.0, Color::WHITE, &None))
            .id();
        assert_eq!(
            world.get::<Text>(fallback).unwrap().0,
            "Damage 2 / Horror 1"
        );
        let icons = world
            .spawn(ui(
                "Damage 2 / Horror 1",
                16.0,
                Color::WHITE,
                &Some(Handle::default()),
            ))
            .id();
        let children = world.get::<Children>(icons).unwrap();
        let visible: String = children
            .iter()
            .map(|child| world.get::<TextSpan>(child).unwrap().0.as_str())
            .collect();
        assert_eq!(visible, "\u{2060}Q\u{2060} 2 / \u{2060}R\u{2060} 1");
        assert_eq!(
            world.get::<AccessibleLabel>(icons).unwrap().0,
            world.get::<AccessibleLabel>(fallback).unwrap().0
        );
    }
}
