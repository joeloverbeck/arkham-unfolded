//! The Gathering table's side of the agent seat (#153): startup assembly and the
//! narrow interface the game-neutral seat reaches the table through. That seat is
//! `unfolded-tabletop`'s `seat`, with its held session's socket and launcher (#175);
//! [`crate::agent_seat`] names it for Arkham. Deleting this file, `agent_seat.rs`,
//! `tabletop_health/seat.rs`, the tools' `seat` command (`tools/src/seat.rs` and
//! `tools/tests/seat.rs`) and the integration points that name them, and asking the
//! shared library for `render` in place of `seat` in `Cargo.toml`, removes the seat.
//!
//! The seat is the same table as native play: the same `install`, rules session,
//! continuity and save path, and artwork verification. Every difference from native
//! startup (`run_window`) is assembled in [`run`] and [`open`], and is exactly:
//!
//! 1. No window event loop: the seat steps updates itself.
//! 2. A logical primary window with no operating-system window behind it.
//! 3. The main camera renders to an image and is the default UI camera.
//! 4. Pointer input names that image as its target.
//! 5. The seat advances the presentation clock in fixed steps, not wall-clock time.
//! 6. Seat-specific rendering admission (`SeatAdmissionPlugin`) replaces native
//!    rendering health, which needs a focused window and is left unchanged.
//! 7. Startup failures are text with an unsuccessful exit, never a dialog and never
//!    the return to scenario selection. Artwork recovery, the artwork-folder chooser
//!    and import completion, which open dialogs in native play, are not installed.
//! 8. The stall watchdog and frame-pacing cadence, which exist for a real display,
//!    do not apply.
//! 9. Sitting marks the table with [`agent_seat::SeatedAgent`], so a Credits link
//!    reports its address instead of reaching the desktop's opener.
//! 10. With `--socket`, the launching command claims the socket and starts the seat as
//!     a detached process of its own ([`agent_seat::socket`]), which performs this same
//!     admission and then serves the socket instead of standard input.

use super::*;
use crate::{
    agent_seat::{
        self, Admission, Focus, Game, Hit, Name, Observed, Seat, SeatKey, refuse, socket,
    },
    tabletop_health::{self, SeatAdmissionPlugin},
    tabletop_pacing::PresentationTime,
    tabletop_support::asset_settings::Collection,
};
use bevy::{camera::primitives::Aabb, ui::CalculatedClip};
use serde_json::json;
use std::time::Duration;

/// The native window's default resolution.
const SIZE: UVec2 = UVec2::new(1600, 1000);
/// Asynchronous artwork loading runs in wall-clock time.
const ADMISSION_LIMIT: Duration = Duration::from_secs(120);

/// An admitted table with an agent seated at it. The artwork lease outlives the app.
pub(super) struct Seated {
    pub(super) app: App,
    pub(super) seat: Seat<Gathering>,
    _artwork: crate::assets::package::ArtworkUse,
}

pub(super) fn run(launch: Result<Option<Options>, String>) -> Result<(), String> {
    let options = match launch.and_then(|options| match options {
        Some(options) if options.agent_seat => Ok(Some(options)),
        Some(_) => Err("--agent-seat is an option of its own, not another option's value".into()),
        None => Ok(None),
    }) {
        Ok(Some(options)) => options,
        Ok(None) => return Ok(()),
        Err(error) => return refuse(error),
    };
    let Some(path) = &options.socket else {
        let mut seated = open(&options).or_else(refuse)?;
        let Seated { app, seat, .. } = &mut seated;
        return seat.serve(app, std::io::stdin().lock(), std::io::stdout().lock());
    };
    if !socket::detached() {
        // The launching command: it returns once the seat it starts is admitted.
        return socket::launch(path).or_else(refuse);
    }
    let listener = socket::detach(path).or_else(refuse)?;
    let mut seated = open(&options).or_else(refuse)?;
    let Seated { app, seat, .. } = &mut seated;
    socket::announce(&seat.ready(app))?;
    listener.serve(seat, app, idle(&options))
}

/// How long a held session waits for a request before it ends as `quit` does.
fn idle(options: &Options) -> Duration {
    options.idle_minutes.map_or(socket::IDLE, |minutes| {
        Duration::from_secs(u64::from(minutes) * 60)
    })
}

/// The same artwork verification and play lease as native startup. Native startup
/// offers recovery dialogs when it fails; the seat refuses once, as text.
fn artwork(
    options: &Options,
) -> Result<
    (
        art::Prepared,
        crate::assets::CollectionReport,
        crate::assets::package::ArtworkUse,
    ),
    String,
> {
    let root = crate::asset_config::resolve_root(options.art_root.clone())?;
    let collection = Collection::Gathering.inspect(&root);
    if !collection.complete || collection.expected_count() == 0 {
        return Err(collection.startup_failure(&root));
    }
    eprintln!(
        "{}\nArtwork root: {}\n{}",
        Collection::Gathering.name(),
        root.display(),
        collection.summary()
    );
    let lease = crate::assets::package::acquire_play(&options.config, &root)?;
    Ok((art::Prepared::new(root)?, collection, lease))
}

pub(super) fn open(options: &Options) -> Result<Seated, String> {
    let (prepared, collection, artwork) = artwork(options).map_err(|reason| {
        format!(
            "{} cannot start play.\nArtwork folder: {}\n\n{reason}\n\nThe agent seat has no artwork recovery: import or choose artwork in the native application. No game decision was made; saves are unchanged.",
            Collection::Gathering.name(),
            options.art_root.display()
        )
    })?;
    let mut app = App::new();
    let mut table = options.open_table()?;
    if let Some(warning) = options.config.warning() {
        table.notice = format!("Asset settings: {warning}");
    }
    app.insert_resource(table);
    options.configure_diagnostics(&mut app)?;
    app.insert_resource(ClearColor(INK))
        .add_plugins(agent_seat::windowless(prepared.asset_plugin()))
        .add_plugins(MeshPickingPlugin)
        .add_plugins(crate::tabletop_chaos_pouch::ChaosPouchPlugin)
        .add_plugins(SeatAdmissionPlugin);
    let art = prepared.load(app.world().resource::<AssetServer>());
    app.insert_resource(art);
    agent_seat::install::<MainCamera>(&mut app, SIZE);
    install(&mut app);
    crate::tabletop_startup::require_artwork(&mut app, &collection);
    let mut seat = Seat::open(Gathering, &mut app)?;
    agent_seat::admit(&mut seat, &mut app, ADMISSION_LIMIT)?;
    Ok(Seated {
        app,
        seat,
        _artwork: artwork,
    })
}

pub(super) struct Gathering;

impl Game for Gathering {
    fn observe(&self, world: &mut World) -> Observed {
        let pending = unsettled(world);
        let components = components(world);
        let controls = controls(world);
        let viewport = viewport(world);
        let table = world.resource::<Table>();
        // The headless player driver's own observation, unchanged. The table can
        // still be presenting its way there, which `shows_latest` discloses.
        let observation = table.session.observe();
        // With agent partners the table offers only the human's options (#205),
        // which is not a presentation still under way.
        let mut shown = table.view();
        shown.choices.clone_from(&observation.choices);
        let shows_latest = shown == observation;
        let control = |control: &Control| {
            json!({
                "choice": control.choice,
                "label": control.label,
                "position": [control.position.x, control.position.y],
                "on_screen": control.on_screen,
                "focused": control.focused,
            })
        };
        let menu = table.context.is_open().then(|| {
            json!({
                "component": table.context.selection().map(|selection| identity(&selection.source)),
                "title": table.context.selection().map_or("Current decision", |selection| selection.title.as_str()),
                "rows": controls.iter().filter(|c| c.menu).map(control).collect::<Vec<_>>(),
            })
        });
        let table = json!({
            "settled": pending.is_empty(),
            "pending": pending,
            "shows_latest": shows_latest,
            "viewport": viewport.map(|size| [size.x, size.y]),
            "modal": modal(world),
            "notice": (!table.notice.is_empty()).then(|| table.notice.clone()),
            "components": components.iter().map(|component| json!({
                "name": component.name,
                "label": component.label,
                "position": component.position.map(|at| [at.x, at.y]),
                "on_screen": component.on_screen,
                "available": component.available,
            })).collect::<Vec<_>>(),
            "menu": menu,
            "controls": controls.iter().filter(|c| !c.menu).map(control).collect::<Vec<_>>(),
        });
        Observed {
            observation: serde_json::to_value(observation).expect("An observation serializes"),
            table,
        }
    }

    fn revision(&self, world: &mut World) -> u64 {
        world.resource::<Table>().session.observe().revision
    }

    fn resolve(&self, world: &mut World, name: &Name) -> Result<Vec<Vec2>, String> {
        let table = world.resource::<Table>();
        // Production accepts a selection only from a piece built for this table state.
        let shown_state = (table.epoch, table.view().revision);
        let (position, reach) = match name {
            Name::Component(wanted) => {
                let mut shown = components(world)
                    .into_iter()
                    .filter(|component| &component.name == wanted);
                let Some(component) = shown.next() else {
                    return Err(format!("No shown component is named {wanted}."));
                };
                let others = shown.count();
                if others > 0 {
                    return Err(format!(
                        "{} shown components are named {wanted}; the name is ambiguous.",
                        others + 1
                    ));
                }
                if component.stamp != shown_state {
                    return Err(format!(
                        "{wanted} is shown from an earlier table state. Settle and observe again."
                    ));
                }
                (component.position, component.reach)
            }
            Name::Choice(wanted) => {
                let mut offered: Vec<_> = controls(world)
                    .into_iter()
                    .filter(|control| control.choice.as_ref() == Some(wanted))
                    .collect();
                // An open contextual menu captures the pointer, so a choice it
                // shares with the decision bar is the menu's row.
                if offered.iter().any(|control| control.menu) {
                    offered.retain(|control| control.menu);
                }
                let mut shown = offered.into_iter();
                let Some(control) = shown.next() else {
                    return Err(format!("No shown control offers choice {wanted}."));
                };
                let others = shown.count();
                if others > 0 {
                    return Err(format!(
                        "{} shown controls offer choice {wanted}; the name is ambiguous.",
                        others + 1
                    ));
                }
                let reach = if control.on_screen {
                    vec![control.position]
                } else {
                    Vec::new()
                };
                (Some(control.position), reach)
            }
        };
        match position {
            _ if !reach.is_empty() => Ok(reach),
            Some(position) => Err(format!(
                "{} is off screen at ({:.0}, {:.0}); bring it into view first.",
                wanted_name(name),
                position.x,
                position.y
            )),
            None => Err(format!(
                "{} is behind the camera; bring it into view first.",
                wanted_name(name)
            )),
        }
    }

    fn identify(&self, world: &mut World, entity: Entity) -> Hit {
        let mut current = Some(entity);
        while let Some(ancestor) = current {
            if let Some(choice) = world.get::<ChoiceControl>(ancestor) {
                return Hit {
                    name: Some(Name::Choice(choice.0.clone())),
                    description: json!({"choice": choice.0, "label": control_label(world, ancestor)}),
                };
            }
            if let Some(target) = world.get::<ContextTarget>(ancestor) {
                let name = component_name(world, ancestor, &target.source);
                return Hit {
                    description: json!({
                        "component": name,
                        "label": component_label(world, ancestor, &target.source),
                    }),
                    name: Some(Name::Component(name)),
                };
            }
            if world.get::<Button>(ancestor).is_some() {
                return Hit {
                    name: None,
                    description: json!({"control": control_label(world, ancestor)}),
                };
            }
            current = world.get::<ChildOf>(ancestor).map(ChildOf::parent);
        }
        let description = if world.get::<ComputedNode>(entity).is_some() {
            let surface = if has_ancestor::<controls::Menu>(world, entity) {
                "contextual menu"
            } else if has_ancestor::<Panel>(world, entity) {
                "panel"
            } else {
                "overlay"
            };
            json!({"surface": surface, "text": first_text(world, entity)})
        } else {
            json!({"piece": world.get::<bevy::prelude::Name>(entity).map_or_else(
                || if world.get::<controls::Backdrop>(entity).is_some() { "table".to_owned() } else { "unnamed".to_owned() },
                |name| name.to_string(),
            )})
        };
        Hit {
            name: None,
            description,
        }
    }

    fn unsettled(&self, world: &mut World) -> Vec<String> {
        unsettled(world).into_iter().map(str::to_owned).collect()
    }

    /// The seat alone advances the presentation clock (difference 5 above).
    fn advance(&self, world: &mut World, by: Duration) {
        world.resource_mut::<PresentationTime>().advance_by(by);
    }

    /// Seat admission (difference 6 above), as its rendering health reports it.
    fn admission(&self, world: &World) -> Admission {
        tabletop_health::admission(world)
    }

    fn keys(&self) -> &[SeatKey] {
        &agent_seat::KEYS
    }

    fn pipelines_ready(&self, world: &World) -> bool {
        tabletop_health::pipelines_ready(world)
    }

    /// The focused control or menu row exactly as `controls` and `menu.rows` report
    /// it; any other focused entity is described as a pointer hit would be.
    fn focus(&self, world: &mut World) -> Focus {
        let Some(entity) = world
            .get_resource::<InputFocus>()
            .and_then(InputFocus::get)
            .filter(|entity| world.get_entity(*entity).is_ok())
        else {
            return Focus::Nothing;
        };
        match controls(world).into_iter().find(|control| control.focused) {
            Some(control) => Focus::Control {
                choice: control.choice,
                label: control.label,
            },
            None => Focus::Other(self.identify(world, entity).description),
        }
    }
}

fn wanted_name(name: &Name) -> &str {
    match name {
        Name::Component(name) | Name::Choice(name) => name,
    }
}

/// The presentation's existing source identity, as stable text.
fn identity(source: &controls::Source) -> String {
    use controls::Source;
    match source {
        Source::Investigator(who) => format!("investigator:{}", who.id()),
        Source::Card(id) => format!("card:{id}"),
        Source::Location(code) => format!("location:{code}"),
        Source::Deck(who) => format!("deck:{}", who.id()),
        Source::Discard(who) => format!("discard:{}", who.id()),
        Source::Removed(who) => format!("removed:{}", who.id()),
        Source::Act => "act".into(),
        Source::Agenda => "agenda".into(),
        Source::Bag => "chaos-bag".into(),
        Source::Token(index) => format!("token:{index}"),
        Source::EncounterDeck => "encounter-deck".into(),
        Source::EncounterDiscard => "encounter-discard".into(),
        Source::Victory => "victory".into(),
        Source::ScenarioRemoved => "scenario-removed".into(),
        Source::SearchPlayer => "searched-player".into(),
        Source::SearchEncounter => "searched-encounter".into(),
        Source::Resolving => "resolving-cards".into(),
        Source::PendingDiscard => "pending-discard".into(),
        Source::Backpack(id) => format!("backpack:{id}"),
        Source::Lita => "lita".into(),
        Source::ScenarioReference => "scenario-reference".into(),
    }
}

/// One source can be shown by several pieces: an investigator by a card and a standee,
/// a location by its card, clue counter, and each sideboard's marker and heading. The
/// card keeps the bare identity; each other piece adds what it is, so every name is exact.
fn component_name(world: &World, entity: Entity, source: &controls::Source) -> String {
    let identity = identity(source);
    let link = world
        .get::<sideboard::Link>(entity)
        .map(sideboard::Link::board);
    let piece = if let Some(board) = link {
        Some(
            if world.get::<Node>(entity).is_some() {
                board.heading_piece
            } else {
                board.members
            }
            .to_owned(),
        )
    } else if world.get::<Node>(entity).is_some() {
        Some("board".to_owned())
    } else if let Some(counter) = world.get::<crate::tabletop_counters::Counter>(entity) {
        Some(counter.kind.name().to_lowercase())
    } else if matches!(
        world.get::<map_motion::Surface>(entity),
        Some(map_motion::Surface::Investigator(_))
    ) {
        Some("standee".to_owned())
    } else {
        None
    };
    piece.map_or(identity.clone(), |piece| format!("{identity}/{piece}"))
}

fn component_label(world: &World, entity: Entity, source: &controls::Source) -> String {
    world
        .get::<AccessibleLabel>(entity)
        .map(|label| label.0.clone())
        .or_else(|| {
            world
                .get::<bevy::prelude::Name>(entity)
                .map(|name| name.to_string())
        })
        .unwrap_or_else(|| identity(source))
}

fn control_label(world: &World, entity: Entity) -> String {
    world
        .get::<AccessibleLabel>(entity)
        .map(|label| label.0.clone())
        .or_else(|| first_text(world, entity))
        .unwrap_or_default()
}

fn first_text(world: &World, entity: Entity) -> Option<String> {
    if let Some(text) = world.get::<Text>(entity) {
        return Some(text.0.clone());
    }
    world
        .get::<Children>(entity)?
        .iter()
        .find_map(|child| first_text(world, child))
}

fn has_ancestor<C: Component>(world: &World, entity: Entity) -> bool {
    let mut current = Some(entity);
    while let Some(ancestor) = current {
        if world.get::<C>(ancestor).is_some() {
            return true;
        }
        current = world.get::<ChildOf>(ancestor).map(ChildOf::parent);
    }
    false
}

fn viewport(world: &mut World) -> Option<Vec2> {
    world
        .query_filtered::<&Camera, With<MainCamera>>()
        .single(world)
        .ok()
        .and_then(Camera::logical_viewport_size)
}

fn within(viewport: Option<Vec2>, at: Vec2) -> bool {
    viewport.is_some_and(|size| at.cmpge(Vec2::ZERO).all() && at.cmple(size).all())
}

struct Piece {
    name: String,
    label: String,
    /// The projected middle of the piece, when it is in front of the camera.
    position: Option<Vec2>,
    on_screen: bool,
    /// On-screen points on the piece where a pointer could rest, middle first.
    reach: Vec<Vec2>,
    available: bool,
    stamp: (u64, u64),
}

/// Every shown piece that selects a source, sorted by name.
fn components(world: &mut World) -> Vec<Piece> {
    let Ok((camera, camera_at)) = world
        .query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>()
        .single(world)
        .map(|(camera, at)| (camera.clone(), *at))
    else {
        return Vec::new();
    };
    let viewport = camera.logical_viewport_size();
    let bound: Vec<Entity> = world
        .query_filtered::<Entity, With<ContextTarget>>()
        .iter(world)
        .collect();
    let world = &*world;
    let mut shown: Vec<Piece> = bound
        .into_iter()
        .filter(|entity| shown(world, *entity))
        .map(|entity| {
            let target = world.get::<ContextTarget>(entity).expect("A bound piece");
            let points: Vec<Option<Vec2>> = match ui_position(world, entity) {
                Some(position) => vec![Some(position)],
                None => world_reach(world, entity)
                    .into_iter()
                    .map(|point| camera.world_to_viewport(&camera_at, point).ok())
                    .collect(),
            };
            let position = points[0];
            Piece {
                name: component_name(world, entity, &target.source),
                label: component_label(world, entity, &target.source),
                position,
                on_screen: position.is_some_and(|at| within(viewport, at)),
                reach: points
                    .into_iter()
                    .flatten()
                    .filter(|at| within(viewport, *at))
                    .collect(),
                available: world
                    .get::<CardFeedback>(entity)
                    .is_some_and(|feedback| feedback.opportunity),
                stamp: (target.epoch, target.revision),
            }
        })
        .collect();
    shown.sort_by(|a, b| a.name.cmp(&b.name));
    shown
}

fn shown(world: &World, entity: Entity) -> bool {
    world
        .get::<InheritedVisibility>(entity)
        .is_none_or(|visibility| visibility.get())
}

/// The logical center of a UI node, where a player would press it.
fn ui_position(world: &World, entity: Entity) -> Option<Vec2> {
    let node = world.get::<ComputedNode>(entity)?;
    let transform = world.get::<UiGlobalTransform>(entity)?;
    Some(transform.translation * node.inverse_scale_factor)
}

/// World points on what a pointer can hit of this piece: the middle of its own and
/// its descendants' shown, pickable meshes first, then points toward its edges. A
/// counter or standee often rests on a card's middle; a player presses beside it.
fn world_reach(world: &World, entity: Entity) -> Vec<Vec3> {
    fn extend(world: &World, entity: Entity, bounds: &mut Option<(Vec3, Vec3)>) {
        let hoverable = world
            .get::<Pickable>(entity)
            .is_none_or(|pickable| pickable.is_hoverable);
        if hoverable
            && shown(world, entity)
            && let (Some(aabb), Some(transform)) = (
                world.get::<Aabb>(entity),
                world.get::<GlobalTransform>(entity),
            )
        {
            for corner in 0..8 {
                let sign = Vec3::new(
                    if corner & 1 == 0 { -1. } else { 1. },
                    if corner & 2 == 0 { -1. } else { 1. },
                    if corner & 4 == 0 { -1. } else { 1. },
                );
                let point = transform.transform_point(
                    Vec3::from(aabb.center) + Vec3::from(aabb.half_extents) * sign,
                );
                *bounds = Some(bounds.map_or((point, point), |(min, max)| {
                    (min.min(point), max.max(point))
                }));
            }
        }
        if let Some(children) = world.get::<Children>(entity) {
            for child in children.iter() {
                extend(world, child, bounds);
            }
        }
    }
    let mut bounds = None;
    extend(world, entity, &mut bounds);
    let Some((min, max)) = bounds else {
        return vec![
            world
                .get::<GlobalTransform>(entity)
                .map_or(Vec3::ZERO, GlobalTransform::translation),
        ];
    };
    let (middle, half) = ((min + max) / 2., (max - min) / 2.);
    [
        (0., 0.),
        (-1., 0.),
        (1., 0.),
        (0., -1.),
        (0., 1.),
        (-1., -1.),
        (1., -1.),
        (-1., 1.),
        (1., 1.),
    ]
    .into_iter()
    .map(|(x, z)| middle + Vec3::new(x * half.x * 0.6, 0., z * half.z * 0.6))
    .collect()
}

struct Control {
    choice: Option<String>,
    label: String,
    position: Vec2,
    on_screen: bool,
    focused: bool,
    menu: bool,
}

/// Every shown button except those that are table components, in reading order.
fn controls(world: &mut World) -> Vec<Control> {
    let viewport = viewport(world);
    let buttons: Vec<Entity> = world
        .query_filtered::<Entity, (With<Button>, Without<ContextTarget>)>()
        .iter(world)
        .collect();
    let world = &*world;
    let focus = world
        .get_resource::<InputFocus>()
        .and_then(|focus| focus.get());
    let mut shown: Vec<Control> = buttons
        .into_iter()
        .filter_map(|entity| {
            let node = world.get::<ComputedNode>(entity)?;
            let transform = world.get::<UiGlobalTransform>(entity)?;
            if node.is_empty()
                || !world
                    .get::<InheritedVisibility>(entity)
                    .is_some_and(|visibility| visibility.get())
            {
                return None;
            }
            let position = transform.translation * node.inverse_scale_factor;
            let clipped = world
                .get::<CalculatedClip>(entity)
                .is_some_and(|clip| !clip.contains_point(transform.translation));
            Some(Control {
                choice: world
                    .get::<ChoiceControl>(entity)
                    .map(|choice| choice.0.clone()),
                label: control_label(world, entity),
                position,
                on_screen: !clipped && within(viewport, position),
                focused: focus == Some(entity),
                menu: has_ancestor::<controls::Menu>(world, entity),
            })
        })
        .collect();
    shown.sort_by(|a, b| {
        (a.position.y, a.position.x)
            .partial_cmp(&(b.position.y, b.position.x))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    shown
}

/// The surface that currently captures the table, in the order the screen draws them.
fn modal(world: &World) -> Option<&'static str> {
    let table = world.resource::<Table>();
    if !world.resource::<RenderHealth>().allows_play() {
        Some("checking graphics")
    } else if table.presentation.result().is_some() {
        Some("test result")
    } else if table.prediction.is_some() {
        Some("pendulum prediction")
    } else if table.restart_confirmation {
        Some("restart confirmation")
    } else if table.presentation.story().is_some() {
        Some("story reading")
    } else if table.presentation.encounter().current().is_some() {
        Some("encounter reading")
    } else if table.result_opportunities() {
        Some("result opportunities")
    } else if table.inspection.is_some() {
        Some("inspection")
    } else if table.history_open {
        Some("history")
    } else if table.violin.is_some() {
        Some("violin choice")
    } else if table.clue_payment.is_some() {
        Some("group clue payment")
    } else if table.menu_open {
        Some("utilities menu")
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
