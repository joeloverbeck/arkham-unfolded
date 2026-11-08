//! Slot text is rendered onto the mat, so perspective and zoom affect it naturally.
use super::*;
use bevy::{camera::RenderTarget, render::render_resource::TextureFormat};

#[derive(Default)]
pub(super) struct Surfaces([Option<Handle<Image>>; 3]);

impl Surfaces {
    pub(super) fn render(
        &mut self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        images: &mut Assets<Image>,
        context: &CardRender,
        ui_scale: f32,
    ) {
        let Some(play) = &context.view.play else {
            return;
        };
        for player in &play.players {
            let mut entries = player
                .slot_usage()
                .into_iter()
                .filter(|slot| slot.used != 0 || slot.spell_ritual != 0)
                .map(|slot| slot.label())
                .collect::<Vec<_>>();
            let masks = player.masks_controlled();
            if masks != 0 {
                entries.push(format!("Mask: {masks}/1"));
            }
            let text = entries.join(" | ");
            if text.is_empty() {
                continue;
            }
            let index = player.investigator.index();
            let image = self.0[index]
                .get_or_insert_with(|| {
                    images.add(Image::new_target_texture(
                        2048,
                        60,
                        TextureFormat::Bgra8UnormSrgb,
                        None,
                    ))
                })
                .clone();
            let camera = commands
                .spawn((
                    ScenePiece,
                    Camera2d,
                    Camera {
                        order: -10 - index as isize,
                        clear_color: ClearColorConfig::Custom(Color::NONE),
                        ..default()
                    },
                    RenderTarget::Image(image.clone().into()),
                ))
                .id();
            commands
                .spawn((
                    ScenePiece,
                    UiTargetCamera(camera),
                    Node {
                        width: percent(100),
                        height: percent(100),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|p| {
                    p.spawn((
                        context.art.text(text, 36. / ui_scale, MUTED),
                        TextLayout {
                            justify: Justify::Center,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                });
            commands.spawn((
                ScenePiece,
                Name::new("Asset slot strip"),
                Mesh3d(meshes.add(Plane3d::default().mesh().size(16.4, 0.48))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color_texture: Some(image),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                })),
                Transform::from_translation(
                    layout::investigator_center(index, 3) + Vec3::new(0., 0.071, 3.475),
                ),
                Pickable::IGNORE,
            ));
        }
    }
}
