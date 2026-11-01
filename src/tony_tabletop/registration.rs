//! Tony's application schedule, shared by native play and its behavioral tests.
//! Hosts supply the window, input/UI, assets, clocks, health and persistence paths.

use super::*;

pub(super) fn install(app: &mut App) {
    app.add_plugins(feedback::FeedbackPlugin)
        .add_plugins(reveal::RevealPlugin)
        .add_plugins(crate::tabletop_counters::CounterPlugin)
        .add_systems(Startup, setup)
        .add_systems(Last, request_redraw.in_set(RequestRedraw))
        .add_systems(
            Update,
            (
                poll_settings,
                handle_actions,
                advance_presentation,
                camera_controls,
                controls::invalidate,
                synchronize_scene,
                synchronize_feedback,
                controls::draw_menu,
                sync_reveal,
                reveal::render,
                world_labels,
                crate::tabletop_tokens::position_labels,
                hide_overview_token_labels,
                animate,
                animate_harm,
                button_colors,
                capture_window,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                clue_motion::synchronize
                    .after(synchronize_scene)
                    .before(synchronize_feedback),
                rebuild_panel
                    .after(synchronize_feedback)
                    .before(controls::draw_menu),
                clue_motion::animate
                    .after(animate_harm)
                    .before(button_colors),
            ),
        );
}
