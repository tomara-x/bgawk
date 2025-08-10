#![allow(clippy::too_many_arguments)]

use avian2d::prelude::*;
use bevy::{
    core_pipeline::{
        bloom::{Bloom, BloomCompositeMode},
        tonemapping::Tonemapping,
    },
    prelude::*,
    winit::{UpdateMode, WinitSettings},
};
use bevy_pancam::*;
use std::time::Duration;

mod audio;
mod config;
mod interaction;
mod joints;
mod lapis;
mod objects;
mod ui;

use config::ConfigPlugin;
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
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::reactive_low_power(Duration::from_secs_f64(1. / 60.)),
            unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs_f64(1. / 30.)),
        })
        .add_plugins(PanCamPlugin)
        .add_plugins(audio::AudioPlugin)
        .add_plugins(InteractPlugin)
        .add_plugins(ObjectsPlugin)
        .add_plugins(JointsPlugin)
        .add_plugins(UiPlugin)
        .add_plugins(LapisPlugin)
        .add_plugins(PhysicsPlugins::default().with_length_unit(100.))
        .add_plugins(PhysicsDebugPlugin::default())
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Gravity::ZERO)
        .insert_resource(SleepingThreshold {
            linear: -1.,
            angular: -1.,
        })
        .add_systems(Startup, setup)
        .add_plugins(ConfigPlugin)
        .run();
}

fn setup(mut commands: Commands, mut config_store: ResMut<GizmoConfigStore>) {
    // disable avian debug gizmos
    config_store.config_mut::<PhysicsGizmos>().0.enabled = false;
    // camera
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
