//! Exercise the actual Bevy input system without a window, renderer or GPU.
use super::*;
use bevy::{
    camera::{CameraProjection, ComputedCameraValues, RenderTargetInfo},
    input::mouse::MouseButtonInput,
    window::CursorMoved,
};

fn app() -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(Table::new(6, DrawMode::One))
        .init_resource::<PointerState>()
        .insert_resource(Time::<()>::default())
        .add_message::<WindowEvent>()
        .add_systems(Update, pointer_input);
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: WindowResolution::new(1440, 960),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    let mut projection = camera_projection();
    projection.update(1440.0, 960.0);
    app.world_mut().spawn((
        TableCamera,
        Camera {
            computed: ComputedCameraValues {
                clip_from_view: projection.get_clip_from_view(),
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(1440, 960),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
        GlobalTransform::from(camera_transform(false)),
    ));
    let positions = layout(&app.world().resource::<Table>().game);
    for p in positions {
        app.world_mut()
            .spawn((CardBody(p.card), Transform::from_translation(p.position)));
    }
    (app, window)
}

fn screen(app: &mut App, position: Vec3) -> Vec2 {
    let mut query = app.world_mut().query::<(&Camera, &GlobalTransform)>();
    let (camera, transform) = query.single(app.world()).unwrap();
    camera.world_to_viewport(transform, position).unwrap()
}
fn source(app: &mut App, from: Source) -> Vec2 {
    let table = app.world().resource::<Table>();
    let card = source_card(&table.game, from).unwrap();
    let position = layout(&table.game)
        .iter()
        .find(|p| p.card == card)
        .unwrap()
        .position;
    screen(app, position)
}
fn target(app: &mut App, to: Target) -> Vec2 {
    let position = target_position(&app.world().resource::<Table>().game, to);
    screen(app, position)
}
fn cursor(app: &mut App, window: Entity, position: Vec2) {
    app.world_mut()
        .write_message(WindowEvent::CursorMoved(CursorMoved {
            window,
            position,
            delta: None,
        }));
}
fn button(app: &mut App, window: Entity, state: ButtonState) {
    app.world_mut()
        .write_message(WindowEvent::MouseButtonInput(MouseButtonInput {
            window,
            button: MouseButton::Left,
            state,
        }));
}

#[test]
fn press_move_release_in_one_frame_commits_exactly_once() {
    let (mut app, window) = app();
    let start = source(
        &mut app,
        Source::Tableau {
            column: 4,
            index: 4,
        },
    );
    let end = target(&mut app, Target::Tableau(5));
    cursor(&mut app, window, start);
    button(&mut app, window, ButtonState::Pressed);
    cursor(&mut app, window, end);
    button(&mut app, window, ButtonState::Released);
    app.update();
    let table = app.world().resource::<Table>();
    assert_eq!(table.game.view().moves, 1);
    assert_eq!(table.game.view().tableau[5].len(), 7);
    assert!(table.game.view().tableau[4].last().unwrap().face_up);
    assert!(table.selection.is_none());
    assert!(!app.world().resource::<PointerState>().held);
    app.update();
    assert_eq!(app.world().resource::<Table>().game.view().moves, 1);
}

#[test]
fn click_then_click_moves_and_invalid_drop_leaves_history_untouched() {
    let (mut app, window) = app();
    let start = source(
        &mut app,
        Source::Tableau {
            column: 4,
            index: 4,
        },
    );
    cursor(&mut app, window, start);
    button(&mut app, window, ButtonState::Pressed);
    button(&mut app, window, ButtonState::Released);
    app.update();
    assert!(app.world().resource::<Table>().selection.is_some());
    let end = target(&mut app, Target::Tableau(5));
    cursor(&mut app, window, end);
    button(&mut app, window, ButtonState::Pressed);
    button(&mut app, window, ButtonState::Released);
    app.update();
    assert_eq!(app.world().resource::<Table>().game.view().moves, 1);
    app.world_mut().resource_mut::<Table>().undo();
    cursor(&mut app, window, start);
    button(&mut app, window, ButtonState::Pressed);
    cursor(&mut app, window, Vec2::new(10.0, 10.0));
    button(&mut app, window, ButtonState::Released);
    app.update();
    let table = app.world().resource::<Table>();
    assert_eq!(table.game.view().moves, 0);
    assert!(table.game.can_redo());
    assert!(table.selection.is_none());
}

#[test]
fn stock_clicks_and_recycle_use_the_core_order() {
    let (mut app, window) = app();
    let stock = screen(&mut app, Vec3::new(column_x(0), 0.25, TOP_Z));
    for _ in 0..25 {
        cursor(&mut app, window, stock);
        button(&mut app, window, ButtonState::Pressed);
        button(&mut app, window, ButtonState::Released);
        app.update();
    }
    let table = app.world().resource::<Table>();
    assert_eq!(table.game.view().moves, 25);
    assert_eq!(table.game.view().stock.len(), 24);
    assert!(table.game.view().waste.is_empty());
}

#[test]
fn automatic_finish_executes_the_shared_proof_and_undo_interrupts_it() {
    let mut table = Table::new(6, DrawMode::One);
    for line in include_str!("../../solitaire-core/tests/fixtures/seed-6.moves").lines() {
        let action = match line {
            "D" => Action::Draw,
            "R" => Action::Recycle,
            _ => {
                let (s, t) = line.split_once('>').unwrap();
                let from = if s == "W" {
                    Source::Waste
                } else {
                    let (c, i) = s[1..].split_once(':').unwrap();
                    Source::Tableau {
                        column: c.parse().unwrap(),
                        index: i.parse().unwrap(),
                    }
                };
                let i = t[1..].parse().unwrap();
                Action::Move {
                    from,
                    to: if t.starts_with('F') {
                        Target::Foundation(i)
                    } else {
                        Target::Tableau(i)
                    },
                }
            }
        };
        assert!(table.apply(action));
        if table.game.auto_finish_plan().is_some() {
            break;
        }
    }
    table.finish();
    assert!(!table.auto.is_empty());
    let mut app = App::new();
    app.insert_resource(table)
        .insert_resource(Time::<()>::default())
        .add_systems(Update, advance_game);
    for _ in 0..500 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(200));
        app.update();
        if app.world().resource::<Table>().game.status() == Status::Won {
            break;
        }
    }
    let mut table = app.world_mut().resource_mut::<Table>();
    assert_eq!(table.game.status(), Status::Won);
    assert_eq!(table.elapsed, 0.0);
    table.undo();
    assert!(table.auto.is_empty());
    assert_eq!(table.game.status(), Status::Playing);
}
