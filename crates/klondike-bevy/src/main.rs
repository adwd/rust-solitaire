use bevy::{
    asset::RenderAssetUsages,
    camera::ScalingMode,
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    image::{CompressedImageFormats, ImageSampler, ImageType},
    input::ButtonState,
    mesh::VertexAttributeValues,
    picking::hover::Hovered,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    window::{PrimaryWindow, WindowEvent, WindowResolution},
};
use klondike_bevy::*;
use klondike_core::{Action, Card, DrawMode, Rank, Source, Status, Suit, Target};
use std::time::{SystemTime, UNIX_EPOCH};

mod card_mesh;

#[cfg(test)]
mod input_tests;

const GOLD: Color = Color::srgb(225.0 / 255.0, 196.0 / 255.0, 126.0 / 255.0);
const PALE: Color = Color::srgb(1.0, 250.0 / 255.0, 239.0 / 255.0);
const MUTED: Color = Color::srgb(151.0 / 255.0, 183.0 / 255.0, 167.0 / 255.0);

#[derive(Component)]
struct TableCamera;
#[derive(Component)]
struct CardBody(Card);
#[derive(Component)]
struct CardFace(usize);
#[derive(Component)]
struct CardGlow(usize);
#[derive(Component)]
struct SlotGlow(Target);
#[derive(Component)]
struct StockGlow;
#[derive(Component)]
struct StatsText;
#[derive(Component)]
struct StatusText;
#[derive(Component)]
struct DealText;
#[derive(Component)]
struct WinPanel;
#[derive(Component)]
struct Confetti {
    velocity: Vec3,
    spin: Vec3,
    life: f32,
}

#[derive(Component, Clone, Copy)]
enum Control {
    Undo,
    Redo,
    Hint,
    Finish,
    New,
    Restart,
    View,
    DrawMode,
}

#[derive(Resource)]
struct Artwork {
    faces: Vec<Handle<Mesh>>,
    back_mesh: Handle<Mesh>,
    face_material: Handle<StandardMaterial>,
    back_material: Handle<StandardMaterial>,
}
#[derive(Resource, Default)]
struct PointerState {
    press: Option<Vec2>,
    cursor: Option<Vec2>,
    held: bool,
    offset: Vec3,
    dragging: bool,
    last_click: Option<(Source, f64)>,
    world: Option<Vec3>,
}
#[derive(Resource)]
struct Capture {
    path: Option<String>,
    taken: bool,
}

fn random_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn main() {
    let mut seed = None;
    let mut draw = DrawMode::One;
    let mut capture_path = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => {
                seed = Some(
                    args.next()
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or_else(|| invalid("--seed requires an unsigned 64-bit integer")),
                )
            }
            "--draw" => {
                draw = match args.next().as_deref() {
                    Some("1") => DrawMode::One,
                    Some("3") => DrawMode::Three,
                    _ => invalid("--draw must be 1 or 3"),
                }
            }
            "--capture" => {
                capture_path = Some(
                    args.next()
                        .unwrap_or_else(|| invalid("--capture requires a PNG path")),
                )
            }
            "--help" | "-h" => {
                println!(
                    "klondike-bevy [--seed <u64>] [--draw 1|3] [--capture <path.png>]\n\n--capture saves a preview after startup and exits.\nN: new deal, R: restart, Space: draw, H: hint, A: finish\nZ: undo, Y: redo, V: camera, Esc: cancel, F12: screenshot"
                );
                return;
            }
            _ => invalid(&format!("unknown argument: {arg}")),
        }
    }
    App::new()
        .insert_resource(Table::new(seed.unwrap_or_else(random_seed), draw))
        .init_resource::<PointerState>()
        .insert_resource(Capture {
            path: capture_path,
            taken: false,
        })
        .insert_resource(ClearColor(Color::srgb(0.025, 0.045, 0.043)))
        .insert_resource(GlobalAmbientLight {
            brightness: 275.0,
            ..default()
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rust Solitaire · The Bevy Table".into(),
                name: Some("adwd.rust-solitaire.bevy".into()),
                resolution: WindowResolution::new(1440, 960),
                resize_constraints: WindowResizeConstraints {
                    min_width: 960.0,
                    min_height: 720.0,
                    ..default()
                },
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (setup_table, setup_ui))
        .add_systems(
            Update,
            (
                controls,
                pointer_input,
                advance_game,
                animate_cards,
                animate_camera,
                update_ui,
                celebration,
                capture,
            )
                .chain(),
        )
        .run();
}

fn invalid(message: &str) -> ! {
    eprintln!("{message}\nUsage: klondike-bevy [--seed <u64>] [--draw 1|3] [--capture <path.png>]");
    std::process::exit(2)
}
fn image(bytes: &[u8], images: &mut Assets<Image>) -> Handle<Image> {
    images.add(
        Image::from_buffer(
            bytes,
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::linear(),
            RenderAssetUsages::RENDER_WORLD,
        )
        .expect("embedded PNG artwork is valid"),
    )
}

fn setup_table(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    table: Res<Table>,
) {
    commands.spawn((
        Camera3d::default(),
        TableCamera,
        Msaa::Sample4,
        Tonemapping::None,
        DebandDither::Disabled,
        Projection::from(camera_projection()),
        camera_transform(false),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 3250.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-5.0, 12.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    let felt_texture = image(include_bytes!("../assets/felt.png"), &mut images);
    let felt = materials.add(StandardMaterial {
        base_color_texture: Some(felt_texture),
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    });
    // The felt fills the view. Depth comes from rounded paper, light and motion.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(30.0, 24.0))),
        MeshMaterial3d(felt),
        Transform::from_xyz(0.0, -0.015, 0.4),
    ));
    let plane = meshes.add(Plane3d::default().mesh().size(CARD_W, CARD_H));
    let slot_bytes: [&[u8]; 6] = [
        include_bytes!("../assets/slot0.png"),
        include_bytes!("../assets/slot1.png"),
        include_bytes!("../assets/slot2.png"),
        include_bytes!("../assets/slot3.png"),
        include_bytes!("../assets/slot4.png"),
        include_bytes!("../assets/slot5.png"),
    ];
    let slot_mats: Vec<_> = slot_bytes
        .iter()
        .map(|b| {
            materials.add(StandardMaterial {
                base_color_texture: Some(image(b, &mut images)),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })
        })
        .collect();
    for column in 0..7 {
        commands.spawn((
            Mesh3d(plane.clone()),
            MeshMaterial3d(slot_mats[5].clone()),
            Transform::from_xyz(column_x(column), 0.015, TABLEAU_Z),
        ));
    }
    for column in [0, 1, 3, 4, 5, 6] {
        let slot = if column == 0 {
            4
        } else if column == 1 {
            5
        } else {
            column - 3
        };
        commands.spawn((
            Mesh3d(plane.clone()),
            MeshMaterial3d(slot_mats[slot].clone()),
            Transform::from_xyz(column_x(column), 0.015, TOP_Z),
        ));
    }
    let glow = materials.add(StandardMaterial {
        base_color_texture: Some(image(
            include_bytes!("../assets/highlight.png"),
            &mut images,
        )),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let glow_mesh = meshes.add(Plane3d::default().mesh().size(CARD_W + 0.09, CARD_H + 0.09));
    for target in (0..4)
        .map(Target::Foundation)
        .chain((0..7).map(Target::Tableau))
    {
        commands.spawn((
            SlotGlow(target),
            Mesh3d(glow_mesh.clone()),
            MeshMaterial3d(glow.clone()),
            Transform::from_translation(target_position(&table.game, target)),
            Visibility::Hidden,
        ));
    }
    commands.spawn((
        StockGlow,
        Mesh3d(glow_mesh.clone()),
        MeshMaterial3d(glow.clone()),
        Transform::from_xyz(column_x(0), 0.03, TOP_Z),
        Visibility::Hidden,
    ));
    let face_texture = image(include_bytes!("../assets/cards.png"), &mut images);
    let back_texture = image(include_bytes!("../assets/back.png"), &mut images);
    let face_material = materials.add(StandardMaterial {
        base_color_texture: Some(face_texture),
        perceptual_roughness: 0.85,
        unlit: true,
        ..default()
    });
    let back_material = materials.add(StandardMaterial {
        base_color_texture: Some(back_texture),
        perceptual_roughness: 0.8,
        unlit: true,
        ..default()
    });
    let paper = materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.87, 0.79),
        perceptual_roughness: 0.9,
        ..default()
    });
    let body = meshes.add(card_mesh::body());
    let card_plane = meshes.add(card_mesh::face());
    let faces: Vec<_> = (0..52)
        .map(|id| {
            let mut mesh = card_mesh::face();
            if let Some(VertexAttributeValues::Float32x2(uvs)) =
                mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
            {
                for uv in uvs {
                    uv[0] = (uv[0] + (id % 8) as f32) / 8.0;
                    uv[1] = (uv[1] + (id / 8) as f32) / 7.0;
                }
            }
            meshes.add(mesh)
        })
        .collect();
    for suit in Suit::ALL {
        for rank in Rank::ALL {
            let card = Card::new(suit, rank);
            let id = card_id(card);
            commands
                .spawn((
                    CardBody(card),
                    Mesh3d(body.clone()),
                    MeshMaterial3d(paper.clone()),
                    Transform::from_xyz(column_x(0), 0.2, TOP_Z),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        CardFace(id),
                        Mesh3d(card_plane.clone()),
                        MeshMaterial3d(back_material.clone()),
                        Transform::from_xyz(0.0, 0.015, 0.0),
                    ));
                    parent.spawn((
                        CardGlow(id),
                        Mesh3d(glow_mesh.clone()),
                        MeshMaterial3d(glow.clone()),
                        Transform::from_xyz(0.0, -0.016, 0.0),
                        Visibility::Hidden,
                    ));
                });
        }
    }
    commands.insert_resource(Artwork {
        faces,
        back_mesh: card_plane,
        face_material,
        back_material,
    });
}

fn label(font: &Handle<Font>, value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font: font.clone().into(),
            font_size: size.into(),
            weight: bevy::text::FontWeight::MEDIUM,
            ..default()
        },
        TextColor(color),
    )
}
fn setup_ui(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    let sans = fonts.add(Font::from_bytes(
        include_bytes!("../assets/fonts/Manrope.ttf").to_vec(),
    ));
    let serif = fonts.add(Font::from_bytes(
        include_bytes!("../assets/fonts/DMSerifDisplay-Regular.ttf").to_vec(),
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(24),
            left: px(42),
            align_items: AlignItems::Baseline,
            column_gap: px(18),
            ..default()
        },
        children![
            label(&sans, "Solitaire", 27.0, PALE),
            label(&sans, "Klondike", 12.0, MUTED),
        ],
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(76),
            left: px(42),
            right: px(42),
            height: px(1),
            ..default()
        },
        BackgroundColor(Color::srgba(0.75, 0.83, 0.73, 0.16)),
    ));
    commands.spawn((
        StatsText,
        label(&sans, "", 17.0, PALE),
        Node {
            position_type: PositionType::Absolute,
            top: px(23),
            right: px(42),
            ..default()
        },
    ));
    commands.spawn((
        DealText,
        label(&sans, "", 11.0, MUTED),
        Node {
            position_type: PositionType::Absolute,
            top: px(52),
            right: px(42),
            ..default()
        },
    ));
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: px(22),
            width: percent(100),
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: px(14),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((StatusText, label(&sans, "", 13.0, MUTED)));
            parent
                .spawn(Node {
                    column_gap: px(8),
                    ..default()
                })
                .with_children(|bar| {
                    for (control, title, key) in [
                        (Control::Undo, "Undo", "Z"),
                        (Control::Redo, "Redo", "Y"),
                        (Control::Hint, "Hint", "H"),
                        (Control::Finish, "Finish", "A"),
                        (Control::Restart, "Restart", "R"),
                        (Control::View, "View", "V"),
                        (Control::DrawMode, "Draw 1 / 3", ""),
                        (Control::New, "New deal", "N"),
                    ] {
                        let primary = matches!(control, Control::New);
                        bar.spawn((
                            Hovered::default(),
                            control,
                            Node {
                                align_items: AlignItems::Center,
                                column_gap: px(12),
                                padding: UiRect::axes(px(17), px(12)),
                                border: UiRect::all(px(1)),
                                border_radius: BorderRadius::all(px(8)),
                                ..default()
                            },
                            BorderColor::all(Color::srgba(0.75, 0.83, 0.73, 0.18)),
                            BackgroundColor(control_color(control, false, false)),
                            children![
                                label(
                                    &sans,
                                    title,
                                    14.0,
                                    if primary {
                                        Color::srgb(0.10, 0.20, 0.16)
                                    } else {
                                        PALE
                                    }
                                ),
                                label(
                                    &sans,
                                    key,
                                    10.0,
                                    if primary {
                                        Color::srgb(0.30, 0.38, 0.28)
                                    } else {
                                        MUTED
                                    }
                                ),
                            ],
                        ));
                    }
                });
            parent.spawn(label(
                &sans,
                "Drag to move   ·   Double-click to send home   ·   Space to draw",
                11.0,
                MUTED,
            ));
        });
    commands.spawn((
        WinPanel,
        Node {
            position_type: PositionType::Absolute,
            top: percent(43),
            left: percent(25),
            width: percent(50),
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(32)),
            row_gap: px(12),
            border_radius: BorderRadius::all(px(16)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.025, 0.065, 0.05, 0.96)),
        Visibility::Hidden,
        children![
            label(&serif, "Beautifully played.", 36.0, GOLD),
            label(
                &sans,
                "All 52 cards are home. Press N for another table.",
                14.0,
                PALE
            ),
        ],
    ));
}
fn control_color(control: Control, hovered: bool, pressed: bool) -> Color {
    if matches!(control, Control::New) {
        if pressed {
            Color::srgb(0.72, 0.66, 0.48)
        } else if hovered {
            Color::srgb(0.94, 0.87, 0.68)
        } else {
            GOLD
        }
    } else if pressed {
        Color::srgb(0.19, 0.32, 0.25)
    } else if hovered {
        Color::srgb(0.14, 0.27, 0.21)
    } else {
        Color::srgba(0.035, 0.12, 0.09, 0.75)
    }
}
fn reset(table: &mut Table, seed: u64, draw: DrawMode) {
    let top = table.top_view;
    *table = Table::new(seed, draw);
    table.top_view = top;
}
fn controls(
    mut table: ResMut<Table>,
    mut pointer: ResMut<PointerState>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut buttons: Query<(&Hovered, &Control, &mut BackgroundColor)>,
) {
    let mut action = None;
    for (hovered, control, mut color) in &mut buttons {
        color.0 = control_color(
            *control,
            hovered.get(),
            hovered.get() && mouse.pressed(MouseButton::Left),
        );
        if hovered.get() && mouse.just_pressed(MouseButton::Left) {
            action = Some(*control);
        }
    }
    let modifier = keys.pressed(KeyCode::SuperLeft)
        || keys.pressed(KeyCode::SuperRight)
        || keys.pressed(KeyCode::ControlLeft)
        || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    for (key, control) in [
        (KeyCode::KeyN, Control::New),
        (KeyCode::KeyR, Control::Restart),
        (KeyCode::KeyH, Control::Hint),
        (KeyCode::KeyA, Control::Finish),
        (KeyCode::KeyV, Control::View),
        (KeyCode::KeyY, Control::Redo),
    ] {
        if keys.just_pressed(key) {
            action = Some(control);
        }
    }
    if keys.just_pressed(KeyCode::KeyZ) {
        action = Some(if modifier && shift {
            Control::Redo
        } else {
            Control::Undo
        });
    }
    if keys.just_pressed(KeyCode::Space) {
        table.draw();
        pointer.press = None;
        pointer.dragging = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        table.cancel();
        *pointer = PointerState::default();
    }
    if let Some(action) = action {
        let seed = table.game.view().seed;
        let draw = table.game.view().rules.draw;
        match action {
            Control::Undo => table.undo(),
            Control::Redo => table.redo(),
            Control::Hint => table.show_hint(),
            Control::Finish => table.finish(),
            Control::New => reset(&mut table, random_seed(), draw),
            Control::Restart => reset(&mut table, seed, draw),
            Control::View => table.top_view = !table.top_view,
            Control::DrawMode => reset(
                &mut table,
                seed,
                if draw == DrawMode::One {
                    DrawMode::Three
                } else {
                    DrawMode::One
                },
            ),
        }
        pointer.dragging = false;
    }
}
fn point_on_plane(ray: &Ray3d, height: f32) -> Option<Vec3> {
    if ray.direction.y.abs() < 0.0001 {
        return None;
    }
    let distance = (height - ray.origin.y) / ray.direction.y;
    (distance > 0.0).then(|| ray.get_point(distance))
}
fn hit_card(
    ray: &Ray3d,
    table: &Table,
    cards: &Query<(&CardBody, &Transform), Without<TableCamera>>,
) -> Option<Source> {
    let placements = layout(&table.game);
    let mut hits = Vec::new();
    for (body, transform) in cards {
        let Some(placed) = placements.iter().find(|p| p.card == body.0) else {
            continue;
        };
        let Some(point) = point_on_plane(ray, transform.translation.y + 0.02) else {
            continue;
        };
        let delta = point - transform.translation;
        if delta.x.abs() <= CARD_W / 2.0 && delta.z.abs() <= CARD_H / 2.0 {
            hits.push((ray.origin.distance_squared(point), placed.source));
        }
    }
    hits.sort_by(|a, b| a.0.total_cmp(&b.0));
    hits.first().and_then(|h| h.1)
}
fn target_at(ray: &Ray3d, game: &klondike_core::Game) -> Option<Target> {
    let point = point_on_plane(ray, 0.14)?;
    for pile in 0..4 {
        let target = Target::Foundation(pile);
        let p = target_position(game, target);
        if (point.x - p.x).abs() < CARD_W * 0.58 && (point.z - p.z).abs() < CARD_H * 0.58 {
            return Some(target);
        }
    }
    for column in 0..7 {
        if (point.x - column_x(column)).abs() < CARD_W * 0.58
            && point.z >= TABLEAU_Z - CARD_H / 2.0
            && point.z < 5.75
        {
            return Some(Target::Tableau(column));
        }
    }
    None
}
// Preserve the OS event order: a press, movement and release may share a frame.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn pointer_input(
    mut table: ResMut<Table>,
    mut pointer: ResMut<PointerState>,
    mut events: MessageReader<WindowEvent>,
    window: Single<(Entity, &Window), With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<TableCamera>>,
    cards: Query<(&CardBody, &Transform), Without<TableCamera>>,
    buttons: Query<&Hovered, With<Control>>,
    time: Res<Time>,
) {
    let blocked = buttons.iter().any(Hovered::get);
    for event in events.read() {
        match event {
            WindowEvent::CursorMoved(moved) if moved.window == window.0 => {
                pointer.cursor = Some(moved.position);
                if pointer.held
                    && pointer
                        .press
                        .is_some_and(|p| p.distance(moved.position) > 5.0)
                {
                    pointer.dragging = table.selection.is_some();
                }
            }
            WindowEvent::CursorLeft(left) if left.window == window.0 => {
                pointer.cursor = None;
            }
            WindowEvent::WindowFocused(focus) if focus.window == window.0 && !focus.focused => {
                *pointer = PointerState::default();
                table.selection = None;
            }
            WindowEvent::MouseButtonInput(input) if input.window == window.0 => {
                let Some(cursor) = pointer.cursor.or_else(|| window.1.cursor_position()) else {
                    continue;
                };
                let Ok(ray) = camera.0.viewport_to_world(camera.1, cursor) else {
                    continue;
                };
                pointer.world = point_on_plane(&ray, 1.0);
                match (input.button, input.state) {
                    (MouseButton::Left, ButtonState::Released) => {
                        if pointer.dragging {
                            if !blocked
                                && let (Some(from), Some(to)) =
                                    (table.selection, target_at(&ray, &table.game))
                            {
                                table.auto.clear();
                                table.apply(Action::Move { from, to });
                            }
                            table.selection = None;
                        }
                        pointer.held = false;
                        pointer.dragging = false;
                        pointer.press = None;
                    }
                    (MouseButton::Right, ButtonState::Pressed) if !blocked => {
                        if let Some(source) = hit_card(&ray, &table, &cards).or(table.selection) {
                            table.promote(source);
                        }
                    }
                    (MouseButton::Left, ButtonState::Pressed) if !blocked => {
                        pointer.held = true;
                        table.auto.clear();
                        table.hint = None;
                        let point = point_on_plane(&ray, 0.15).unwrap_or(Vec3::ZERO);
                        if (point.x - column_x(0)).abs() < CARD_W / 2.0
                            && (point.z - TOP_Z).abs() < CARD_H / 2.0
                        {
                            table.draw();
                            pointer.press = None;
                            continue;
                        }
                        if let (Some(from), Some(to)) =
                            (table.selection, target_at(&ray, &table.game))
                            && table.game.validate(Action::Move { from, to }).is_ok()
                        {
                            table.apply(Action::Move { from, to });
                            pointer.press = None;
                            continue;
                        }
                        if let Some(source) = hit_card(&ray, &table, &cards) {
                            let now = time.elapsed_secs_f64();
                            if pointer
                                .last_click
                                .is_some_and(|(last, t)| last == source && now - t < 0.35)
                            {
                                table.promote(source);
                                pointer.last_click = None;
                                pointer.press = None;
                                continue;
                            }
                            pointer.last_click = Some((source, now));
                            table.selection = Some(source);
                            pointer.press = Some(cursor);
                            if let (Some(card), Some(world)) =
                                (source_card(&table.game, source), pointer.world)
                                && let Some(origin) =
                                    layout(&table.game).iter().find(|p| p.card == card)
                            {
                                pointer.offset = origin.position - world;
                                pointer.offset.y = 0.0;
                            }
                        } else {
                            table.selection = None;
                            pointer.press = None;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    if let Some(cursor) = pointer.cursor
        && let Ok(ray) = camera.0.viewport_to_world(camera.1, cursor)
    {
        pointer.world = point_on_plane(&ray, 1.0);
    }
}

fn advance_game(mut table: ResMut<Table>, time: Res<Time>) {
    if table.started && table.game.status() != Status::Won && table.auto.is_empty() {
        table.elapsed += time.delta_secs_f64();
    }
    if !table.auto.is_empty() {
        table.auto_clock -= time.delta_secs();
        if table.auto_clock <= 0.0 {
            if let Some(action) = table.auto.pop_front()
                && !table.apply(action)
            {
                table.auto.clear();
            }
            table.auto_clock = 0.18;
        }
    }
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)] // Bevy system queries keep component access explicit.
fn animate_cards(
    table: Res<Table>,
    pointer: Res<PointerState>,
    artwork: Res<Artwork>,
    time: Res<Time>,
    mut cards: Query<(&CardBody, &mut Transform)>,
    mut faces: Query<(
        &CardFace,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    mut glows: Query<(&CardGlow, &mut Visibility), (Without<SlotGlow>, Without<StockGlow>)>,
    mut slots: Query<
        (&SlotGlow, &mut Transform, &mut Visibility),
        (Without<CardBody>, Without<CardGlow>, Without<StockGlow>),
    >,
    mut stock: Query<&mut Visibility, (With<StockGlow>, Without<CardGlow>, Without<SlotGlow>)>,
) {
    let placements = layout(&table.game);
    let selected = table
        .selection
        .map_or_else(Vec::new, |s| selected_cards(&table.game, s));
    let hinted = match table.hint {
        Some(Action::Move { from, .. }) => selected_cards(&table.game, from),
        _ => vec![],
    };
    let origin = selected
        .first()
        .and_then(|c| placements.iter().find(|p| p.card == *c))
        .map(|p| p.position);
    let amount = 1.0 - (-time.delta_secs() * 18.0).exp();
    for (body, mut transform) in &mut cards {
        let placed = placements
            .iter()
            .find(|p| p.card == body.0)
            .expect("the core always has 52 cards");
        let mut goal = placed.position;
        let dragging = pointer.dragging && selected.contains(&body.0);
        if dragging && let (Some(world), Some(origin)) = (pointer.world, origin) {
            goal = world + pointer.offset + (placed.position - origin);
            goal.y = 1.0 + (placed.position.y - origin.y);
        }
        transform.translation = transform
            .translation
            .lerp(goal, if dragging { 0.75 } else { amount });
        transform.rotation = transform.rotation.slerp(
            if dragging {
                Quat::from_euler(EulerRot::XYZ, -0.035, 0.0, 0.025)
            } else {
                Quat::IDENTITY
            },
            amount,
        );
    }
    for (face, mut mesh, mut material) in &mut faces {
        let up = placements
            .iter()
            .find(|p| card_id(p.card) == face.0)
            .is_some_and(|p| p.face_up);
        mesh.0 = if up {
            artwork.faces[face.0].clone()
        } else {
            artwork.back_mesh.clone()
        };
        material.0 = if up {
            artwork.face_material.clone()
        } else {
            artwork.back_material.clone()
        };
    }
    for (glow, mut visibility) in &mut glows {
        *visibility = if selected
            .iter()
            .chain(&hinted)
            .any(|c| card_id(*c) == glow.0)
        {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (slot, mut transform, mut visibility) in &mut slots {
        transform.translation = target_position(&table.game, slot.0) - Vec3::Y * 0.025;
        *visibility = if matches!(table.hint,Some(Action::Move { to,.. }) if to == slot.0) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut visibility in &mut stock {
        *visibility = if matches!(table.hint, Some(Action::Draw | Action::Recycle)) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
fn camera_projection() -> OrthographicProjection {
    OrthographicProjection {
        scaling_mode: ScalingMode::AutoMin {
            min_width: 17.2,
            min_height: 12.8,
        },
        ..OrthographicProjection::default_3d()
    }
}
fn camera_transform(top: bool) -> Transform {
    Transform::from_xyz(0.0, 24.0, if top { 0.41 } else { 7.4 })
        .looking_at(Vec3::new(0.0, 0.0, 0.4), Vec3::Y)
}
fn animate_camera(
    table: Res<Table>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut camera: Single<(&mut Transform, &mut Projection), With<TableCamera>>,
) {
    let goal = camera_transform(table.top_view);
    let amount = 1.0 - (-time.delta_secs() * 7.0).exp();
    let (transform, projection) = &mut *camera;
    transform.translation = transform.translation.lerp(goal.translation, amount);
    transform.rotation = transform.rotation.slerp(goal.rotation, amount);
    // Preserve readable indices; widen the camera only when a long column
    // would reach the header or controls, including in the top-down view.
    let inverse = goal.compute_affine().inverse();
    let mut needed: f32 = 12.8;
    let top_space = (0.5 - 96.0 / window.height()).max(0.1);
    let bottom_space = (0.5 - 140.0 / window.height()).max(0.1);
    for p in layout(&table.game) {
        let top = inverse
            .transform_point3(p.position - Vec3::Z * CARD_H * 0.5)
            .y;
        let bottom = inverse
            .transform_point3(p.position + Vec3::Z * CARD_H * 0.5)
            .y;
        needed = needed.max(top / top_space).max(-bottom / bottom_space);
    }
    if let Projection::Orthographic(projection) = &mut **projection
        && let ScalingMode::AutoMin { min_height, .. } = &mut projection.scaling_mode
    {
        *min_height += (needed - *min_height) * amount;
    }
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)] // Bevy system queries keep component access explicit.
fn update_ui(
    table: Res<Table>,
    mut text: ParamSet<(
        Query<&mut Text, With<StatsText>>,
        Query<&mut Text, With<StatusText>>,
        Query<&mut Text, With<DealText>>,
    )>,
    mut win: Query<&mut Visibility, With<WinPanel>>,
) {
    let view = table.game.view();
    let seconds = table.elapsed as u64;
    for mut t in &mut text.p0() {
        t.0 = format!(
            "{:02}:{:02}      {} moves      {}/52 home",
            seconds / 60,
            seconds % 60,
            view.moves,
            view.foundations.iter().map(Vec::len).sum::<usize>()
        );
    }
    for mut t in &mut text.p1() {
        t.0.clone_from(&table.message);
    }
    for mut t in &mut text.p2() {
        t.0 = format!(
            "Draw {}   ·   Deal {}   ·   {}",
            view.rules.draw.count(),
            view.seed,
            if table.top_view {
                "Top view"
            } else {
                "Table view"
            }
        );
    }
    for mut visibility in &mut win {
        *visibility = if table.game.status() == Status::Won {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
fn celebration(
    mut commands: Commands,
    table: Res<Table>,
    mut was_won: Local<bool>,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Confetti, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let won = table.game.status() == Status::Won;
    if won && !*was_won {
        let mesh = meshes.add(Cuboid::new(0.065, 0.025, 0.13));
        let colors = [
            GOLD,
            Color::srgb(0.82, 0.32, 0.25),
            Color::srgb(0.35, 0.69, 0.57),
        ];
        let mats: Vec<_> = colors
            .into_iter()
            .map(|c| {
                materials.add(StandardMaterial {
                    base_color: c,
                    unlit: true,
                    ..default()
                })
            })
            .collect();
        for i in 0..100 {
            let angle = i as f32 * 2.39996;
            let speed = 1.0 + (i % 11) as f32 * 0.3;
            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mats[i % 3].clone()),
                Transform::from_xyz(0.0, 1.0, 0.0),
                Confetti {
                    velocity: Vec3::new(
                        angle.cos() * speed,
                        3.0 + (i % 7) as f32 * 0.4,
                        angle.sin() * speed,
                    ),
                    spin: Vec3::new(2.0, angle, 1.0),
                    life: 5.0,
                },
            ));
        }
    }
    *was_won = won;
    for (entity, mut p, mut transform) in &mut particles {
        p.life -= time.delta_secs();
        p.velocity.y -= time.delta_secs() * 3.0;
        transform.translation += p.velocity * time.delta_secs();
        transform.rotate(Quat::from_euler(
            EulerRot::XYZ,
            p.spin.x * time.delta_secs(),
            p.spin.y * time.delta_secs(),
            p.spin.z * time.delta_secs(),
        ));
        if p.life <= 0.0 || !won {
            commands.entity(entity).despawn();
        }
    }
}
fn capture(
    mut commands: Commands,
    mut capture: ResMut<Capture>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(path) = capture.path.clone() {
        if !capture.taken && time.elapsed_secs() > 4.0 {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
            capture.taken = true;
        }
        if capture.taken && time.elapsed_secs() > 7.0 {
            exit.write(AppExit::Success);
        }
    }
    if keys.just_pressed(KeyCode::F12) {
        let _ = std::fs::create_dir_all("screenshots");
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!(
                "screenshots/bevy-{}.png",
                random_seed()
            )));
    }
}
