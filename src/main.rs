pub mod config;

use std::collections::{HashMap, HashSet};

use bevy::{
    prelude::*,
    window::{EnabledButtons, PrimaryWindow, WindowResized},
};
use rand::prelude::*;

use crate::config::*;

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BoardPos {
    pub x: i32,
    pub y: i32,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
impl Direction {
    pub fn to_delta(&self) -> (i32, i32) {
        match self {
            Direction::Up => (0, -1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
            Direction::Down => (0, 1),
        }
    }
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
        x_range: impl rand::distr::uniform::SampleRange<i32>,
        y_range: impl rand::distr::uniform::SampleRange<i32>,
    ) -> BoardPos {
        BoardPos {
            x: self.rng.random_range(x_range),
            y: self.rng.random_range(y_range),
        }
    }

    pub fn random_direction(&mut self) -> Direction {
        let random_int_dir = self.rng.random_range(0..3);
        match random_int_dir {
            0 => Direction::Up,
            1 => Direction::Right,
            2 => Direction::Down,
            3 => Direction::Left,
            _ => panic!("GameRng::random_direction(): 5th direction picked???"),
        }
    }
}

#[derive(Debug)]
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

#[derive(Component, Clone)]
pub struct SnakeSegment {
    pub pos: BoardPos,
    pub next: Option<BoardPos>,
    pub prev: Option<BoardPos>,
    pub direction: Direction,
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
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
        rng: &mut ResMut<GameRng>,
        board: &Board,
    ) -> Self {
        let mut snake = Self {
            snake_segments: HashMap::new(),
            head_pos: start_pos,
            tail_pos: start_pos,
            direction: rng.random_direction(),
        };
        let _ = snake.spawn_segment(commands, meshes, materials, board);
        snake
    }

    pub fn spawn_segment(
        &mut self,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
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
                            direction: Direction::Right,
                        },
                    ))
                    .id();
                self.snake_segments.insert(self.head_pos, entity);
            }
            false => (),
        }
        entity
    }

    pub fn segment_by_pos(&self, pos: BoardPos) -> Entity {
        let ent = self.snake_segments.get(&pos).unwrap();
        *ent
    }
}

#[derive(Component, Clone, Debug)]
pub struct Apple;

// FIXME: Board should not hold references to snake, apple, or anything else. Rely on Bevy's ECS for that. Decouple board (maximize + column/row counts)
#[derive(Resource, Debug)]
pub struct Board {
    pub maximize_shenanigans: MaximizeShenanigans,
    column_count: u32,
    row_count: u32,
    snake: Option<Entity>,
    apple: Option<BoardPos>,
}

impl Board {
    pub fn new() -> Self {
        Self {
            maximize_shenanigans: MaximizeShenanigans::new(),
            column_count: 0,
            row_count: 0,
            snake: None,
            apple: None,
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

fn snake_missing(board: Res<Board>) -> bool {
    board.snake.is_none()
}
fn spawn_snake(
    mut board: ResMut<Board>,
    mut rng: ResMut<GameRng>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let x_center = board.cols() / 2;
    let y_center = board.rows() / 2;
    let offset_from_center = SNAKE_SPAWN_AREA_MARGIN / 2.;
    let offset_x = (board.cols() as f32 * offset_from_center) as u32;
    let offset_y = (board.rows() as f32 * offset_from_center) as u32;
    let x_start = (x_center - offset_x) as i32;
    let y_start = (y_center - offset_y) as i32;
    let x_end = (x_center + offset_x) as i32;
    let y_end = (y_center + offset_y) as i32;
    let snake_pos = rng.random_pos(x_start..=x_end, y_start..=y_end);

    let snake_struct = Snake::new(
        snake_pos,
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut rng,
        &board,
    );
    board.snake = Some(dbg!(commands.spawn(snake_struct).id()));
}

// NOTE: this can bite me in the ass in the future. I may not want to instantly spawn apple each time it is missing. Or this may work fine. Check this AFTER interaction for snake eating the apple is implemented.
fn apple_missing(board: Res<Board>) -> bool {
    board.apple.is_none()
}
fn grow_apple(
    mut board: ResMut<Board>,
    mut snake_query: Query<&Snake>,
    mut rng: ResMut<GameRng>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let conflicting_tiles = if let Some(snake_ent) = board.snake {
        let snek = snake_query.get_mut(snake_ent).unwrap();
        snek.snake_segments.keys().cloned().collect()
    } else {
        HashSet::new()
    };
    let apple_pos = loop {
        let maybe_pos = rng.random_pos(0..(board.cols() as i32), 0..(board.rows() as i32));
        if !conflicting_tiles.contains(&maybe_pos) {
            break maybe_pos;
        }
    };
    let apple_world_pos = board_pos_to_world(&apple_pos, &board);
    board.apple = Some(apple_pos);

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(TILE_SIZE, TILE_SIZE))),
        MeshMaterial2d(materials.add(Color::srgb_from_array(APPLE_COLOR))),
        Transform::from_xyz(apple_world_pos.0, apple_world_pos.1, 0.0),
        Apple,
    ));
}

// TODO: Already did move board pos method for Direction.
// Now add Direction to Snake/Snake Segments and try to move the snake.
// Right now I believe that involves first moving all the snake segments in the Board resource,
// And second (potentially a separate system) - moving transform components (so that the movement reflects on the screen).
// Does each segment has its own movement direction? If so, when do segment directions change? Automatically somehow, or do I need to update segment directions manually as well?

// TODO: Develop two chained systems.
// 1. First one figures out and updates directions of all current snake segments.
// 2. Second one that runs in chain after the first one, moves all segments into stated directions starting with the head and going further back until the end of the snake.
// NOTE: MOVEMENT RULES:
// - If segment is head, move in the current snake direction.
// - If segment is non-head, move in the direction where the previous (closer-to-head) segment just was.

// REFACTOR: IMPORTANT: Just move the TAIL into where the HEAD Is supposed to go!! That's it????
fn update_segment_directions(
    board: Res<Board>,
    snake_query: Query<&Snake>,
    mut segment_query: Query<&mut SnakeSegment>,
) {
    if let Some(snake_ent) = board.snake {
        let snek = snake_query.get(snake_ent).unwrap();
        let mut current_segment = snek.segment_by_pos(snek.head_pos);
        let mut directions_to_update: Vec<(Entity, Direction)> = vec![];

        loop {
            let current_segment_ref = segment_query.get(current_segment).unwrap();

            let dir = if let Some(prev_pos) = current_segment_ref.prev {
                let segment_ent = snek.segment_by_pos(prev_pos);
                let prev_segment = segment_query.get(segment_ent).unwrap();
                prev_segment.direction.clone()
            } else {
                snek.direction.clone()
            };
            directions_to_update.push((current_segment, dir));

            if let Some(next_pos) = current_segment_ref.next {
                current_segment = *snek.snake_segments.get(&next_pos).unwrap();
            } else {
                break;
            }
        }

        for (entity, direction) in directions_to_update {
            segment_query.get_mut(entity).unwrap().direction = direction;
        }
    }
}

fn move_board_pos(pos: &BoardPos, delta: (i32, i32)) -> BoardPos {
    BoardPos {
        x: pos.x + delta.0,
        y: pos.y + delta.1,
    }
}

fn move_snake_segments(
    board: Res<Board>,
    mut snake_query: Query<&mut Snake>,
    mut segment_query: Query<&mut SnakeSegment>,
    mut segment_transform_q: Query<&mut Transform, With<SnakeSegment>>,
) {
    if let Some(snake_ent) = board.snake {
        let mut snek = snake_query.get_mut(snake_ent).unwrap();
        for mut segment in segment_query.iter_mut() {
            let segment_entity = snek.segment_by_pos(segment.pos);
            let new_pos = move_board_pos(&segment.pos, segment.direction.to_delta());
            let new_world_pos = board_pos_to_world(&new_pos, &board);

            segment.pos = new_pos;
            snek.snake_segments.remove(&segment.pos);
            snek.snake_segments.insert(new_pos, segment_entity);

            let mut t = segment_transform_q.get_mut(segment_entity).unwrap();
            t.translation.x = new_world_pos.0;
            t.translation.y = new_world_pos.1;
        }
    }
}

fn monitor_game_over(board: Res<Board>, segments_q: Query<&SnakeSegment>) {
    for segment in segments_q.iter() {
        if segment.pos.x < 0
            || segment.pos.x >= board.cols() as i32
            || segment.pos.y < 0
            || segment.pos.y >= board.rows() as i32
        {
            error!("Sorry, Game Over! You Lost ^_^");
            std::process::exit(0);
        }
    }
}

// REFACTOR: think about replacing MaximizeShenanigans with a complex PreInit state that includes maximization + snake spawning. Only when the window is maximized + snake has been spawned, we can conclude that the Init has finished and change state to GameRunning.

// TODO: this works, but it allows snake to move into the exact opposite direciton without turning around. This needs to be forbidden.
fn change_snake_direction(
    keypress: Res<ButtonInput<KeyCode>>,
    board: Res<Board>,
    mut snake: Query<&mut Snake>,
) {
    if let Some(snake_ent) = board.snake {
        let mut snek = snake.get_mut(snake_ent).unwrap();
        if keypress.just_pressed(KeyCode::ArrowUp) || keypress.just_pressed(KeyCode::KeyW) {
            snek.direction = Direction::Up;
        } else if keypress.just_pressed(KeyCode::ArrowRight) || keypress.just_pressed(KeyCode::KeyD)
        {
            snek.direction = Direction::Right;
        } else if keypress.just_pressed(KeyCode::ArrowDown) || keypress.just_pressed(KeyCode::KeyS)
        {
            snek.direction = Direction::Down;
        } else if keypress.just_pressed(KeyCode::ArrowLeft) || keypress.just_pressed(KeyCode::KeyA)
        {
            snek.direction = Direction::Left
        }
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb_from_array(SNEK_BACKGROUND_COLOR)))
        .insert_resource(GameRng::new())
        .insert_resource(Board::new())
        .insert_resource(MainWindowDesc::new())
        .insert_resource(Time::<Fixed>::from_seconds(0.5))
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
        .add_systems(
            Update,
            spawn_snake.run_if(is_maximized).run_if(snake_missing),
        )
        .add_systems(
            Update,
            grow_apple
                .run_if(is_maximized)
                .run_if(apple_missing)
                .after(spawn_snake),
        )
        .add_systems(
            Update,
            change_snake_direction
                .run_if(is_maximized)
                .after(spawn_snake),
        )
        .add_systems(
            FixedUpdate,
            (
                update_segment_directions,
                move_snake_segments,
                monitor_game_over,
            )
                .chain()
                .run_if(is_maximized),
        )
        .add_systems(Update, draw_grid_lines)
        .run();
}
