use ggez::graphics::Color;

pub const ENABLE_VSYNC: bool = true;

pub const TEAM_SIZE: usize = 1;

pub const VIRTUAL_WIDTH: f32 = 1980.0;
pub const VIRTUAL_HEIGHT: f32 = 1080.0;

pub const ATTACK_IMAGE: &str = "/normal.png";
pub const BACKGROUND_IMAGE: &str = "/background.png";
pub const PARY_IMAGE: &str = "/pary.png";
pub const MAP_COLOR: Color = Color::new(0.0, 0.0, 0.0, 1.0);
pub const NAME_COLOR: Color = Color::new(0.6, 0.6, 0.6, 1.0);
pub const TRAIL_OPACITY: f32 = 0.15;

pub const TEAM_ONE_START_POS: [f32; 2] = [820.0, 470.0];
pub const TEAM_TWO_START_POS: [f32; 2] = [1160.0, 470.0];

pub const PLAYER_SIZE: f32 = 20.0;

// ticks per second
pub const TICK_RATE: usize = 60;

pub const MAX_SPEED: [f32; 2] = [300.0, 600.0];
pub const ACCELERATION: f32 = 5000.0;
pub const GRAVITY: f32 = 1400.0;
pub const RESISTANCE: f32 = 1400.0;
pub const WALL_SLIDE_SPEED: f32 = 0.0;

pub const RESPAWN_TIME: f32 = 2.5;
