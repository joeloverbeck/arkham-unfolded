use super::*;
pub(super) fn button(
    art: &Art,
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: Action,
) -> Entity {
    parent
        .spawn((
            Button,
            Hovered::default(),
            TabIndex(0),
            AccessibleLabel::new(label),
            Node {
                width: percent(100),
                min_height: px(38),
                padding: UiRect::all(px(8)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_shrink: 0.,
                ..default()
            },
            feedback::context_button(),
        ))
        .observe(move |_: On<Activate>, mut queue: ResMut<Actions>| queue.0.push(action.clone()))
        .with_child((art.text(label, 14., PAPER), Pickable::IGNORE))
        .id()
}
#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    mut commands: Commands,
    mut table: ResMut<Table>,
    art: Res<Art>,
    orbit: Res<Orbit>,
    panels: Query<Entity, With<Panel>>,
    window: Single<&Window>,
    images: Res<Assets<Image>>,
    mut last: Local<(Vec2, [bool; 3])>,
) {
    let size = Vec2::new(window.width(), window.height());
    let portraits = ["06003", "06005", "60401"].map(|code| {
        art.image(&format!("standee/{code}")).and_then(|handle| {
            images
                .get(&handle)
                .map(|image| (handle.clone(), image.size_f32()))
        })
    });
    let readiness = portraits.each_ref().map(Option::is_some);
    if last.0 != size || last.1 != readiness {
        *last = (size, readiness);
        table.dirty = true;
    }
    if !table.dirty {
        return;
    }
    table.dirty = false;
    for entity in &panels {
        commands.entity(entity).despawn();
    }
    let view = table.view();
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(16),
                top: px(10),
                right: px(130),
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                ..default()
            },
            Pickable::IGNORE,
            Panel,
        ))
        .with_children(|p| {
            p.spawn(art.text(
                "Prepared Study escape · The Gathering · Standard",
                20.,
                GOLD,
            ));
            p.spawn(
                art.text(
                    view.investigators
                        .iter()
                        .map(|i| {
                            format!(
                                "{}: {} resources · {} clues · {} actions{}",
                                i.investigator
                                    .name()
                                    .split_whitespace()
                                    .next()
                                    .unwrap_or("Investigator"),
                                i.resources,
                                i.clues,
                                i.ordinary_actions,
                                if i.turn_finished { " (turn ended)" } else { "" }
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("     |     "),
                    13.,
                    PAPER,
                ),
            );
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(12),
                top: px(12),
                width: px(108),
                ..default()
            },
            Panel,
        ))
        .with_children(|p| {
            button(&art, p, "Menu", Action::Utilities);
        });
    commands.spawn((Node{position_type:PositionType::Absolute,left:px(10),top:px(76),bottom:px(220),width:px(124),flex_direction:FlexDirection::Column,row_gap:px(7),overflow:Overflow::scroll_y(),..default()},Pickable::IGNORE,Panel)).observe(scroll_panel).with_children(|p|{
        for (index,who) in Investigator::ALL.into_iter().enumerate(){
            if let Some((image,dimensions))=&portraits[index]{
                let dimensions=*dimensions*(108./dimensions.x).min(82./dimensions.y);
                p.spawn((Button,Hovered::default(),TabIndex(0),AccessibleLabel::new(format!("Focus {}",who.name())),Node{width:px(dimensions.x+10.),height:px(dimensions.y+10.),padding:UiRect::all(px(5)),align_self:AlignSelf::Center,flex_shrink:0.,..default()},feedback::context_button()))
                    .observe(move|_:On<Activate>,mut queue:ResMut<Actions>|queue.0.push(Action::Focus(Area::Investigator(index))))
                    .with_child((ImageNode::new(image.clone()),Node{width:px(dimensions.x),height:px(dimensions.y),..default()},Pickable::IGNORE));
            }else{button(&art,p,who.name(),Action::Focus(Area::Investigator(index)));}
        }
        for (name,area) in [("Scenario",Area::Scenario),("Test",Area::Test),("Overview",Area::Overview)]{button(&art,p,name,Action::Focus(area));}
        p.spawn(art.text("WASD: pan\nRight drag: orbit\nWheel: zoom\nHome: overview\n\nFocus does not select who acts.",11.,MUTED));
    });
    commands.spawn((Node{position_type:PositionType::Absolute,left:px(150),right:px(12),bottom:px(10),max_height:px((size.y*0.29).min(270.)),padding:UiRect::all(px(12)),flex_direction:FlexDirection::Column,row_gap:px(5),overflow:Overflow::scroll_y(),..default()},BackgroundColor(INK),Panel)).observe(scroll_panel).with_children(|p|{
        p.spawn(art.text(table.replay.as_ref().map_or_else(||view.actor.map_or("CHOOSE THE NEXT INVESTIGATOR".into(),|who|format!("{} · LIVE DECISION",who.name())),|r|format!("RECORDED REPLAY · MOMENT {} / {}",r.index()+1,r.len())),12.,GOLD));
        p.spawn(art.text(&view.prompt,18.,PAPER));
        if let Some(endpoint)=view.endpoint {
            let explanation=match endpoint {
                crate::party::Endpoint::Escaped => "Escaped the Study. The group spent six clues; all three investigators are in revealed Hallway, Study is removed, and Act 2 is now in play. This prepared attempt stops before further actions. Unfinished turns and remaining actions are preserved.",
                crate::party::Endpoint::Declined => "Finished without advancing. The group retains all six clues and remains in Study. Unfinished turns and remaining actions are preserved; this is neither defeat nor a scenario resolution.",
                crate::party::Endpoint::TurnsEnded => "All three turns ended before the group gathered six clues. The attempt stops before the enemy phase, with the resulting Study position preserved. This is neither defeat nor a scenario resolution.",
            };
            p.spawn(art.text(explanation,17.,GOLD));
        }
        if view.revision==0{
            p.spawn(art.text("Prepared position: all three investigators begin in Study with zero resources and six clues to discover. Choose their turn order. Each gets one turn with three ordinary actions. At the first settled action leaving six clues held, choose to spend them and escape or finish without advancing. If all turns finish first, stop before the enemy phase. The endpoint preserves unfinished turns; this is not scenario setup or a complete scenario.",14.,MUTED));
        }
        if table.show_test_summary(&view) && let Some(test)=&view.test{
            let total=test.total.map_or(String::new(),|total|format!(" · final value {total}"));
            p.spawn(art.text(format!("{} · Intellect {} · modifier {:+}{total} · difficulty {} · {}",test.investigator.name(),test.skill_value,test.modifier,test.difficulty,test.result.map_or("Pending",|success|if success{"Success"}else{"Failure"})),15.,GOLD));
        }
        p.spawn(Node{flex_direction:FlexDirection::Row,flex_wrap:FlexWrap::Wrap,column_gap:px(8),row_gap:px(5),..default()}).with_children(|row|{
            let mut control=|label:&str,action:Action|{row.spawn(Node{width:px(240),..default()}).with_children(|p|{button(&art,p,label,action);});};
            if let Some(replay)=&table.replay{
                if replay.index()>0{control("Previous moment",Action::Back);}
                if replay.index()+1<replay.len(){control("Next moment",Action::Next);}
                control("Return to live attempt",Action::Live);
            }else{
                for choice in &view.choices{if choice.id=="commit:none"||!controls::has_source(&choice.id){control(&choice.label,Action::Choose(view.revision,choice.id.clone()));}}
                for cancel in [false, true] {
                    if let Some((label, action)) = table.context.confirmation_control(&view, cancel) {
                        control(&label, action);
                    }
                }
                if view.endpoint.is_some(){control("Start another attempt",Action::New);}
            }
        });
        if let Some(summary) = table.context.pending_summary(&view) {
            p.spawn(art.text(summary,13.,GOLD));
        }
        if table.replay.is_none()&&view.endpoint.is_none(){
            let hint=if view.actor.is_none(){"Select an investigator card, then choose their turn. Portrait buttons only move the camera."}else if view.choices.iter().any(|c|c.id.starts_with("commit:")){"Select hand cards across the investigator areas; confirm selected cards, or commit none."}else if view.choices.iter().any(|c|c.id.starts_with("cancel:")){"Select revealed tokens in the Test area, then confirm your legal cancellation."}else if view.choices.iter().any(|c|c.id.starts_with("resolve:")){"Select a revealed token in the Test area to choose the next token to resolve."}else if view.choices.iter().any(|c|c.id.starts_with("lucky:")){"Select Lucky! in Patrice's hand to pay one resource, or decline below."}else if view.choices.iter().any(|c|c.id.starts_with("shuffle:retain:")){"Select Patrice's discard to choose the card retained, or decline the shuffle."}else if view.choices.iter().any(|c|c.id=="advance"){"Select Act 1 to spend six clues and escape, or finish without advancing below."}else{"Select Study to Investigate, your investigator to gain a resource, or your deck to Draw. Use the reveal shortcut when offered."};
            p.spawn(art.text(hint,12.,MUTED));
        }
        if !table.notice.is_empty(){p.spawn(art.text(&table.notice,12.,MUTED));}
    });
    if table.context.utilities_open() {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(12),
                    top: px(62),
                    width: px(305),
                    max_height: px((size.y - 240.).max(150.)),
                    padding: UiRect::all(px(12)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(7),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(INK),
                GlobalZIndex(45),
                Panel,
            ))
            .observe(scroll_panel)
            .with_children(|p| {
                button(&art, p, "Action history and rules reasons", Action::History);
                button(
                    &art,
                    p,
                    if table.reduced_motion {
                        "Motion: reduced"
                    } else {
                        "Motion: animated"
                    },
                    Action::Motion,
                );
                if table.replay.is_none() {
                    button(&art, p, "Save live attempt", Action::Save);
                    button(&art, p, "Reopen saved attempt", Action::Reopen);
                    button(&art, p, "Open recorded replay", Action::Replay);
                } else {
                    button(
                        &art,
                        p,
                        if orbit.is_following() {
                            "Camera following: on"
                        } else {
                            "Resume camera following"
                        },
                        Action::Follow,
                    );
                }
                p.spawn(art.text(&art.status, 12., MUTED));
                button(&art, p, "Choose asset folder", Action::AssetFolder);
                button(&art, p, "Close menu", Action::Dismiss);
            });
    }
    if table.modal() {
        inspection(&mut commands, &table, &view, &art, size);
    }
}
fn inspection(commands: &mut Commands, table: &Table, view: &Observation, art: &Art, size: Vec2) {
    let width = (size.x * 0.94).min(1200.);
    let height = size.y * 0.9;
    let card = table.inspect.map(|i| &CARDS[i]);
    commands.spawn((Node{position_type:PositionType::Absolute,width:percent(100),height:percent(100),align_items:AlignItems::Center,justify_content:JustifyContent::Center,..default()},BackgroundColor(Color::srgba(0.015,0.025,0.03,0.88)),GlobalZIndex(90),ModalDialogBarrier,Panel)).observe(|_:On<RequestClose>,mut queue:ResMut<Actions>|queue.0.push(Action::Close)).with_children(|overlay|{
        overlay.spawn((Node{width:px(width),max_height:px(height),padding:UiRect::all(px(22)),flex_direction:FlexDirection::Column,row_gap:px(16),..default()},BackgroundColor(INK),ModalDialog,TabGroup::modal())).with_children(|p|{
            p.spawn(art.text(card.map_or("Action history",|c|c.title),27.,GOLD));button(art,p,"Close · Esc",Action::Close);
            p.spawn((Node{max_height:px((height-150.).max(100.)),overflow:Overflow::scroll_y(),flex_direction:FlexDirection::Column,align_items:AlignItems::Center,row_gap:px(16),..default()},ScrollPosition::default())).observe(scroll_panel).with_children(|body|{
                if let Some(card)=card{
                    if let Some(image)=art.image(card.role){let w=(width-55.).min(if card.landscape{760.}else{430.});let h=w*if card.landscape{5./7.}else{7./5.};
                        body.spawn(Node{width:px(w),height:px(h),flex_shrink:0.,..default()}).with_children(|frame|{frame.spawn((ImageNode::new(image),Node{position_type:PositionType::Absolute,left:percent(50),top:percent(50),width:px(if card.landscape{h}else{w}),height:px(if card.landscape{w}else{h}),..default()},UiTransform{translation:Val2::percent(-50,-50),rotation:if card.landscape{Rot2::degrees(-90.)}else{Rot2::IDENTITY},..default()},Pickable::IGNORE));});
                    }
                    body.spawn((art.text(card.kind,18.,GOLD),Node{width:percent(100),..default()}));body.spawn((art.text(card.text,20.,PAPER),Node{width:percent(100),..default()}));
                    if table.replay.is_none(){
                        for held in view.investigators.iter().flat_map(|i|&i.hand).filter(|c|card.role==format!("card/{}",c.code)){
                            if let Some(reason)=view.unavailable.iter().find(|choice|choice.id==format!("play:{}",held.id)){
                                body.spawn((art.text(format!("Unavailable now: {}",reason.label),16.,GOLD),Node{width:percent(100),..default()}));
                            }
                        }
                    }
                }else{
                    for line in &view.history{body.spawn((art.text(line,18.,PAPER),Node{width:percent(100),..default()}));}
                    if !view.unavailable.is_empty(){
                        body.spawn(art.text("Unavailable at this decision",18.,GOLD));
                        for choice in &view.unavailable{body.spawn((art.text(&choice.label,16.,MUTED),Node{width:percent(100),..default()}));}
                    }
                    body.spawn((art.text("Rules: pinned Chapter One Rules Reference (acts p3, clues p7, locations p14, investigation windows pp23–24, skill tests p26); original Gathering Standard reference and selected card faces; Legacy FAQ 2.5; Innsmouth curse rules. Scope and exact source identities: GitHub #55.",14.,MUTED),Node{width:percent(100),..default()}));
                }
            });
        });
    });
}
