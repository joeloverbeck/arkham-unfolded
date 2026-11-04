use super::*;

#[test]
fn scenario_pile_captions_and_heading_clear_the_book_at_normal_focus() {
    use crate::tabletop_layout::heading_checks;
    use bevy::camera::primitives::MeshAabb;
    let dir = tempfile::tempdir().unwrap();
    for size in [Vec2::new(1280., 800.), Vec2::new(1600., 1000.)] {
        for (scale, populated) in [(1., false), (2., false), (1., true), (2., true)] {
            let mut app = host(dir.path().join("layout.json"));
            if populated {
                let session = Session::with_hostile_fixture(
                    crate::gathering::HostileFixture::GatheringPriestDefeat,
                );
                let mut play = session.observe().play.unwrap();
                play.scenario.victory.push(crate::gathering::CardView {
                    id: 7000,
                    code: "01118".into(),
                    title: "Flesh-Eater".into(),
                    weakness: false,
                });
                play.encounter_discard.push(crate::gathering::CardView {
                    id: 7001,
                    code: "01162".into(),
                    title: "Rotting Remains".into(),
                    weakness: false,
                });
                let mut table = Table::new(session, dir.path().join("layout.json"));
                table.presentation = vec![PresentationMoment {
                    revealed_story: vec![],
                    revision: 1,
                    sequence: 0,
                    caption: "Populated Scenario layout witness".into(),
                    play,
                }];
                app.insert_resource(table);
            }
            heading_checks::viewport::<MainCamera>(&mut app, size, scale);
            app.world_mut()
                .resource_mut::<Actions>()
                .0
                .push(Action::Focus(Area::Scenario));
            for _ in 0..5 {
                app.update();
            }
            let world = app.world_mut();
            let labels = world
                .query::<(Entity, &WorldLabel, &Text)>()
                .iter(world)
                .filter(|(_, anchor, _)| anchor.0.z < layout::scenario_map_bounds().min.y)
                .map(|(entity, _, text)| {
                    (
                        text.0.clone(),
                        heading_checks::rendered_text_bounds(world, entity, scale),
                    )
                })
                .collect::<Vec<_>>();
            for (index, (left_text, left)) in labels.iter().enumerate() {
                for (right_text, right) in &labels[index + 1..] {
                    assert!(
                        left.max.x + 0.5 <= right.min.x
                            || right.max.x + 0.5 <= left.min.x
                            || left.max.y + 0.5 <= right.min.y
                            || right.max.y + 0.5 <= left.min.y,
                        "{size:?} {scale}x populated={populated}: {left_text:?} {left:?} overlaps {right_text:?} {right:?}"
                    );
                }
            }
            let pages = world
                .query::<(&ContextTarget, &Mesh3d, &GlobalTransform)>()
                .iter(world)
                .filter(|(target, _, _)| {
                    matches!(
                        target.source,
                        controls::Source::Agenda | controls::Source::Act
                    )
                })
                .map(|(target, mesh, transform)| {
                    (target.source.clone(), mesh.0.clone(), *transform)
                })
                .collect::<Vec<_>>();
            assert_eq!(pages.len(), 2);
            let (camera, camera_transform) = world
                .query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>()
                .single(world)
                .unwrap();
            for (source, mesh, transform) in pages {
                let bounds = world
                    .resource::<Assets<Mesh>>()
                    .get(&mesh)
                    .unwrap()
                    .get_aabb()
                    .unwrap();
                let mut page = Rect {
                    min: Vec2::splat(f32::INFINITY),
                    max: Vec2::splat(f32::NEG_INFINITY),
                };
                for x in [-1., 1.] {
                    for y in [-1., 1.] {
                        for z in [-1., 1.] {
                            let corner = Vec3::from(bounds.center)
                                + Vec3::from(bounds.half_extents) * Vec3::new(x, y, z);
                            let point = camera
                                .world_to_viewport(
                                    camera_transform,
                                    transform.transform_point(corner),
                                )
                                .unwrap();
                            page.min = page.min.min(point);
                            page.max = page.max.max(point);
                        }
                    }
                }
                for (text, label) in &labels {
                    assert!(
                        label.max.x + 2. <= page.min.x
                            || page.max.x + 2. <= label.min.x
                            || label.max.y + 2. <= page.min.y
                            || page.max.y + 2. <= label.min.y,
                        "{size:?} {scale}x populated={populated}: {text:?} {label:?} overlaps {source:?} page {page:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn victory_caption_tracks_the_displayed_moment_and_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("victory.json");
    let mut app = host(path.clone());
    assert!(rendered_text(&mut app).contains("0 cards\n0 VP"));
    let mut session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringPriestDefeat);
    session
        .choose(session.observe().revision, "action:ordinary:fight:400")
        .unwrap();
    for _ in 0..8 {
        let view = session.observe();
        if view.choices.iter().any(|c| c.id == "resolution:burn") {
            break;
        }
        let choice = [
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "lucky:decline",
        ]
        .into_iter()
        .find(|id| view.choices.iter().any(|c| c.id == *id))
        .unwrap();
        session.choose(view.revision, choice).unwrap();
    }
    session
        .choose(session.observe().revision, "resolution:burn")
        .unwrap();
    assert_eq!(
        session
            .observe()
            .play
            .unwrap()
            .scenario
            .outcome
            .unwrap()
            .experience_gained,
        [4; 3]
    );
    app.insert_resource(Table::new(session, path.clone()));
    app.update();
    assert!(rendered_text(&mut app).contains("1 cards\n2 VP"));
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .extend([Action::Save, Action::Reopen]);
    app.update();
    assert!(rendered_text(&mut app).contains("1 cards\n2 VP"));
}

#[test]
fn only_eligible_locations_join_the_display_at_scenario_end() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = host(dir.path().join("locations.json"));
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringPriestDefeat);
    let mut play = session.observe().play.unwrap();
    // A public-state witness isolates the RR p21 collection boundary. The
    // actual saved fight path is covered by the test above.
    play.scenario.victory.push(crate::gathering::CardView {
        id: 900,
        code: "01118".into(),
        title: "Flesh-Eater".into(),
        weakness: false,
    });
    play.scenario.victory.push(crate::gathering::CardView {
        id: 901,
        code: "01116".into(),
        title: "Ghoul Priest".into(),
        weakness: false,
    });
    for room in &mut play.scenario.locations {
        if room.code == "01113" {
            room.clues = 0;
        }
        if room.code == "01114" {
            room.clues = 0;
            room.revealed = false;
        }
    }
    for ended in [false, true] {
        if ended {
            play.scenario.outcome = Some(crate::gathering::ResolutionRecord {
                resolution: crate::gathering::Resolution::BurnHouse,
                house_burned: true,
                ghoul_priest_alive: false,
                lita_forced_to_find_others: false,
                experience_gained: [6; 3],
                physical_trauma_gained: [0; 3],
                mental_trauma_gained: [0; 3],
                killed: vec![],
                lita_recipient: None,
                lita_recipient_pending: false,
            });
        }
        let mut table = Table::new(session.clone(), dir.path().join("locations.json"));
        table.presentation = vec![PresentationMoment {
            revealed_story: vec![],
            revision: 1,
            sequence: 0,
            caption: "Victory collection witness".into(),
            play: play.clone(),
        }];
        app.insert_resource(table);
        app.update();
        let caption = if ended {
            "3 cards\n4 VP"
        } else {
            "2 cards\n3 VP"
        };
        assert!(rendered_text(&mut app).contains(caption));
        let view = app.world().resource::<Table>().view();
        assert_eq!(
            view.play.as_ref().unwrap(),
            &play,
            "display must not mutate recorded state"
        );
        assert_eq!(
            controls::exists(&view, &controls::Source::Location("01113".into())),
            !ended
        );
        let pile = inspection_text(&view, "victory").unwrap();
        assert_eq!(pile.contains("\n\nAttic\n"), ended);
        assert!(!pile.contains("\n\nCellar\n"));
        // Collecting the Priest reveals the Act reverse. Use the actual story
        // acknowledgment before inspecting the pile, just as a player must.
        for _ in 0..3 {
            if app.world().resource::<Table>().reading.current().is_none() {
                break;
            }
            app.world_mut()
                .resource_mut::<Actions>()
                .0
                .push(Action::ContinueStory);
            app.update();
        }
        click_source(&mut app, controls::Source::Victory);
        let menu = rendered_text(&mut app);
        assert!(menu.contains("Ghoul Priest") && menu.contains("Flesh-Eater"));
        assert_eq!(menu.contains("Inspect Attic"), ended);
    }
}
