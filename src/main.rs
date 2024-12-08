#![allow(clippy::too_many_arguments, clippy::map_entry)]

use avian2d::prelude::*;
use bevy::{
    core_pipeline::{
        bloom::{Bloom, BloomCompositeMode},
        tonemapping::Tonemapping,
    },
    prelude::*,
};
use bevy_pancam::*;

mod interaction;
mod joints;
mod lapis;
mod objects;
mod ui;
use {interaction::*, joints::*, lapis::*, objects::*, ui::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: String::from("bgawk!"),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PanCamPlugin)
        .add_plugins(InteractPlugin)
        .add_plugins(ObjectsPlugin)
        .add_plugins(JointsPlugin)
        .add_plugins(UiPlugin)
        .add_plugins(PhysicsPlugins::default())
        //.add_plugins(PhysicsDebugPlugin::default())
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Gravity::ZERO)
        .add_systems(Startup, setup)
        .insert_resource(Lapis::new())
        .insert_resource(UpdateCode::default())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera {
            hdr: true,
            ..default()
        },
        Camera2d,
        Tonemapping::TonyMcMapface,
        Transform::from_translation(Vec3::Z),
        Bloom {
            intensity: 0.4,
            low_frequency_boost: 0.6,
            composite_mode: BloomCompositeMode::Additive,
            ..default()
        },
        PanCam {
            grab_buttons: vec![MouseButton::Left],
            move_keys: bevy_pancam::DirectionKeys::arrows(),
            enabled: false,
            ..default()
        },
    ));
}
