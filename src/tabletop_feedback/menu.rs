//! Contextual action-list construction. Adapters own content, typed activation
//! bindings, menu lifetime, placement constraints and scrolling policy.

use bevy::{
    input_focus::{
        AutoFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::Button,
};

/// Ordered adapter-authored text and actions; labels already contain their
/// presentation-only detail breaks. This module never interprets game choices.
pub(crate) enum Item<A> {
    Text(String, f32, Color),
    Action(String, A),
}

/// The existing adapters use different action text sizes and picking targets.
pub(crate) struct MenuText<'a> {
    pub symbol_font: &'a Option<Handle<Font>>,
    pub action_size: f32,
    pub action_pickable: Pickable,
}

/// Build the complete modal list, acquiring focus at its first actionable row.
/// The returned root accepts the adapter's scroll observer and cleanup markers.
/// `bind` attaches typed activation behavior; creating/focusing a row does not
/// invoke that behavior. Ordinary utility/inspection buttons remain independent.
pub(crate) fn spawn<A>(
    commands: &mut Commands,
    panel: Node,
    decoration: impl Bundle,
    text: MenuText,
    items: impl IntoIterator<Item = Item<A>>,
    mut bind: impl FnMut(&mut EntityCommands, A),
) -> Entity {
    commands
        .spawn((
            panel,
            decoration,
            TabGroup::modal(),
            ScrollPosition::default(),
        ))
        .with_children(|parent| {
            let mut first = true;
            for item in items {
                match item {
                    Item::Text(value, size, color) => {
                        parent.spawn(crate::tabletop_harm_text::ui(
                            value,
                            size,
                            color,
                            text.symbol_font,
                        ));
                    }
                    Item::Action(label, action) => {
                        let mut button = parent.spawn((
                            Button,
                            Hovered::default(),
                            TabIndex(0),
                            AccessibleLabel::new(label.clone()),
                            row(),
                            super::context_button(),
                            BackgroundColor(super::NEUTRAL),
                        ));
                        if first {
                            button.insert(AutoFocus);
                            first = false;
                        }
                        bind(&mut button, action);
                        button.with_child((
                            crate::tabletop_harm_text::ui(
                                label,
                                text.action_size,
                                Color::srgb(0.91, 0.89, 0.81),
                                text.symbol_font,
                            ),
                            text.action_pickable,
                        ));
                    }
                }
            }
        })
        .id()
}

/// Keep rows beside their source, fitting the actual viewport and reserved UI.
/// Height follows content until the available space requires vertical scrolling.
pub(crate) fn panel(anchor: Vec2, viewport: Vec2, top: f32, bottom: f32) -> Node {
    let margin = 12.0_f32.min(viewport.x.min(viewport.y) * 0.25);
    let width = 340.0_f32.min((viewport.x - 2.0 * margin).max(1.0));
    let top = top.min(viewport.y * 0.2).max(margin);
    let bottom = (viewport.y - bottom)
        .max(top + 1.0)
        .min(viewport.y - margin);
    let right = anchor.x + 18.0;
    let left = if right + width <= viewport.x - margin {
        right
    } else {
        anchor.x - width - 18.0
    }
    .clamp(margin, (viewport.x - width - margin).max(margin));
    let source_y = anchor.y.clamp(top, bottom);
    let below = bottom - source_y;
    let above = source_y - top;
    let (top_edge, bottom_edge, height) = if below >= 240.0 || below >= above {
        let start = (source_y - 48.0).max(top);
        (px(start), Val::Auto, bottom - start)
    } else {
        let end = (source_y + 48.0).min(bottom);
        (Val::Auto, px(viewport.y - end), end - top)
    };
    Node {
        position_type: PositionType::Absolute,
        left: px(left),
        top: top_edge,
        bottom: bottom_edge,
        width: px(width),
        max_height: px(height.clamp(1.0, 500.0)),
        padding: UiRect::all(px(8)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: px(6),
        overflow: Overflow::scroll_y(),
        ..default()
    }
}

/// Move the first existing detail boundary onto a new line, preserving words
/// and symbols rather than parsing or shortening the action's rules meaning.
pub(crate) fn label(value: &str) -> String {
    if value.contains(" (") {
        value.replacen(" (", "\n(", 1)
    } else {
        value.replacen(" · ", "\n", 1)
    }
}

/// A column lets its text wrap to the row's width without a fixed row height.
fn row() -> Node {
    Node {
        width: percent(100),
        min_width: px(0),
        min_height: px(40),
        padding: UiRect::axes(px(12), px(9)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        justify_content: JustifyContent::Center,
        flex_shrink: 0.0,
        border_radius: BorderRadius::all(px(4)),
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        camera::{ComputedCameraValues, RenderTarget, RenderTargetInfo},
        window::{PrimaryWindow, WindowRef},
    };

    fn rect(world: &World, entity: Entity) -> Rect {
        let size = world.get::<ComputedNode>(entity).unwrap().size();
        let transform = world.get::<UiGlobalTransform>(entity).unwrap();
        Rect {
            min: transform.transform_point2(-size * 0.5),
            max: transform.transform_point2(size * 0.5),
        }
    }

    #[test]
    fn long_action_rows_wrap_without_overlapping_inspect_at_viewport_edges() {
        for (size, adapter) in [UVec2::new(1600, 1000), UVec2::new(420, 320)]
            .into_iter()
            .flat_map(|size| ["tony", "party", "jacqueline"].map(|adapter| (size, adapter)))
        {
            let mut app = crate::tabletop_layout::heading_checks::app();
            app.init_resource::<Assets<StandardMaterial>>()
                .add_plugins(crate::tabletop_feedback::FeedbackPlugin);
            let window = app
                .world_mut()
                .spawn((
                    Window {
                        resolution: (size.x, size.y).into(),
                        ..default()
                    },
                    PrimaryWindow,
                ))
                .id();
            app.world_mut().spawn((
                Camera {
                    computed: ComputedCameraValues {
                        target_info: Some(RenderTargetInfo {
                            physical_size: size,
                            scale_factor: 1.0,
                        }),
                        ..default()
                    },
                    ..default()
                },
                RenderTarget::Window(WindowRef::Entity(window)),
            ));
            // Real adapter content, including their surrounding heading/hint
            // and status recipes. Only the content varies; construction is production.
            let hint = crate::tabletop_feedback::KEYBOARD_HINT;
            let gold = super::super::GOLD;
            let muted = Color::srgb(0.62, 0.71, 0.70);
            let bottom_reserved = (size.y as f32 * 0.29).min(250.) + 20.;
            let (top, bottom, action_size, mut content) = match adapter {
                "tony" => (68., bottom_reserved, 16., vec![
                    Item::Text(format!("Study · Tony's decision\n{hint}"), 14., gold),
                    Item::Action("Investigate\n1 action\nAttack of opportunity: Ghoul Minion\n1 damage + 1 horror".into(), ()),
                ]),
                "party" => (110., bottom_reserved, 14., vec![
                    Item::Text(format!("Patrice's turn\n{hint}"), 14., gold),
                    Item::Action(label("Patrice: Investigate Study (one action; intellect against shroud 2)"), ()),
                ]),
                "jacqueline" => (12., 12., 16., vec![
                    Item::Text("Promise of Power".into(), 17., gold),
                    Item::Text(hint.into(), 14., muted),
                    Item::Action(label("Commit Promise of Power (+4 willpower; add a curse)"), ()),
                    Item::Action("Dismiss".into(), ()),
                ]),
                _ => unreachable!(),
            };
            content.push(Item::Action("Inspect".into(), ()));
            let panel = spawn(
                &mut app.world_mut().commands(),
                super::panel(
                    size.as_vec2() - Vec2::splat(2.),
                    size.as_vec2(),
                    top,
                    bottom,
                ),
                BackgroundColor(Color::BLACK),
                MenuText {
                    // Absent symbol art must retain readable English glyphs.
                    symbol_font: &None,
                    action_size,
                    action_pickable: if adapter == "party" {
                        Pickable::IGNORE
                    } else {
                        Pickable::default()
                    },
                },
                content,
                |_, ()| {},
            );
            app.world_mut().flush();
            app.finish();
            app.cleanup();
            for _ in 0..5 {
                app.update();
            }
            let world = app.world();
            let children = world.get::<Children>(panel).unwrap();
            let rows: Vec<_> = children
                .iter()
                .filter(|e| world.get::<Button>(*e).is_some())
                .collect();
            assert_eq!(
                world.resource::<bevy::input_focus::InputFocus>().get(),
                Some(rows[0])
            );
            assert!(world.get::<TabGroup>(panel).is_some());
            let panel_rect = rect(app.world(), panel);
            assert!(panel_rect.min.x >= 12. && panel_rect.max.x <= size.x as f32 - 12.);
            assert!(panel_rect.min.y >= top.min(size.y as f32 * 0.2));
            assert!(panel_rect.max.y <= size.y as f32 - bottom + 0.1);
            let mut previous_bottom = f32::NEG_INFINITY;
            for child in children.iter() {
                let child_rect = rect(world, child);
                assert!(
                    child_rect.min.y > previous_bottom,
                    "{adapter}: children overlap"
                );
                previous_bottom = child_rect.max.y;
                let text = if world.get::<Button>(child).is_some() {
                    world.get::<Children>(child).unwrap()[0]
                } else {
                    child
                };
                let text_rect = rect(world, text);
                let layout = world.get::<bevy::text::TextLayoutInfo>(text).unwrap();
                assert!(!layout.glyphs.is_empty(), "{adapter}: real glyphs required");
                assert!(text_rect.min.cmpge(child_rect.min - Vec2::splat(0.1)).all());
                assert!(text_rect.max.cmple(child_rect.max + Vec2::splat(0.1)).all());
                assert!(
                    layout.size.x <= text_rect.width() + 0.1,
                    "{adapter}: glyphs overflow"
                );
            }
            {
                let action_text = world.get::<Children>(rows[0]).unwrap()[0];
                assert!(
                    rect(world, action_text).height() > 32.,
                    "{adapter}: details must wrap"
                );
                assert!(
                    rect(world, rows[0]).height() > rect(world, *rows.last().unwrap()).height()
                );
            }
        }
    }
}
