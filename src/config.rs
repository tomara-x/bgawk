use crate::objects::AttractionFactor;
use crate::ui::ScaleFactor;
use avian2d::prelude::Gravity;
use bevy::app::{App, Plugin, PostStartup};
use bevy::prelude::{Query, Res, ResMut, Resource, Window};
use bevy::time::{Time, Virtual};
use clap::Parser;
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use xdg::BaseDirectories;

pub struct ConfigPlugin;

#[derive(Parser, Debug, Resource, Serialize, Deserialize)]
#[command(version, about, long_about = None)]
pub struct Config {
    #[arg(long, default_value_t = false)]
    pub pause: bool,

    #[arg(long, default_value_t = 0.0)]
    pub gravity_x: f32,

    #[arg(long, default_value_t = 0.0)]
    pub gravity_y: f32,

    #[arg(long, default_value_t = 0.01)]
    pub attraction: f32,

    #[arg(long, default_value_t = 1.0)]
    pub scale_factor: f32,

    #[arg(long, default_value_t = 1280.0)]
    pub win_width: f32,

    #[arg(long, default_value_t = 720.0)]
    pub win_height: f32,
}

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        let xdg_dirs = BaseDirectories::with_prefix("bgawk").unwrap();
        let config_path = xdg_dirs.place_config_file("config.toml").unwrap();

        let config: Config = Figment::new()
            .merge(Serialized::defaults(Config::parse()))
            .merge(Toml::file(config_path))
            .extract()
            .unwrap();

        app.insert_resource(config)
            .add_systems(PostStartup, configure);
    }
}

fn configure(
    config: Res<Config>,
    mut time: ResMut<Time<Virtual>>,
    mut gravity: ResMut<Gravity>,
    mut attraction_factor: ResMut<AttractionFactor>,
    mut scale_factor: ResMut<ScaleFactor>,
    mut win: Query<&mut Window>,
) {
    if config.pause {
        time.pause();
    }

    gravity.0.x = config.gravity_x;
    gravity.0.y = config.gravity_y;

    attraction_factor.0 = config.attraction;

    scale_factor.0 = config.scale_factor;
    let res = &mut win.single_mut().resolution;
    res.set_scale_factor(config.scale_factor);
    res.set(config.win_width, config.win_height);
}
