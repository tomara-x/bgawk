use crate::objects::*;
use avian2d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, SizedSample,
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
mod units;
mod waves;
use {
    arrays::*, atomics::*, bools::*, entities::*, floats::*, helpers::*, ints::*, nets::*,
    sequencers::*, sources::*, units::*, waves::*,
};

pub struct LapisPlugin;

impl Plugin for LapisPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LapisData::new());
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

impl LapisData {
    pub fn new() -> Self {
        let (slot, slot_back) = Slot::new(Box::new(dc(0.) | dc(0.)));
        let (ls, lr) = bounded(4096);
        let (rs, rr) = bounded(4096);
        default_out_device(slot_back);
        default_in_device(ls, rs);
        LapisData {
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
        }
    }
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
                    eval_stmt(stmt, self, false);
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
                    let input = self.data.input.clone();
                    self.data.buffer.push_str(&input);
                    eval_stmt(stmt, self, false);
                    self.data.input.clear();
                }
                Err(err) => {
                    self.data.buffer.push_str(&format!("\n// error: {}", err));
                }
            }
        }
    }
    pub fn quiet_eval(&mut self, input: &str) {
        if let Ok(stmt) = parse_str::<Stmt>(&format!("{{{}}}", input)) {
            eval_stmt(stmt, self, true);
        }
    }
}

#[allow(clippy::map_entry)]
fn eval_stmt(s: Stmt, lapis: &mut Lapis, quiet: bool) {
    match s {
        Stmt::Local(expr) => {
            if let Some(k) = pat_ident(&expr.pat) {
                if let Some(expr) = expr.init {
                    if let Some(v) = eval_float(&expr.expr, lapis) {
                        lapis.drop(&k);
                        lapis.data.fmap.insert(k, v);
                    } else if let Some(v) = eval_net(&expr.expr, lapis) {
                        lapis.drop(&k);
                        lapis.data.gmap.insert(k, v);
                    } else if let Some(arr) = eval_vec(&expr.expr, lapis) {
                        lapis.drop(&k);
                        lapis.data.vmap.insert(k, arr);
                    } else if let Some(id) =
                        method_nodeid(&expr.expr, lapis).or(path_nodeid(&expr.expr, lapis))
                    {
                        lapis.drop(&k);
                        lapis.data.idmap.insert(k, id);
                    } else if let Some(b) = eval_bool(&expr.expr, lapis) {
                        lapis.drop(&k);
                        lapis.data.bmap.insert(k, b);
                    } else if let Some(s) = eval_shared(&expr.expr, lapis) {
                        lapis.drop(&k);
                        lapis.data.smap.insert(k, s);
                    } else if let Some(w) = eval_wave(&expr.expr, lapis) {
                        lapis.drop(&k);
                        let wave = Arc::new(w);
                        lapis.data.wmap.insert(k, wave);
                    } else if let Some(seq) = call_seq(&expr.expr, lapis) {
                        lapis.drop(&k);
                        lapis.data.seqmap.insert(k, seq);
                    } else if let Some(source) = eval_source(&expr.expr, lapis) {
                        lapis.drop(&k);
                        lapis.data.srcmap.insert(k, source);
                    } else if let Some(event) =
                        method_eventid(&expr.expr, lapis).or(path_eventid(&expr.expr, lapis))
                    {
                        lapis.drop(&k);
                        lapis.data.eventmap.insert(k, event);
                    } else if let Some(entity) = eval_entity(&expr.expr, lapis) {
                        lapis.drop(&k);
                        lapis.data.entitymap.insert(k, entity);
                    }
                }
            }
        }
        Stmt::Expr(expr, _) => match expr {
            Expr::Assign(expr) => match *expr.left {
                Expr::Path(_) => {
                    let Some(ident) = nth_path_ident(&expr.left, 0) else {
                        return;
                    };
                    if let Some(f) = eval_float(&expr.right, lapis) {
                        if let Some(var) = lapis.data.fmap.get_mut(&ident) {
                            *var = f;
                        }
                    } else if lapis.data.gmap.contains_key(&ident) {
                        if let Some(g) = eval_net(&expr.right, lapis) {
                            lapis.data.gmap.insert(ident, g);
                        }
                    } else if lapis.data.vmap.contains_key(&ident) {
                        if let Some(a) = eval_vec(&expr.right, lapis) {
                            lapis.data.vmap.insert(ident, a);
                        }
                    } else if let Some(id) =
                        method_nodeid(&expr.right, lapis).or(path_nodeid(&expr.right, lapis))
                    {
                        if let Some(var) = lapis.data.idmap.get_mut(&ident) {
                            *var = id;
                        }
                    } else if let Some(b) = eval_bool(&expr.right, lapis) {
                        if let Some(var) = lapis.data.bmap.get_mut(&ident) {
                            *var = b;
                        }
                    } else if let Some(s) = eval_shared(&expr.right, lapis) {
                        if let Some(var) = lapis.data.smap.get_mut(&ident) {
                            *var = s;
                        }
                    } else if let Some(s) = eval_source(&expr.right, lapis) {
                        if let Some(var) = lapis.data.srcmap.get_mut(&ident) {
                            *var = s;
                        }
                    } else if let Some(event) =
                        method_eventid(&expr.right, lapis).or(path_eventid(&expr.right, lapis))
                    {
                        if let Some(var) = lapis.data.eventmap.get_mut(&ident) {
                            *var = event;
                        }
                    } else if let Some(entity) = eval_entity(&expr.right, lapis) {
                        if let Some(var) = lapis.data.entitymap.get_mut(&ident) {
                            *var = entity;
                        }
                    }
                }
                Expr::Index(left) => {
                    if let Some(k) = nth_path_ident(&left.expr, 0) {
                        if let Some(index) = eval_usize(&left.index, lapis) {
                            if let Some(right) = eval_float(&expr.right, lapis) {
                                if let Some(vec) = lapis.data.vmap.get_mut(&k) {
                                    if let Some(v) = vec.get_mut(index) {
                                        *v = right;
                                    }
                                }
                            }
                        }
                    }
                }
                Expr::Lit(left) => {
                    if let Lit::Str(left) = left.lit {
                        if let Expr::Lit(right) = *expr.right {
                            if let Some(shortcut) = parse_shortcut(left.value()) {
                                lapis.data.keys.retain(|x| x.0 != shortcut);
                                if let Lit::Str(right) = right.lit {
                                    let code = right.value();
                                    if !code.is_empty() {
                                        lapis.data.keys.push((shortcut, code));
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
            Expr::ForLoop(expr) => {
                let Some(ident) = pat_ident(&expr.pat) else {
                    return;
                };
                let bounds = range_bounds(&expr.expr, lapis);
                let arr = eval_vec(&expr.expr, lapis);
                let tmp = lapis.data.fmap.remove(&ident);
                if let Some((r0, r1)) = bounds {
                    for i in r0..r1 {
                        lapis.data.fmap.insert(ident.clone(), i as f32);
                        for stmt in &expr.body.stmts {
                            eval_stmt(stmt.clone(), lapis, quiet);
                        }
                    }
                } else if let Some(arr) = arr {
                    for i in arr {
                        lapis.data.fmap.insert(ident.clone(), i);
                        for stmt in &expr.body.stmts {
                            eval_stmt(stmt.clone(), lapis, quiet);
                        }
                    }
                }
                if let Some(old) = tmp {
                    lapis.data.fmap.insert(ident, old);
                } else {
                    lapis.data.fmap.remove(&ident);
                }
            }
            Expr::If(expr) => {
                if let Some(cond) = eval_bool(&expr.cond, lapis) {
                    if cond {
                        let expr = Expr::Block(ExprBlock {
                            attrs: Vec::new(),
                            label: None,
                            block: expr.then_branch,
                        });
                        eval_stmt(Stmt::Expr(expr, None), lapis, quiet);
                    } else if let Some((_, else_branch)) = expr.else_branch {
                        eval_stmt(Stmt::Expr(*else_branch, None), lapis, quiet);
                    }
                }
            }
            Expr::Block(expr) => {
                for stmt in expr.block.stmts {
                    eval_stmt(stmt, lapis, quiet);
                }
            }
            _ => {
                if let Some(n) = eval_float(&expr, lapis) {
                    if !quiet {
                        lapis.data.buffer.push_str(&format!("\n// {:?}", n));
                    }
                } else if let Some(arr) = eval_vec(&expr, lapis) {
                    if !quiet {
                        lapis.data.buffer.push_str(&format!("\n// {:?}", arr));
                    }
                } else if let Some(mut g) = eval_net_cloned(&expr, lapis) {
                    if !quiet {
                        let info = g.display().replace('\n', "\n// ");
                        lapis.data.buffer.push_str(&format!("\n// {}", info));
                        lapis
                            .data
                            .buffer
                            .push_str(&format!("Size           : {}", g.size()));
                    }
                } else if let Some(id) = method_nodeid(&expr, lapis).or(path_nodeid(&expr, lapis)) {
                    if !quiet {
                        lapis.data.buffer.push_str(&format!("\n// {:?}", id));
                    }
                } else if let Some(b) = eval_bool(&expr, lapis) {
                    if !quiet {
                        lapis.data.buffer.push_str(&format!("\n// {:?}", b));
                    }
                } else if let Some(s) = eval_shared(&expr, lapis) {
                    if !quiet {
                        lapis
                            .data
                            .buffer
                            .push_str(&format!("\n// Shared({})", s.value()));
                    }
                } else if let Some(w) = path_wave(&expr, lapis) {
                    if !quiet {
                        let info = format!(
                            "\n// Wave(ch:{}, sr:{}, len:{}, dur:{})",
                            w.channels(),
                            w.sample_rate(),
                            w.len(),
                            w.duration()
                        );
                        lapis.data.buffer.push_str(&info);
                    }
                } else if let Some(w) = eval_wave(&expr, lapis) {
                    if !quiet {
                        lapis.data.buffer.push_str(&format!(
                            "\n// Wave(ch:{}, sr:{}, len:{}, dur:{})",
                            w.channels(),
                            w.sample_rate(),
                            w.len(),
                            w.duration()
                        ));
                    }
                } else if let Some(seq) = path_seq(&expr, lapis).or(call_seq(&expr, lapis).as_ref())
                {
                    if !quiet {
                        let info = format!(
                            "\n// Sequencer(outs: {}, has_backend: {}, replay: {})",
                            seq.outputs(),
                            seq.has_backend(),
                            seq.replay_events()
                        );
                        lapis.data.buffer.push_str(&info);
                    }
                } else if let Some(source) = eval_source(&expr, lapis) {
                    lapis.data.buffer.push_str(&format!("\n// {:?}", source));
                } else if let Some(event) =
                    method_eventid(&expr, lapis).or(path_eventid(&expr, lapis))
                {
                    if !quiet {
                        lapis.data.buffer.push_str(&format!("\n// {:?}", event));
                    }
                } else if let Some(entity) = eval_entity(&expr, lapis) {
                    if !quiet {
                        lapis.data.buffer.push_str(&format!("\n// {:?}", entity));
                    }
                } else if let Expr::Binary(expr) = expr {
                    float_bin_assign(&expr, lapis);
                } else if let Expr::MethodCall(expr) = expr {
                    match expr.method.to_string().as_str() {
                        "play" => {
                            if let Some(g) = eval_net(&expr.receiver, lapis) {
                                if g.inputs() == 0 && g.outputs() == 1 {
                                    lapis
                                        .data
                                        .slot
                                        .set(Fade::Smooth, 0.01, Box::new(g | dc(0.)));
                                } else if g.inputs() == 0 && g.outputs() == 2 {
                                    lapis.data.slot.set(Fade::Smooth, 0.01, Box::new(g));
                                } else {
                                    lapis.data.slot.set(
                                        Fade::Smooth,
                                        0.01,
                                        Box::new(dc(0.) | dc(0.)),
                                    );
                                }
                            }
                        }
                        "tick" => {
                            let Some(input) = expr.args.first() else {
                                return;
                            };
                            let Some(in_arr) = eval_vec(input, lapis) else {
                                return;
                            };
                            let mut output = Vec::new();
                            if let Some(k) = nth_path_ident(&expr.receiver, 0) {
                                if let Some(g) = &mut lapis.data.gmap.get_mut(&k) {
                                    if g.inputs() != in_arr.len() {
                                        return;
                                    }
                                    output.resize(g.outputs(), 0.);
                                    g.tick(&in_arr, &mut output);
                                }
                            } else if let Some(mut g) = eval_net(&expr.receiver, lapis) {
                                if g.inputs() != in_arr.len() {
                                    return;
                                }
                                output.resize(g.outputs(), 0.);
                                g.tick(&in_arr, &mut output);
                            }
                            if let Some(out) = expr.args.get(1) {
                                if let Some(k) = nth_path_ident(out, 0) {
                                    if let Some(var) = lapis.data.vmap.get_mut(&k) {
                                        *var = output;
                                    }
                                }
                            } else if !quiet {
                                lapis.data.buffer.push_str(&format!("\n// {:?}", output));
                            }
                        }
                        "play_backend" => {
                            if let Some(k) = nth_path_ident(&expr.receiver, 0) {
                                if let Some(g) = &mut lapis.data.gmap.get_mut(&k) {
                                    if !g.has_backend() {
                                        let g = g.backend();
                                        if g.inputs() == 0 && g.outputs() == 2 {
                                            lapis.data.slot.set(Fade::Smooth, 0.01, Box::new(g));
                                        }
                                    }
                                } else if let Some(seq) = &mut lapis.data.seqmap.get_mut(&k) {
                                    if !seq.has_backend() {
                                        let backend = seq.backend();
                                        if backend.outputs() == 2 {
                                            lapis.data.slot.set(
                                                Fade::Smooth,
                                                0.01,
                                                Box::new(backend),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        "drop" => {
                            if let Some(k) = nth_path_ident(&expr.receiver, 0) {
                                lapis.drop(&k);
                            }
                        }
                        "error" if !quiet => {
                            if let Some(k) = nth_path_ident(&expr.receiver, 0) {
                                if let Some(g) = &mut lapis.data.gmap.get_mut(&k) {
                                    let error = format!("\n// {:?}", g.error());
                                    lapis.data.buffer.push_str(&error);
                                }
                            }
                        }
                        _ => {
                            wave_methods(&expr, lapis);
                            net_methods(&expr, lapis);
                            vec_methods(&expr, lapis);
                            shared_methods(&expr, lapis);
                            seq_methods(&expr, lapis);
                        }
                    }
                }
            }
        },
        _ => {}
    }
}

fn default_out_device(slot: SlotBackend) {
    let host = cpal::default_host();
    if let Some(device) = host.default_output_device() {
        if let Ok(default_config) = device.default_output_config() {
            let mut config = default_config.config();
            config.channels = 2;
            match default_config.sample_format() {
                cpal::SampleFormat::F32 => run::<f32>(&device, &config, slot),
                cpal::SampleFormat::I16 => run::<i16>(&device, &config, slot),
                cpal::SampleFormat::U16 => run::<u16>(&device, &config, slot),
                format => eprintln!("unsupported sample format: {}", format),
            }
        }
    }
}

fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig, slot: SlotBackend)
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
            std::mem::forget(stream);
        }
    }
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

fn default_in_device(ls: Sender<f32>, rs: Sender<f32>) {
    let host = cpal::default_host();
    if let Some(device) = host.default_input_device() {
        if let Ok(config) = device.default_input_config() {
            match config.sample_format() {
                cpal::SampleFormat::F32 => run_in::<f32>(&device, &config.into(), ls, rs),
                cpal::SampleFormat::I16 => run_in::<i16>(&device, &config.into(), ls, rs),
                cpal::SampleFormat::U16 => run_in::<u16>(&device, &config.into(), ls, rs),
                format => eprintln!("unsupported sample format: {}", format),
            }
        }
    }
}

fn run_in<T>(device: &cpal::Device, config: &cpal::StreamConfig, ls: Sender<f32>, rs: Sender<f32>)
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
            std::mem::forget(stream);
        }
    }
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
