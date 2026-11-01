//! Shared screen-space geometry for contextual action lists.

use bevy::prelude::*;

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
pub(crate) fn row() -> Node {
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
        for size in [UVec2::new(1600, 1000), UVec2::new(420, 320)] {
            let mut app = crate::tabletop_layout::heading_checks::app();
            app.init_resource::<Assets<Mesh>>();
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
            let panel = app
                .world_mut()
                .spawn(panel(
                    size.as_vec2() - Vec2::splat(2.),
                    size.as_vec2(),
                    12.,
                    12.,
                ))
                .id();
            let mut rows = Vec::new();
            let mut texts = Vec::new();
            for label in [
                "Patrice: Investigate Study (one action; intellect against shroud 2)\nAttack of opportunity: Ghoul Minion, 1 damage and 1 horror",
                "Inspect",
            ] {
                let text = app
                    .world_mut()
                    .spawn(crate::tabletop_harm_text::ui(
                        super::label(label),
                        16.,
                        Color::WHITE,
                        &None,
                    ))
                    .id();
                let row = app.world_mut().spawn(row()).add_child(text).id();
                app.world_mut().entity_mut(panel).add_child(row);
                rows.push(row);
                texts.push(text);
            }
            app.finish();
            app.cleanup();
            for _ in 0..5 {
                app.update();
            }
            let panel_rect = rect(app.world(), panel);
            let action_rect = rect(app.world(), rows[0]);
            let inspect_rect = rect(app.world(), rows[1]);
            let text_rect = rect(app.world(), texts[0]);
            assert!(panel_rect.min.cmpge(Vec2::ZERO).all());
            assert!(panel_rect.max.cmple(size.as_vec2()).all());
            assert!(action_rect.height() > inspect_rect.height());
            assert!(action_rect.max.y < inspect_rect.min.y);
            assert!(text_rect.min.cmpge(action_rect.min).all());
            assert!(text_rect.max.cmple(action_rect.max).all());
            assert!(
                text_rect.height() > 32.,
                "long text must wrap: {text_rect:?}"
            );
            let layout = app
                .world()
                .get::<bevy::text::TextLayoutInfo>(texts[0])
                .unwrap();
            assert!(!layout.glyphs.is_empty(), "test must lay out real glyphs");
        }
    }
}
