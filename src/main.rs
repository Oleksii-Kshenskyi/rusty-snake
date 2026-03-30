pub mod config;

use std::collections::HashMap;

use bevy::{
    prelude::*,
    window::{EnabledButtons, PrimaryWindow, WindowResized},
};
use rand::prelude::*;

use crate::config::*;

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BoardPos {
    pub x: u32,
    pub y: u32,
}

fn board_pos_to_world(pos: &BoardPos, board: &Board) -> (f32, f32) {
    let grid_width = board.cols() as f32 * TILE_SIZE;
    let grid_height = board.rows() as f32 * TILE_SIZE;

    // Tile center in top-left-origin space
    let x = pos.x as f32 * TILE_SIZE + TILE_SIZE / 2.;
    let y = pos.y as f32 * TILE_SIZE + TILE_SIZE / 2.;

    // Convert to Bevy's center-origin, Y-up
    (x - grid_width / 2., grid_height / 2. - y)
}

#[derive(Resource, Debug)]
pub struct MainWindowDesc {
    pub width: f32,
    pub height: f32,
    pub id: Option<Entity>,
}
impl MainWindowDesc {
    pub fn new() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            id: None,
        }
    }
}

#[derive(Resource)]
pub struct GameRng {
    pub rng: rand::rngs::StdRng,
}
impl GameRng {
    pub fn new() -> Self {
        Self {
            rng: rand::make_rng(),
        }
    }

    pub fn random_pos(
        &mut self,
        x_range: impl rand::distr::uniform::SampleRange<u32>,
        y_range: impl rand::distr::uniform::SampleRange<u32>,
    ) -> BoardPos {
        BoardPos {
            x: self.rng.random_range(x_range),
            y: self.rng.random_range(y_range),
        }
    }
}

pub struct MaximizeShenanigans {
    pub last_known_size: u64,
    pub assume_maximize_happened: bool,
}
impl MaximizeShenanigans {
    pub fn new() -> Self {
        Self {
            last_known_size: 0,
            assume_maximize_happened: false,
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Component, Clone)]
pub struct SnakeSegment {
    pub pos: BoardPos,
    pub next: Option<BoardPos>,
    pub prev: Option<BoardPos>,
}

#[derive(Component, Clone)]
pub struct Snake {
    pub snake_segments: HashMap<BoardPos, Entity>,
    pub head_pos: BoardPos,
    pub tail_pos: BoardPos,
    pub direction: Direction,
}
impl Snake {
    pub fn new(
        start_pos: BoardPos,
        commands: Commands,
        meshes: ResMut<Assets<Mesh>>,
        materials: ResMut<Assets<ColorMaterial>>,
        board: &Board,
    ) -> Self {
        let mut snake = Self {
            snake_segments: HashMap::new(),
            head_pos: start_pos,
            tail_pos: start_pos,
            direction: Direction::Right,
        };
        let _ = snake.spawn_segment(commands, meshes, materials, board);
        snake
    }

    pub fn spawn_segment(
        &mut self,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<ColorMaterial>>,
        board: &Board,
    ) -> Entity {
        let is_head = self.snake_segments.is_empty();
        let mut entity = Entity::from_raw_u32(0).unwrap();
        match is_head {
            true => {
                let head_xy = board_pos_to_world(&self.head_pos, board);
                entity = commands
                    .spawn((
                        Mesh2d(meshes.add(Rectangle::new(TILE_SIZE, TILE_SIZE))),
                        MeshMaterial2d(materials.add(Color::srgb_from_array(SNEK_SEGMENT_COLOR))),
                        Transform::from_xyz(head_xy.0, head_xy.1, 0.0),
                        SnakeSegment {
                            pos: self.head_pos,
                            next: None,
                            prev: None,
                        },
                    ))
                    .id();
                self.snake_segments.insert(self.head_pos, entity);
            }
            false => (),
        }
        entity
    }
}

#[derive(Resource)]
pub struct Board {
    pub maximize_shenanigans: MaximizeShenanigans,
    column_count: u32,
    row_count: u32,
    snake: Option<Snake>,
}

impl Board {
    pub fn new() -> Self {
        Self {
            maximize_shenanigans: MaximizeShenanigans::new(),
            column_count: 0,
            row_count: 0,
            snake: None,
        }
    }
    pub fn set_size_once(&mut self, cols: u32, rows: u32) {
        self.column_count = cols;
        self.row_count = rows;
    }

    pub fn cols(&self) -> u32 {
        self.column_count
    }
    pub fn rows(&self) -> u32 {
        self.row_count
    }
}

fn init_game(
    mut window_res: ResMut<MainWindowDesc>,
    mut commands: Commands,
    mut window: Single<&mut Window>,
    window_ent: Query<Entity, With<PrimaryWindow>>,
) {
    let win_ent = window_ent.single().unwrap();
    commands.spawn(Camera2d);
    window.set_maximized(true);
    window_res.id = Some(win_ent);
}

fn on_window_resized(
    mut reader: MessageReader<WindowResized>,
    mut window_desc: ResMut<MainWindowDesc>,
    mut board: ResMut<Board>,
    mut window: Query<&mut Window>,
) {
    for e in reader.read() {
        if let Some(desc_ent) = window_desc.id {
            if e.window == desc_ent {
                let current_size = e.width as u64 * e.height as u64;
                if board.maximize_shenanigans.last_known_size != 0
                    && board.maximize_shenanigans.last_known_size < current_size
                {
                    board.maximize_shenanigans.assume_maximize_happened = true;
                    let mut w = window.get_mut(desc_ent).unwrap();
                    w.resizable = false;
                    w.enabled_buttons = EnabledButtons {
                        maximize: false,
                        ..default()
                    };
                }
                board.maximize_shenanigans.last_known_size = current_size;
                window_desc.width = e.width;
                window_desc.height = e.height;
                board.set_size_once(
                    e.width as u32 / TILE_SIZE as u32,
                    e.height as u32 / TILE_SIZE as u32,
                );
            }
        }
    }
}

fn draw_grid_lines(mut gizmos: Gizmos, window_desc: ResMut<MainWindowDesc>) {
    let column_count: u32 = window_desc.width as u32 / TILE_SIZE as u32;
    let row_count: u32 = window_desc.height as u32 / TILE_SIZE as u32;

    gizmos
        .grid_2d(
            Isometry2d::IDENTITY,
            UVec2::new(column_count, row_count),
            Vec2::splat(TILE_SIZE),
            Color::srgb_from_array(GRID_LINE_COLOR),
        )
        .outer_edges();
}

fn is_maximized(board: Res<Board>) -> bool {
    board.maximize_shenanigans.assume_maximize_happened
}

// TODO: actually spawn the goddamn snake!!
fn spawn_snake(
    board: &mut Board,
    rng: &mut ResMut<GameRng>,
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    let x_center = board.cols() / 2;
    let y_center = board.rows() / 2;
    let offset_from_center = SNAKE_SPAWN_AREA_MARGIN / 2.;
    let offset_x = (board.cols() as f32 * offset_from_center) as u32;
    let offset_y = (board.rows() as f32 * offset_from_center) as u32;
    let x_start = x_center - offset_x;
    let y_start = y_center - offset_y;
    let x_end = x_center + offset_x;
    let y_end = y_center + offset_y;
    let snake_pos = rng.random_pos(x_start..=x_end, y_start..=y_end);

    board.snake = Some(Snake::new(snake_pos, commands, meshes, materials, board));
    error!("Put snake at {:?}!", snake_pos);
}
fn update_snake(
    mut board: ResMut<Board>,
    mut rng: ResMut<GameRng>,
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    if board.snake.is_none() {
        spawn_snake(&mut board, &mut rng, commands, meshes, materials);
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb_from_array(SNEK_BACKGROUND_COLOR)))
        .insert_resource(GameRng::new())
        .insert_resource(Board::new())
        .insert_resource(MainWindowDesc::new())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (800, 600).into(),
                title: "BEVY THE SNEK".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, init_game)
        .add_systems(Update, on_window_resized)
        .add_systems(Update, update_snake.run_if(is_maximized))
        .add_systems(Update, draw_grid_lines)
        .run();
}
