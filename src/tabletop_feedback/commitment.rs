//! Project a readable commitment badge above the actual card's upper edge.
use super::{CardFeedback, GOLD, TEAL};
use crate::{tabletop::symbols::Skill, tabletop_harm_text};
use bevy::prelude::*;

#[derive(Clone, Copy)]
pub(crate) struct Contribution {
    pub bonus: u8,
    pub skill: Skill,
}

// This is UI rather than 3D child geometry, but still dies with its source card.
#[derive(Component)]
#[relationship(relationship_target = CardBadges)]
pub(super) struct BadgeOf(Entity);

#[derive(Component, Default)]
#[relationship_target(relationship = BadgeOf, linked_spawn)]
pub(super) struct CardBadges(Vec<Entity>);

#[derive(Component)]
pub(super) struct Badge {
    anchor: Vec3,
    shown: Option<(String, Option<Handle<Font>>)>,
}

pub(super) fn attach(commands: &mut Commands, card: Entity, anchor: Vec3) {
    commands.spawn((
        Name::new("Card commitment contribution"),
        BadgeOf(card),
        Badge {
            anchor,
            shown: None,
        },
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(px(7), px(3)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(5)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.035, 0.07, 0.075)),
        BorderColor::all(TEAL),
        Visibility::Hidden,
        Pickable::IGNORE,
    ));
}

#[allow(clippy::type_complexity)]
pub(super) fn badges(
    mut commands: Commands,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    cards: Query<(&CardFeedback, &GlobalTransform)>,
    mut badges: Query<(
        Entity,
        &BadgeOf,
        &mut Badge,
        &ComputedNode,
        &mut Node,
        &mut Visibility,
        &mut BorderColor,
    )>,
) {
    let camera = camera.iter().find(|(camera, _)| camera.is_active);
    for (entity, owner, mut badge, computed, mut node, mut visibility, mut border) in &mut badges {
        *visibility = Visibility::Hidden;
        let Ok((card, transform)) = cards.get(owner.0) else {
            continue;
        };
        let Some(contribution) = card.commitment else {
            continue;
        };
        let label = format!(
            "{}+{} {}",
            if card.pending_choice {
                "Selected · "
            } else {
                ""
            },
            contribution.bonus,
            contribution.skill.name()
        );
        let identity = (label.clone(), card.symbol_font.clone());
        if badge.shown.as_ref() != Some(&identity) {
            commands
                .entity(entity)
                .despawn_children()
                .with_children(|parent| {
                    parent.spawn((
                        tabletop_harm_text::ui(label, 17.0, Color::WHITE, &card.symbol_font),
                        TextLayout::no_wrap(),
                        Pickable::IGNORE,
                    ));
                });
            badge.shown = Some(identity);
        }
        *border = BorderColor::all(if card.pending_choice { GOLD } else { TEAL });
        if let Some((camera, camera_transform)) = camera
            && let Ok(point) =
                camera.world_to_viewport(camera_transform, transform.transform_point(badge.anchor))
        {
            let size = computed.size() * computed.inverse_scale_factor();
            node.left = px(point.x - size.x * 0.5);
            node.top = px(point.y - size.y - 4.0);
            *visibility = Visibility::Inherited;
        }
    }
}
