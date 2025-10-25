use crate::{audio::*, interaction::Selected, objects::*};
use avian2d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Key, Modifiers};
use fundsp::hacker::*;
use std::{collections::HashMap, sync::Arc};
use syn::{parse_str, Stmt};

mod arrays;
mod atomics;
mod bools;
mod entities;
pub mod floats;
mod helpers;
mod ints;
mod nets;
mod sequencers;
mod sources;
mod statements;
mod waves;
use statements::*;

pub struct LapisPlugin;

impl Plugin for LapisPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LapisData>();
    }
}

#[derive(Resource, Default)]
pub struct LapisData {
    pub input: String,
    pub buffer: String,
    pub fmap: HashMap<String, f32>,
    pub vmap: HashMap<String, Vec<f32>>,
    pub gmap: HashMap<String, Net>,
    pub idmap: HashMap<String, NodeId>,
    pub bmap: HashMap<String, bool>,
    pub smap: HashMap<String, Shared>,
    pub wmap: HashMap<String, Arc<Wave>>,
    pub seqmap: HashMap<String, Sequencer>,
    pub eventmap: HashMap<String, EventId>,
    pub srcmap: HashMap<String, Source>,
    pub entitymap: HashMap<String, Entity>,
    pub atomic_table_map: HashMap<String, Arc<AtomicTable>>,
    // (modifiers, key, pressed)
    pub keys: HashMap<(Modifiers, Key, bool), String>,
    pub keys_active: bool,
    pub keys_repeat: bool,
    pub quiet: bool,
    pub about: bool,
    pub help: bool,
}

#[derive(SystemParam)]
pub struct Lapis<'w, 's> {
    pub data: ResMut<'w, LapisData>,
    pub commands: Commands<'w, 's>,
    pub trans_query: Query<'w, 's, &'static Transform>,
    pub mass_query: Query<'w, 's, &'static Mass>,
    pub lin_velocity_query: Query<'w, 's, &'static LinearVelocity>,
    pub ang_velocity_query: Query<'w, 's, &'static AngularVelocity>,
    pub restitution_query: Query<'w, 's, &'static Restitution>,
    pub lin_damp_query: Query<'w, 's, &'static LinearDamping>,
    pub ang_damp_query: Query<'w, 's, &'static AngularDamping>,
    pub inertia_query: Query<'w, 's, &'static AngularInertia>,
    pub sides_query: Query<'w, 's, &'static Sides>,
    pub material_ids: Query<'w, 's, &'static MeshMaterial2d<ColorMaterial>>,
    pub materials: Res<'w, Assets<ColorMaterial>>,
    pub cm_query: Query<'w, 's, &'static CenterOfMass>,
    pub friction_query: Query<'w, 's, &'static Friction>,
    pub tail_query: Query<'w, 's, &'static Tail>,
    pub layer_query: Query<'w, 's, &'static CollisionLayers>,
    pub body_query: Query<'w, 's, &'static RigidBody>,
    pub sensor_query: Query<'w, 's, &'static Sensor>,
    pub fixed_query: Query<'w, 's, &'static FixedJoint>,
    pub distance_query: Query<'w, 's, &'static DistanceJoint>,
    pub revolute_query: Query<'w, 's, &'static RevoluteJoint>,
    pub prismatic_query: Query<'w, 's, &'static PrismaticJoint>,
    pub time: ResMut<'w, Time<Virtual>>,
    pub selected_query: Query<'w, 's, Entity, With<Selected>>,
    pub audio_out: ResMut<'w, AudioOutput>,
    pub input_receiver: Res<'w, AudioInputReceiver1>,
    pub in_stream_config: Res<'w, InStreamConfig>,
    pub out_stream_config: Res<'w, OutStreamConfig>,
}

impl Lapis<'_, '_> {
    pub fn drop(&mut self, k: &str) {
        self.data.fmap.remove(k);
        self.data.vmap.remove(k);
        self.data.gmap.remove(k);
        self.data.idmap.remove(k);
        self.data.bmap.remove(k);
        self.data.smap.remove(k);
        self.data.wmap.remove(k);
        self.data.seqmap.remove(k);
        self.data.eventmap.remove(k);
        self.data.srcmap.remove(k);
        self.data.entitymap.remove(k);
        self.data.atomic_table_map.remove(k);
    }
    pub fn eval(&mut self, input: &str) {
        if !input.is_empty() {
            self.data.buffer.push('\n');
            self.data.buffer.push_str(input);
            match parse_str::<Stmt>(&format!("{{{input}}}")) {
                Ok(stmt) => {
                    let out = eval_stmt(stmt, self);
                    self.data.buffer.push_str(&out);
                }
                Err(err) => {
                    self.data.buffer.push_str(&format!("\n// error: {err}"));
                }
            }
        }
    }
    pub fn eval_input(&mut self) {
        if !self.data.input.is_empty() {
            match parse_str::<Stmt>(&format!("{{{}}}", self.data.input)) {
                Ok(stmt) => {
                    self.data.buffer.push('\n');
                    let input = std::mem::take(&mut self.data.input);
                    let out = eval_stmt(stmt, self);
                    self.data.buffer.push_str(&input);
                    self.data.buffer.push_str(&out);
                }
                Err(err) => {
                    self.data.buffer.push_str(&format!("\n// error: {err}"));
                }
            }
        }
    }
    pub fn quiet_eval(&mut self, input: &str) {
        if let Ok(stmt) = parse_str::<Stmt>(&format!("{{{input}}}")) {
            eval_stmt(stmt, self);
        }
    }
}
