use ggez::graphics::Color;

pub const ENABLE_VSYNC: bool = true;

pub const C_TEAM: usize = 0;
pub const C_PLAYER: usize = 0;

pub const REQUIRED_PLAYERS: usize = 2;

pub const SERVER_IP: &str = "0.0.0.0";
pub const SERVER_PORT: &str = "4000";

pub const VIRTUAL_WIDTH: f32 = 1980.0;
pub const VIRTUAL_HEIGHT: f32 = 1080.0;

pub const BACKGROUND_IMAGE: &str = "/background.png";
pub const MAP_COLOR: Color = Color::new(0.0, 0.0, 0.0, 1.0);
pub const NAME_COLOR: Color = Color::new(0.6, 0.6, 0.6, 1.0);
pub const TEAM_ONE_COLOR: Color = Color::new(0.0, 0.0, 1.0, 1.0);
pub const TEAM_TWO_COLOR: Color = Color::new(1.0, 0.0, 0.0, 1.0);
pub const TRAIL_OPACITY: f32 = 0.15;

pub const TEAM_ONE_START_POS: [f32; 2] = [250.0, 300.0];
pub const TEAM_TWO_START_POS: [f32; 2] = [550.0, 300.0];

pub const PLAYER_SIZE: f32 = 20.0;

pub const MAX_SPEED: [f32; 2] = [300.0, 600.0];
pub const ACCELERATION: f32 = 5000.0;
pub const GRAVITY: f32 = 1400.0;
pub const RESISTANCE: f32 = 1400.0;
pub const WALL_SLIDE_SPEED: f32 = 0.0;

pub const RESPAWN_TIME: f32 = 2.5;
