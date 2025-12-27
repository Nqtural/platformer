use anyhow::Result;
use serde::Deserialize;
use toml;
use foundation::color::Color;

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
    team_one_color: Color,
    team_two_color: Color,
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
}

impl Config {
    pub fn get() -> Result<Self> {
        let toml_str = std::fs::read_to_string("config.toml")?;
        let config: Config = toml::from_str(&toml_str)?;
        Ok(config)
    }

    // GETTERS
    #[must_use]
    pub fn playername(&self) -> &str { &self.player.name }

    #[must_use]
    pub fn team_one_color(&self) -> Color {
        self.teams.team_one_color.clone()
    }

    #[must_use]
    pub fn team_two_color(&self) -> Color {
        self.teams.team_two_color.clone()
    }

    #[must_use]
    pub fn serverip(&self) -> &str { &self.server.ip }

    #[must_use]
    pub fn serverport(&self) -> &str { &self.server.port }

    #[must_use]
    pub fn clientip(&self) -> &str { &self.client.ip }

    #[must_use]
    pub fn clientport(&self) -> &str { &self.client.port }

    #[must_use]
    pub fn camera_bias(&self) -> f32 { self.camera.bias }

    #[must_use]
    pub fn camera_zoom(&self) -> f32 { self.camera.zoom }
}
