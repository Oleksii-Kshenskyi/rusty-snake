pub mod config;

use bevy::{prelude::*, window::WindowResized};

use crate::config::*;

#[derive(Resource)]
struct MainWindowDesc {
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

fn init_game(
    mut window_res: ResMut<MainWindowDesc>,
    mut commands: Commands,
    mut window: Single<&mut Window>,
    window_ent: Query<Entity, With<Window>>,
) {
    let win_ent = window_ent.single().unwrap();
    commands.spawn(Camera2d);
    window.set_maximized(true);
    window_res.id = Some(win_ent);
}

fn on_window_resized(
    mut reader: MessageReader<WindowResized>,
    mut window_desc: ResMut<MainWindowDesc>,
) {
    for e in reader.read() {
        if let Some(desc_ent) = window_desc.id {
            if e.window == desc_ent {
                window_desc.width = e.width;
                window_desc.height = e.height;
            }
        }
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb_from_array(SNEK_BACKGROUND_COLOR)))
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
        .run();
}
