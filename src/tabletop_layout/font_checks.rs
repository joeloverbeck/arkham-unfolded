//! Coverage of displayed text against the actual font asset, without a desktop.
use bevy::prelude::*;

pub(crate) fn assert_supported(world: &mut World) {
    let failures = unsupported(world);
    assert!(
        failures.is_empty(),
        "Unsupported displayed text: {failures:#?}"
    );
}

fn unsupported(world: &mut World) -> Vec<String> {
    let mut query = world.query::<(Entity, Option<&Text>, Option<&TextSpan>, Option<&Text2d>)>();
    let fonts = world.resource::<Assets<Font>>();
    let mut failures = Vec::new();
    for (entity, text, span, world_text) in query.iter(world) {
        let Some(value) = text
            .map(|v| &v.0)
            .or_else(|| span.map(|v| &v.0))
            .or_else(|| world_text.map(|v| &v.0))
        else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        let mut ancestor = entity;
        let font = loop {
            if let Some(font) = world.get::<TextFont>(ancestor) {
                break Some(font);
            }
            let Some(parent) = world.get::<ChildOf>(ancestor) else {
                break None;
            };
            ancestor = parent.parent();
        };
        let Some(TextFont {
            font: FontSource::Handle(handle),
            ..
        }) = font
        else {
            failures.push(format!("{entity:?}: no explicit font asset for {value:?}"));
            continue;
        };
        let Some(font) = fonts
            .get(handle)
            .and_then(|font| swash::FontRef::from_index(font.data.as_ref(), 0))
        else {
            failures.push(format!(
                "{entity:?}: missing or unreadable assigned font for {value:?}"
            ));
            continue;
        };
        for c in value
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '\u{2060}')
        {
            if font.charmap().map(c) == 0 {
                failures.push(format!("{entity:?}: U+{:04X} {c:?} in {value:?}", c as u32));
            }
        }
    }
    failures
}

#[test]
fn detects_missing_glyphs_in_displayed_spans_and_uses_the_assigned_font() {
    let mut app = super::heading_checks::app();
    let world = app.world_mut();
    let custom = world
        .resource_mut::<Assets<Font>>()
        .add(Font::from_bytes(bevy::text::DEFAULT_FONT_DATA.to_vec()));
    let root = world
        .spawn((
            Text::new("Ready"),
            AccessibleLabel("▸ accessible wording".into()),
        ))
        .id();
    let child = world
        .spawn((
            TextSpan::new("\u{2060}Q\u{2060}\n"),
            TextFont {
                font: custom.clone().into(),
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    assert!(unsupported(world).is_empty());
    world.get_mut::<TextSpan>(child).unwrap().0 = "▸".into();
    assert!(
        unsupported(world)
            .iter()
            .any(|error| error.contains("U+25B8"))
    );
    world.get_mut::<TextSpan>(child).unwrap().0 = "Q".into();
    world.resource_mut::<Assets<Font>>().remove(custom.id());
    assert!(
        unsupported(world)
            .iter()
            .any(|error| error.contains("missing or unreadable assigned font"))
    );
}
