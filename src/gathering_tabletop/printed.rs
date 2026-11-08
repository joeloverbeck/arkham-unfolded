//! Names and doom readouts are ink on their mats, independent of camera projection.
use super::*;
use bevy::{
    camera::RenderTarget, mesh::VertexAttributeValues, render::render_resource::TextureFormat,
};

const NAMES: [&str; 14] = [
    "Scenario",
    "Test",
    "Tony Morgan",
    "Patrice Hathaway",
    "Jacqueline Fine",
    "Threats",
    "Assets",
    "Hand",
    "Deck",
    "Discard",
    "Map",
    "Victory display",
    "Committed cards",
    "Total doom",
];
const WIDTH: u32 = 512;
const ROW: u32 = 64;

#[derive(Component)]
pub(super) struct PrintedLabel;

#[derive(Default)]
pub(super) struct Surfaces(Option<Handle<Image>>);

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
        let doom = doom::readout(context);
        let image = self
            .0
            .get_or_insert_with(|| {
                images.add(Image::new_target_texture(
                    WIDTH,
                    ROW * NAMES.len() as u32,
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
                    order: -20,
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
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|root| {
                for name in NAMES {
                    let (text, size, color) = match name {
                        "Total doom" => (doom.as_deref().unwrap_or(""), 42., PAPER),
                        _ => (name, 40., MUTED),
                    };
                    root.spawn((
                        Node {
                            width: px(WIDTH as f32 / ui_scale),
                            height: px(ROW as f32 / ui_scale),
                            flex_shrink: 0.,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|row| {
                        row.spawn((
                            context.art.text(text, size / ui_scale, color),
                            Pickable::IGNORE,
                        ));
                    });
                }
            });
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(image),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        let mut labels = vec![
            (
                "Scenario",
                Vec3::new(0., 0., layout::SCENARIO_FAR_EDGE - 0.65),
                0.52,
            ),
            ("Test", layout::TEST_CENTER + Vec3::new(0., 0., -5.65), 0.52),
        ];
        labels.extend(layout::scenario_zone_labels().map(|(name, at)| (name, at, 0.34)));
        labels.push(("Committed cards", layout::test_zone_labels()[0].1, 0.34));
        if doom.is_some() {
            labels.push(("Total doom", doom::ALTAR + Vec3::Z * 1.2, 0.64));
        }
        for (index, investigator) in context.view.investigators.iter().enumerate() {
            labels.push((
                investigator.investigator.name(),
                // The mat's upper edge is 7.5 units from its center; the title
                // sits outside it with 0.39 units clear of the label plane.
                layout::investigator_center(index, 3) + Vec3::new(0., 0., -8.15),
                0.52,
            ));
            labels.extend(layout::zone_labels(index).map(|(name, at)| (name, at, 0.34)));
        }
        for (name, mut at, height) in labels {
            let row = NAMES
                .iter()
                .position(|entry| *entry == name)
                .expect("known mat name");
            let mut mesh = Plane3d::default()
                .mesh()
                .size(height * WIDTH as f32 / ROW as f32, height)
                .build();
            if let Some(VertexAttributeValues::Float32x2(uvs)) =
                mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
            {
                for uv in uvs {
                    uv[1] = (row as f32 + uv[1]) / NAMES.len() as f32;
                }
            }
            at.y = 0.071;
            let entity = commands
                .spawn((
                    ScenePiece,
                    PrintedLabel,
                    Name::new(format!("Printed area label: {name}")),
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(at),
                    Pickable::IGNORE,
                ))
                .id();
            if name == "Total doom" {
                commands
                    .entity(entity)
                    .remove::<Pickable>()
                    .insert((
                        Button,
                        bevy::input_focus::tab_navigation::TabIndex(0),
                        AccessibleLabel::new("Inspect total doom"),
                    ))
                    .observe(|_: On<Activate>, mut actions: ResMut<Actions>| {
                        actions
                            .0
                            .push(Action::Inspect("Total doom".into(), "total-doom".into()));
                    });
            }
        }
    }
}
