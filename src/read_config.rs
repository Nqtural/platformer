use ggez::{
    GameError,
    GameResult,
    graphics::Color,
};
use serde::Deserialize;
use toml;

#[derive(Deserialize)]
pub struct Config {
    player: Player,
    teams: Teams,
    camera: Camera,
    client: ClientConfig,
    server: ServerConfig,
}

#[derive(Deserialize)]
struct Player {
    name: String,
}

#[derive(Deserialize)]
struct Teams {
    team_one_color: RGB,
    team_two_color: RGB,
}

#[derive(Deserialize)]
struct RGB {
    r: f32,
    g: f32,
    b: f32,
}

impl RGB {
    fn to_color_object(&self) -> Color {
        Color::new(
            self.r,
            self.g,
            self.b,
            1.0,
        )
    }
}

#[derive(Deserialize)]
struct Camera {
    bias: f32,
    zoom: f32,
}

#[derive(Deserialize)]
struct ClientConfig {
    ip: String,
    port: String,
}

#[derive(Deserialize)]
struct ServerConfig {
    ip: String,
    port: String,
    render_server: bool,
}

impl Config {
    pub fn get() -> GameResult<Self> {
        let toml_str = std::fs::read_to_string("config.toml")
            .map_err(|e| GameError::ResourceLoadError(e.to_string()))?;
        let config: Config = toml::from_str(&toml_str)
            .map_err(|e| GameError::ConfigError(e.to_string()))?;
        Ok(config)
    }

    // GETTERS
    #[must_use]
    pub fn playername(&self) -> &str { &self.player.name }

    #[must_use]
    pub fn team_one_color(&self) -> Color {
        self.teams.team_one_color.to_color_object()
    }

    #[must_use]
    pub fn team_two_color(&self) -> Color {
        self.teams.team_two_color.to_color_object()
    }

    #[must_use]
    pub fn serverip(&self) -> &str { &self.server.ip }

    #[must_use]
    pub fn serverport(&self) -> &str { &self.server.port }

    #[must_use]
    pub fn render_server(&self) -> bool { self.server.render_server }

    #[must_use]
    pub fn clientip(&self) -> &str { &self.client.ip }

    #[must_use]
    pub fn clientport(&self) -> &str { &self.client.port }

    #[must_use]
    pub fn camera_bias(&self) -> f32 { self.camera.bias }

    #[must_use]
    pub fn camera_zoom(&self) -> f32 { self.camera.zoom }
}
