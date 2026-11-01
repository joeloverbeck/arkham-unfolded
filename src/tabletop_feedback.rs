//! Presentation feedback shared by the two tables. Adapters own action legality,
//! modal capture and pending choices; this module never dispatches a game action.

use crate::tabletop_pacing::{PresentationTime, Redraw};
mod commitment;
pub(crate) mod menu;
use bevy::{
    camera::visibility::VisibilitySystems,
    input_focus::{InputFocus, tab_navigation::TabNavigationPlugin},
    picking::{hover::HoverMap, pointer::PointerId},
    prelude::*,
    ui::Pressed,
};
pub(crate) use commitment::Contribution;

const NEUTRAL: Color = Color::srgb(0.12, 0.19, 0.21);
const HOVER: Color = Color::srgb(0.24, 0.40, 0.39);
const GOLD: Color = Color::srgb(0.86, 0.69, 0.37);
const TEAL: Color = Color::srgb(0.25, 0.88, 0.76);
const HOVER_FADE_SECONDS: f32 = 0.12;
pub(crate) const KEYBOARD_HINT: &str = "Tab / Shift+Tab: move\nEnter / Space: choose · Esc: close";

/// Every meaningful menu operation participates: Some is an inspection target,
/// None is a game operation. Never choose between targets or execute a game action.
pub(crate) fn sole_inspection<T>(operations: impl IntoIterator<Item = Option<T>>) -> Option<T> {
    let mut operations = operations.into_iter();
    let target = operations.next()??;
    operations.next().is_none().then_some(target)
}

/// Contextual choices and reveal controls share this style; utility controls retain theirs.
#[derive(Component)]
pub(crate) struct ContextButton;

/// Presentation-only opacity; hover and keyboard focus retain their shared style.
#[derive(Component)]
pub(crate) struct FeedbackOpacity(pub f32);

pub(crate) fn context_button() -> impl Bundle {
    (ContextButton, Outline::new(px(2), px(2), Color::NONE))
}

#[derive(Resource, Default)]
pub(crate) struct FeedbackSettings {
    /// Inspection, other modals and native dialogs capture the table pointer.
    pub blocked: bool,
    pub reduced_motion: bool,
}

/// Add only to currently clickable faces. Inspection counts as interaction.
#[derive(Component, Default)]
pub(crate) struct CardFeedback {
    pub menu_source: bool,
    pub pending_choice: bool,
    pub commitment: Option<Contribution>,
    pub symbol_font: Option<Handle<Font>>,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Mark {
    Hover,
    MenuSource,
    PendingChoice,
    AvailableCommitment,
}

#[derive(Component)]
struct CardMark {
    card: Entity,
    kind: Mark,
    opacity: f32,
}

pub(crate) struct FeedbackPlugin;

impl Plugin for FeedbackPlugin {
    fn build(&self, app: &mut App) {
        // DefaultPlugins supplies focus and input dispatch, but tab navigation
        // must be installed explicitly on the production path used by both tables.
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        app.init_resource::<FeedbackSettings>().add_systems(
            PostUpdate,
            (button_feedback, card_feedback).before(VisibilitySystems::VisibilityPropagate),
        );
        app.add_systems(
            PostUpdate,
            commitment::badges.before(bevy::ui::UiSystems::Layout),
        );
    }
}

/// Child geometry follows the face's full transform, including the upright books.
/// Handles are owned by those children and released when their card despawns.
pub(crate) fn attach_card_feedback(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    card: Entity,
    width: f32,
    length: f32,
    surface_height: f32,
) {
    commands.entity(card).insert(CardFeedback::default());
    commitment::attach(
        commands,
        card,
        Vec3::new(0.0, surface_height + 0.04, -length * 0.5),
    );
    let cube = meshes.add(Cuboid::from_size(Vec3::ONE));
    for (kind, color, name) in [
        (
            Mark::Hover,
            Color::srgb(0.76, 0.91, 0.91),
            "Card hover frame",
        ),
        (
            Mark::MenuSource,
            Color::srgb(0.40, 0.72, 0.85),
            "Card menu source",
        ),
        (Mark::PendingChoice, GOLD, "Card pending choice"),
        (Mark::AvailableCommitment, TEAL, "Card available commitment"),
    ] {
        let material = materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let marker = commands
            .spawn((
                Name::new(name),
                Transform::from_translation(Vec3::Y * surface_height),
                Visibility::Hidden,
                Pickable::IGNORE,
                CardMark {
                    card,
                    kind,
                    opacity: 0.0,
                },
                MeshMaterial3d(material.clone()),
            ))
            .id();
        commands.entity(card).add_child(marker);
        for (position, scale) in segments(kind, width, length) {
            let segment = commands
                .spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(position).with_scale(scale),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(marker).add_child(segment);
        }
    }
}

fn segments(kind: Mark, width: f32, length: f32) -> Vec<(Vec3, Vec3)> {
    // All marks lie just outside and above the card face. They never cover text.
    let x = width * 0.5 + 0.045;
    let z = length * 0.5 + 0.045;
    let y = 0.018;
    let thin = 0.025;
    match kind {
        Mark::Hover => vec![
            (Vec3::new(0.0, y, -z), Vec3::new(2.0 * x, thin, thin)),
            (Vec3::new(0.0, y, z), Vec3::new(2.0 * x, thin, thin)),
            (Vec3::new(-x, y, 0.0), Vec3::new(thin, thin, 2.0 * z)),
            (Vec3::new(x, y, 0.0), Vec3::new(thin, thin, 2.0 * z)),
        ],
        Mark::MenuSource => vec![(
            Vec3::new(0.0, y, z + 0.09),
            Vec3::new(width * 0.42, 0.035, 0.055),
        )],
        Mark::PendingChoice => {
            let corner = width.min(length) * 0.22;
            let mut bars = Vec::with_capacity(8);
            for side_x in [-1.0, 1.0] {
                for side_z in [-1.0, 1.0] {
                    let x = x + 0.065;
                    let z = z + 0.065;
                    bars.push((
                        Vec3::new(side_x * (x - corner * 0.5), y, side_z * z),
                        Vec3::new(corner, 0.035, 0.045),
                    ));
                    bars.push((
                        Vec3::new(side_x * x, y, side_z * (z - corner * 0.5)),
                        Vec3::new(0.045, 0.035, corner),
                    ));
                }
            }
            bars
        }
        Mark::AvailableCommitment => vec![
            (
                Vec3::new(-x - 0.06, y, 0.0),
                Vec3::new(0.055, 0.035, length * 0.65),
            ),
            (
                Vec3::new(x + 0.06, y, 0.0),
                Vec3::new(0.055, 0.035, length * 0.65),
            ),
        ],
    }
}

#[allow(clippy::type_complexity)]
fn button_feedback(
    mut buttons: Query<
        (
            Entity,
            &bevy::picking::hover::Hovered,
            Has<Pressed>,
            &mut BackgroundColor,
            &mut Outline,
            Option<&FeedbackOpacity>,
        ),
        With<ContextButton>,
    >,
    focus: Option<Res<InputFocus>>,
) {
    for (entity, hover, pressed, mut background, mut outline, opacity) in &mut buttons {
        background.0 = if pressed {
            GOLD
        } else if hover.0 {
            HOVER
        } else {
            NEUTRAL
        };
        // Focus remains visible on the initial keyboard target, independently of hover.
        outline.color = if focus
            .as_ref()
            .is_some_and(|focus| focus.get() == Some(entity))
        {
            GOLD
        } else {
            Color::NONE
        };
        if let Some(opacity) = opacity {
            background.0 = background.0.with_alpha(background.0.alpha() * opacity.0);
            outline.color = outline.color.with_alpha(outline.color.alpha() * opacity.0);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn card_feedback(
    settings: Res<FeedbackSettings>,
    hover: Option<Res<HoverMap>>,
    windows: Query<&Window>,
    ui: Query<(), With<Node>>,
    cards: Query<&CardFeedback>,
    mut marks: Query<(
        &mut CardMark,
        &MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Option<Res<PresentationTime>>,
    mut redraw: Option<ResMut<Redraw>>,
) {
    let hits = hover.as_ref().and_then(|map| map.get(&PointerId::Mouse));
    let available = !settings.blocked
        && windows
            .iter()
            .any(|window| window.focused && window.cursor_position().is_some())
        && !hits.is_some_and(|hits| hits.keys().any(|entity| ui.contains(*entity)));
    let dt = time.as_ref().map_or(0.0, |time| time.delta_secs());
    for (mut mark, material, mut visibility) in &mut marks {
        let wanted = cards.get(mark.card).is_ok_and(|card| match mark.kind {
            Mark::Hover => available && hits.is_some_and(|hits| hits.contains_key(&mark.card)),
            Mark::MenuSource => card.menu_source,
            Mark::PendingChoice => card.pending_choice,
            Mark::AvailableCommitment => card.commitment.is_some() && !card.pending_choice,
        });
        let opacity = if !wanted {
            0.0
        } else if mark.kind != Mark::Hover || settings.reduced_motion {
            1.0
        } else {
            (mark.opacity + dt / HOVER_FADE_SECONDS).min(1.0)
        };
        *visibility = if opacity > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if opacity != mark.opacity {
            mark.opacity = opacity;
            if let Some(mut material) = materials.get_mut(material) {
                material.base_color.set_alpha(opacity);
            }
        }
        if wanted
            && opacity < 1.0
            && let Some(redraw) = redraw.as_mut()
        {
            redraw.request();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::picking::{backend::HitData, hover::Hovered};
    use std::time::Duration;

    fn test_app() -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<HoverMap>()
            .init_resource::<InputFocus>()
            .init_resource::<PresentationTime>()
            .add_plugins(bevy::transform::TransformPlugin)
            .add_plugins(FeedbackPlugin);
        let mut window = Window {
            focused: true,
            ..default()
        };
        window.set_cursor_position(Some(Vec2::new(30.0, 30.0)));
        let window = app.world_mut().spawn(window).id();
        (app, window)
    }

    fn add_card(app: &mut App) -> Entity {
        let card = app
            .world_mut()
            .spawn(Transform::from_rotation(Quat::from_rotation_x(1.2)))
            .id();
        app.world_mut()
            .resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
                world.resource_scope(|world, mut materials: Mut<Assets<StandardMaterial>>| {
                    let mut commands = world.commands();
                    attach_card_feedback(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        card,
                        2.0,
                        2.8,
                        0.0,
                    );
                });
            });
        app.world_mut().flush();
        card
    }

    fn point_at(app: &mut App, entity: Entity) {
        app.world_mut()
            .resource_mut::<HoverMap>()
            .entry(PointerId::Mouse)
            .or_default()
            .insert(entity, HitData::new(Entity::PLACEHOLDER, 1.0, None, None));
    }

    fn opacity(app: &mut App, kind: Mark) -> f32 {
        app.world_mut()
            .query::<&CardMark>()
            .iter(app.world())
            .find(|mark| mark.kind == kind)
            .unwrap()
            .opacity
    }

    #[test]
    fn keyboard_focus_never_consumes_the_pointer_hover_feedback() {
        let (mut app, _) = test_app();
        let focused = app
            .world_mut()
            .spawn((context_button(), Hovered(false), BackgroundColor(GOLD)))
            .id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(focused, bevy::input_focus::FocusCause::Auto);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(focused).unwrap().0,
            NEUTRAL
        );
        assert_eq!(app.world().get::<Outline>(focused).unwrap().color, GOLD);
        app.world_mut().entity_mut(focused).insert(Hovered(true));
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(focused).unwrap().0,
            HOVER
        );
        assert_eq!(app.world().get::<Outline>(focused).unwrap().color, GOLD);
    }

    #[test]
    fn commitment_availability_is_steady_and_selected_has_distinct_marks() {
        let (mut app, window) = test_app();
        let card = add_card(&mut app);
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(None);
        app.world_mut()
            .get_mut::<CardFeedback>(card)
            .unwrap()
            .commitment = Some(Contribution {
            bonus: 4,
            skill: crate::tabletop::symbols::Skill::Willpower,
        });
        app.update();
        assert_eq!(opacity(&mut app, Mark::AvailableCommitment), 1.0);
        assert_eq!(opacity(&mut app, Mark::Hover), 0.0);
        assert_eq!(opacity(&mut app, Mark::PendingChoice), 0.0);
        assert!(
            app.world_mut()
                .query::<&AccessibleLabel>()
                .iter(app.world())
                .any(|label| label.0 == "+4 willpower")
        );
        app.world_mut()
            .get_mut::<CardFeedback>(card)
            .unwrap()
            .pending_choice = true;
        app.update();
        assert_eq!(opacity(&mut app, Mark::AvailableCommitment), 0.0);
        assert_eq!(opacity(&mut app, Mark::PendingChoice), 1.0);
        assert!(
            app.world_mut()
                .query::<&AccessibleLabel>()
                .iter(app.world())
                .any(|label| label.0 == "Selected · +4 willpower")
        );
        app.world_mut()
            .get_mut::<CardFeedback>(card)
            .unwrap()
            .commitment = None;
        app.update();
        assert_eq!(opacity(&mut app, Mark::AvailableCommitment), 0.0);
        assert_eq!(opacity(&mut app, Mark::PendingChoice), 1.0);
        app.world_mut().despawn(card);
        assert_eq!(
            app.world_mut()
                .query::<&commitment::Badge>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn hover_settles_and_clears_without_erasing_pending_choice_or_menu_source() {
        let (mut app, window) = test_app();
        let card = add_card(&mut app);
        *app.world_mut().get_mut::<CardFeedback>(card).unwrap() = CardFeedback {
            menu_source: true,
            pending_choice: true,
            ..default()
        };
        point_at(&mut app, card);
        app.world_mut()
            .resource_mut::<PresentationTime>()
            .advance_by(Duration::from_millis(60));
        app.update();
        assert!((opacity(&mut app, Mark::Hover) - 0.5).abs() < 0.001);
        app.update();
        assert_eq!(opacity(&mut app, Mark::Hover), 1.0);
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(None);
        app.update();
        assert_eq!(opacity(&mut app, Mark::Hover), 0.0);
        assert_eq!(opacity(&mut app, Mark::MenuSource), 1.0);
        assert_eq!(opacity(&mut app, Mark::PendingChoice), 1.0);
        assert_eq!(segments(Mark::Hover, 2.0, 2.8).len(), 4);
        assert_eq!(segments(Mark::MenuSource, 2.0, 2.8).len(), 1);
        assert_eq!(segments(Mark::PendingChoice, 2.0, 2.8).len(), 8);
    }

    #[test]
    fn reduced_motion_is_static_and_ui_modal_focus_loss_block_hover() {
        let (mut app, window) = test_app();
        let card = add_card(&mut app);
        app.world_mut()
            .resource_mut::<FeedbackSettings>()
            .reduced_motion = true;
        point_at(&mut app, card);
        app.update();
        assert_eq!(opacity(&mut app, Mark::Hover), 1.0);
        let ui = app.world_mut().spawn(Node::default()).id();
        point_at(&mut app, ui);
        app.update();
        assert_eq!(opacity(&mut app, Mark::Hover), 0.0);
        app.world_mut()
            .resource_mut::<HoverMap>()
            .get_mut(&PointerId::Mouse)
            .unwrap()
            .remove(&ui);
        app.world_mut().resource_mut::<FeedbackSettings>().blocked = true;
        app.update();
        assert_eq!(opacity(&mut app, Mark::Hover), 0.0);
        app.world_mut().resource_mut::<FeedbackSettings>().blocked = false;
        app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
        app.update();
        assert_eq!(opacity(&mut app, Mark::Hover), 0.0);
        app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
        app.update();
        assert_eq!(opacity(&mut app, Mark::Hover), 1.0);
    }

    #[test]
    fn tilted_face_marks_follow_the_card_without_replacing_geometry_on_hover() {
        let (mut app, _) = test_app();
        let card = add_card(&mut app);
        app.update();
        let marker_entities = app.world().get::<Children>(card).unwrap().to_vec();
        assert_eq!(marker_entities.len(), 4);
        let pose = Transform::from_xyz(3.0, 1.0, -2.0).with_rotation(Quat::from_rotation_x(1.4));
        app.world_mut().entity_mut(card).insert(pose);
        point_at(&mut app, card);
        app.world_mut()
            .resource_mut::<FeedbackSettings>()
            .reduced_motion = true;
        app.update();
        assert_eq!(
            app.world().get::<Children>(card).unwrap().to_vec(),
            marker_entities
        );
        let normal = pose.rotation * Vec3::Y;
        for marker in marker_entities {
            for segment in app.world().get::<Children>(marker).unwrap() {
                let actual = app
                    .world()
                    .get::<GlobalTransform>(*segment)
                    .unwrap()
                    .translation();
                let local = app.world().get::<Transform>(*segment).unwrap().translation;
                assert!(actual.abs_diff_eq(pose.transform_point(local), 0.0001));
                assert!((actual - pose.translation).dot(normal) > 0.0);
                assert_eq!(
                    app.world().get::<Pickable>(*segment),
                    Some(&Pickable::IGNORE)
                );
            }
        }
        app.world_mut().despawn(card);
        assert_eq!(
            app.world_mut()
                .query::<&CardMark>()
                .iter(app.world())
                .count(),
            0
        );
    }
}
