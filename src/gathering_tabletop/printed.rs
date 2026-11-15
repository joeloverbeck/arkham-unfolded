//! Names and doom readouts are ink on their mats, independent of camera projection.
use super::*;
use unfolded_tabletop::printed::{Atlas, Settings};

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
pub(super) struct Surfaces(Atlas);

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
        let rows = NAMES
            .map(|name| {
                let (text, size, color) = match name {
                    "Total doom" => (doom.as_deref().unwrap_or(""), 42., PAPER),
                    _ => (name, 40., MUTED),
                };
                context.art.text(text, size / ui_scale, color)
            })
            .into();
        let printed = self.0.render(
            commands,
            materials,
            images,
            Settings {
                width: WIDTH,
                row_height: ROW,
                camera_order: -20,
                ui_scale,
            },
            rows,
        );
        commands.entity(printed.camera).insert(ScenePiece);
        commands.entity(printed.root).insert(ScenePiece);
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
            at.y = 0.071;
            let entity = printed.spawn(
                commands,
                meshes,
                row,
                height,
                Transform::from_translation(at),
            );
            commands.entity(entity).insert((
                ScenePiece,
                PrintedLabel,
                Name::new(format!("Printed area label: {name}")),
            ));
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
