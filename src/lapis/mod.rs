use crate::objects::*;
use avian2d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, SizedSample, Stream,
};
use crossbeam_channel::{bounded, Receiver, Sender};
use egui::KeyboardShortcut;
use fundsp::hacker32::*;
use std::{collections::HashMap, sync::Arc};
use syn::*;

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
mod units;
mod waves;
use {
    arrays::*, atomics::*, bools::*, entities::*, floats::*, helpers::*, ints::*, nets::*,
    sequencers::*, sources::*, statements::*, units::*, waves::*,
};

pub struct LapisPlugin;

impl Plugin for LapisPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_lapis)
            .add_systems(Update, toggle_help)
            .add_observer(set_out_device)
            .add_observer(set_in_device);
    }
}

fn toggle_help(keyboard_input: Res<ButtonInput<KeyCode>>, mut lapis: ResMut<LapisData>) {
    if keyboard_input.just_pressed(KeyCode::F1) {
        lapis.help = !lapis.help;
    } else if keyboard_input.just_pressed(KeyCode::Escape) {
        lapis.help = false;
    }
}

#[derive(Resource)]
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
    pub slot: Slot,
    pub receivers: (Receiver<f32>, Receiver<f32>),
    pub keys: Vec<(KeyboardShortcut, String)>,
    pub keys_active: bool,
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
}

impl Lapis<'_, '_> {
    pub fn drop(&mut self, k: &String) {
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
    }
    pub fn eval(&mut self, input: &str) {
        if !input.is_empty() {
            self.data.buffer.push('\n');
            self.data.buffer.push_str(input);
            match parse_str::<Stmt>(&format!("{{{}}}", input)) {
                Ok(stmt) => {
                    let out = eval_stmt(stmt, self);
                    self.data.buffer.push_str(&out);
                }
                Err(err) => {
                    self.data.buffer.push_str(&format!("\n// error: {}", err));
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
                    self.data.buffer.push_str(&format!("\n// error: {}", err));
                }
            }
        }
    }
    pub fn quiet_eval(&mut self, input: &str) {
        if let Ok(stmt) = parse_str::<Stmt>(&format!("{{{}}}", input)) {
            eval_stmt(stmt, self);
        }
    }
}

struct OutStream(Option<Stream>);
struct InStream(Option<Stream>);

fn init_lapis(world: &mut World) {
    let (slot, slot_back) = Slot::new(Box::new(dc(0.) | dc(0.)));
    let (ls, lr) = bounded(4096);
    let (rs, rr) = bounded(4096);
    let stream = default_out_device(slot_back);
    world.insert_non_send_resource(OutStream(stream));
    let stream = default_in_device(ls, rs);
    world.insert_non_send_resource(InStream(stream));
    world.insert_resource(LapisData {
        input: String::new(),
        buffer: String::new(),
        fmap: HashMap::new(),
        vmap: HashMap::new(),
        gmap: HashMap::new(),
        idmap: HashMap::new(),
        bmap: HashMap::new(),
        smap: HashMap::new(),
        wmap: HashMap::new(),
        seqmap: HashMap::new(),
        eventmap: HashMap::new(),
        srcmap: HashMap::new(),
        entitymap: HashMap::new(),
        slot,
        receivers: (lr, rr),
        keys: Vec::new(),
        keys_active: false,
        quiet: false,
        about: false,
        help: false,
    });
}

fn default_out_device(slot: SlotBackend) -> Option<Stream> {
    let host = cpal::default_host();
    if let Some(device) = host.default_output_device() {
        if let Ok(default_config) = device.default_output_config() {
            let mut config = default_config.config();
            config.channels = 2;
            return match default_config.sample_format() {
                cpal::SampleFormat::F32 => run::<f32>(&device, &config, slot),
                cpal::SampleFormat::I16 => run::<i16>(&device, &config, slot),
                cpal::SampleFormat::U16 => run::<u16>(&device, &config, slot),
                format => {
                    eprintln!("unsupported sample format: {}", format);
                    None
                }
            };
        }
    }
    None
}

fn default_in_device(ls: Sender<f32>, rs: Sender<f32>) -> Option<Stream> {
    let host = cpal::default_host();
    if let Some(device) = host.default_input_device() {
        if let Ok(config) = device.default_input_config() {
            return match config.sample_format() {
                cpal::SampleFormat::F32 => run_in::<f32>(&device, &config.into(), ls, rs),
                cpal::SampleFormat::I16 => run_in::<i16>(&device, &config.into(), ls, rs),
                cpal::SampleFormat::U16 => run_in::<u16>(&device, &config.into(), ls, rs),
                format => {
                    eprintln!("unsupported sample format: {}", format);
                    None
                }
            };
        }
    }
    None
}

#[derive(Event)]
struct OutDevice(usize, usize);

fn set_out_device(
    trig: Trigger<OutDevice>,
    mut stream: NonSendMut<OutStream>,
    mut lapis: ResMut<LapisData>,
) {
    let OutDevice(h, d) = *trig.event();
    if let Some(host_id) = cpal::ALL_HOSTS.get(h) {
        if let Ok(host) = cpal::host_from_id(*host_id) {
            if let Ok(mut devices) = host.output_devices() {
                if let Some(device) = devices.nth(d) {
                    if let Ok(default_config) = device.default_output_config() {
                        let mut config = default_config.config();
                        config.channels = 2;
                        let (slot, slot_back) = Slot::new(Box::new(dc(0.) | dc(0.)));
                        lapis.slot = slot;
                        let s = match default_config.sample_format() {
                            cpal::SampleFormat::F32 => run::<f32>(&device, &config, slot_back),
                            cpal::SampleFormat::I16 => run::<i16>(&device, &config, slot_back),
                            cpal::SampleFormat::U16 => run::<u16>(&device, &config, slot_back),
                            format => {
                                eprintln!("unsupported sample format: {}", format);
                                None
                            }
                        };
                        if s.is_some() {
                            stream.0 = s;
                        }
                    }
                }
            }
        }
    }
}

#[derive(Event)]
struct InDevice(usize, usize);

fn set_in_device(
    trig: Trigger<InDevice>,
    mut stream: NonSendMut<InStream>,
    mut lapis: ResMut<LapisData>,
) {
    let InDevice(h, d) = *trig.event();
    if let Some(host_id) = cpal::ALL_HOSTS.get(h) {
        if let Ok(host) = cpal::host_from_id(*host_id) {
            if let Ok(mut devices) = host.input_devices() {
                if let Some(device) = devices.nth(d) {
                    if let Ok(config) = device.default_input_config() {
                        let (ls, lr) = bounded(4096);
                        let (rs, rr) = bounded(4096);
                        lapis.receivers = (lr, rr);
                        let s = match config.sample_format() {
                            cpal::SampleFormat::F32 => {
                                run_in::<f32>(&device, &config.into(), ls, rs)
                            }
                            cpal::SampleFormat::I16 => {
                                run_in::<i16>(&device, &config.into(), ls, rs)
                            }
                            cpal::SampleFormat::U16 => {
                                run_in::<u16>(&device, &config.into(), ls, rs)
                            }
                            format => {
                                eprintln!("unsupported sample format: {}", format);
                                None
                            }
                        };
                        if s.is_some() {
                            stream.0 = s;
                        }
                    }
                }
            }
        }
    }
}

fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig, slot: SlotBackend) -> Option<Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let mut slot = BlockRateAdapter::new(Box::new(slot));

    let mut next_value = move || {
        let (l, r) = slot.get_stereo();
        (
            if l.is_normal() { l.clamp(-1., 1.) } else { 0. },
            if r.is_normal() { r.clamp(-1., 1.) } else { 0. },
        )
    };
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| write_data(data, &mut next_value),
        err_fn,
        None,
    );
    if let Ok(stream) = stream {
        if let Ok(()) = stream.play() {
            return Some(stream);
        }
    }
    None
}

fn write_data<T>(output: &mut [T], next_sample: &mut dyn FnMut() -> (f32, f32))
where
    T: SizedSample + FromSample<f32>,
{
    for frame in output.chunks_mut(2) {
        let sample = next_sample();
        frame[0] = T::from_sample(sample.0);
        frame[1] = T::from_sample(sample.1);
    }
}

fn run_in<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    ls: Sender<f32>,
    rs: Sender<f32>,
) -> Option<Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            read_data(data, channels, ls.clone(), rs.clone())
        },
        err_fn,
        None,
    );
    if let Ok(stream) = stream {
        if let Ok(()) = stream.play() {
            return Some(stream);
        }
    }
    None
}

fn read_data<T>(input: &[T], channels: usize, ls: Sender<f32>, rs: Sender<f32>)
where
    T: SizedSample,
    f32: FromSample<T>,
{
    for frame in input.chunks(channels) {
        for (channel, sample) in frame.iter().enumerate() {
            if channel & 1 == 0 {
                let _ = ls.try_send(sample.to_sample::<f32>());
            } else {
                let _ = rs.try_send(sample.to_sample::<f32>());
            }
        }
    }
}
