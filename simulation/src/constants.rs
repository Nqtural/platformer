use foundation::color::Color;

pub const VIRTUAL_WIDTH: f32 = 1980.0;
pub const VIRTUAL_HEIGHT: f32 = 1080.0;

pub const MAP_COLOR: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
pub const NAME_COLOR: Color = Color { r: 0.6, g: 0.6, b: 0.6, a: 1.0 };
pub const TRAIL_OPACITY: f32 = 0.15;

pub const PLAYER_SIZE: f32 = 20.0;

// ticks per second
pub const TICK_RATE: usize = 60;

pub const MAX_SPEED: [f32; 2] = [300.0, 600.0];
pub const ACCELERATION: f32 = 5000.0;
pub const GRAVITY: f32 = 1400.0;
pub const RESISTANCE: f32 = 1400.0;
pub const WALL_SLIDE_SPEED: f32 = 0.0;

pub const RESPAWN_TIME: f32 = 2.5;
