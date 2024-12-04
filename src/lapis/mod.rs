use bevy::prelude::*;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, SizedSample,
};
use crossbeam_channel::{bounded, Receiver, Sender};
use fundsp::hacker32::*;
use std::{collections::HashMap, sync::Arc};
use syn::*;

mod arrays;
mod atomics;
mod bools;
mod floats;
mod helpers;
mod ints;
mod nets;
mod sequencers;
mod units;
mod waves;
use {
    arrays::*, atomics::*, bools::*, floats::*, helpers::*, ints::*, nets::*, sequencers::*,
    waves::*,
};

#[derive(Resource)]
pub struct Lapis {
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
    pub slot: Slot,
    pub receivers: (Receiver<f32>, Receiver<f32>),
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
pub struct UpdateCode(pub String);

impl Lapis {
    pub fn new() -> Self {
        let (slot, slot_back) = Slot::new(Box::new(dc(0.) | dc(0.)));
        let (ls, lr) = bounded(4096);
        let (rs, rr) = bounded(4096);
        default_out_device(slot_back);
        default_in_device(ls, rs);
        Lapis {
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
            slot,
            receivers: (lr, rr),
        }
    }
    pub fn eval(&mut self, input: &str) {
        if !input.is_empty() {
            self.buffer.push('\n');
            match parse_str::<Stmt>(input) {
                Ok(stmt) => {
                    self.buffer.push_str(input);
                    eval_stmt(stmt, self, false);
                }
                Err(err) => {
                    self.buffer.push_str(&format!("\n// error: {}", err));
                }
            }
        }
    }
    pub fn quiet_eval(&mut self, input: &str) {
        if let Ok(stmt) = parse_str::<Stmt>(input) {
            eval_stmt(stmt, self, true);
        }
    }
}

// TODO clean this up
fn eval_stmt(s: Stmt, lapis: &mut Lapis, quiet: bool) {
    match s {
        Stmt::Local(expr) => {
            if let Some(k) = pat_ident(&expr.pat) {
                if let Some(expr) = expr.init {
                    if let Some(v) = eval_float(&expr.expr, lapis) {
                        remove_from_all_maps(&k, lapis);
                        lapis.fmap.insert(k, v);
                    } else if let Some(v) = eval_net(&expr.expr, lapis) {
                        remove_from_all_maps(&k, lapis);
                        lapis.gmap.insert(k, v);
                    } else if let Some(arr) = eval_vec(&expr.expr, lapis) {
                        remove_from_all_maps(&k, lapis);
                        lapis.vmap.insert(k, arr);
                    } else if let Some(id) =
                        method_nodeid(&expr.expr, lapis).or(path_nodeid(&expr.expr, lapis))
                    {
                        remove_from_all_maps(&k, lapis);
                        lapis.idmap.insert(k, id);
                    } else if let Some(b) = eval_bool(&expr.expr, lapis) {
                        remove_from_all_maps(&k, lapis);
                        lapis.bmap.insert(k, b);
                    } else if let Some(s) = eval_shared(&expr.expr, lapis) {
                        remove_from_all_maps(&k, lapis);
                        lapis.smap.insert(k, s);
                    } else if let Some(w) = eval_wave(&expr.expr, lapis) {
                        remove_from_all_maps(&k, lapis);
                        let wave = Arc::new(w);
                        lapis.wmap.insert(k, wave);
                    } else if let Some(seq) = call_seq(&expr.expr, lapis) {
                        remove_from_all_maps(&k, lapis);
                        lapis.seqmap.insert(k, seq);
                    } else if let Some(event) =
                        method_eventid(&expr.expr, lapis).or(path_eventid(&expr.expr, lapis))
                    {
                        remove_from_all_maps(&k, lapis);
                        lapis.eventmap.insert(k, event);
                    }
                }
            }
        }
        Stmt::Expr(expr, _) => match expr {
            Expr::MethodCall(ref method) => match method.method.to_string().as_str() {
                "play" => {
                    if let Some(g) = eval_net(&method.receiver, lapis) {
                        if g.inputs() == 0 && g.outputs() == 1 {
                            lapis.slot.set(Fade::Smooth, 0.01, Box::new(g | dc(0.)));
                        } else if g.inputs() == 0 && g.outputs() == 2 {
                            lapis.slot.set(Fade::Smooth, 0.01, Box::new(g));
                        } else {
                            lapis
                                .slot
                                .set(Fade::Smooth, 0.01, Box::new(dc(0.) | dc(0.)));
                        }
                    }
                }
                "tick" => {
                    let Some(input) = method.args.first() else {
                        return;
                    };
                    let Some(in_arr) = eval_vec_cloned(input, lapis) else {
                        return;
                    };
                    let mut output = Vec::new();
                    if let Some(k) = nth_path_ident(&method.receiver, 0) {
                        if let Some(g) = &mut lapis.gmap.get_mut(&k) {
                            if g.inputs() != in_arr.len() {
                                return;
                            }
                            output.resize(g.outputs(), 0.);
                            g.tick(&in_arr, &mut output);
                        }
                    } else if let Some(mut g) = eval_net(&method.receiver, lapis) {
                        if g.inputs() != in_arr.len() {
                            return;
                        }
                        output.resize(g.outputs(), 0.);
                        g.tick(&in_arr, &mut output);
                    }
                    if let Some(out) = method.args.get(1) {
                        if let Some(k) = nth_path_ident(out, 0) {
                            if let Some(var) = lapis.vmap.get_mut(&k) {
                                *var = output;
                            }
                        }
                    } else if !quiet {
                        lapis.buffer.push_str(&format!("\n// {:?}", output));
                    }
                }
                "play_backend" => {
                    if let Some(k) = nth_path_ident(&method.receiver, 0) {
                        if let Some(g) = &mut lapis.gmap.get_mut(&k) {
                            if !g.has_backend() {
                                let g = g.backend();
                                if g.inputs() == 0 && g.outputs() == 2 {
                                    lapis.slot.set(Fade::Smooth, 0.01, Box::new(g));
                                }
                            }
                        } else if let Some(seq) = &mut lapis.seqmap.get_mut(&k) {
                            if !seq.has_backend() {
                                let backend = seq.backend();
                                if backend.outputs() == 2 {
                                    lapis.slot.set(Fade::Smooth, 0.01, Box::new(backend));
                                }
                            }
                        }
                    }
                }
                "drop" => {
                    if let Some(k) = nth_path_ident(&method.receiver, 0) {
                        remove_from_all_maps(&k, lapis);
                    }
                }
                "error" => {
                    if quiet {
                        return;
                    }
                    if let Some(k) = nth_path_ident(&method.receiver, 0) {
                        if let Some(g) = &mut lapis.gmap.get_mut(&k) {
                            lapis.buffer.push_str(&format!("\n// {:?}", g.error()));
                        }
                    }
                }
                "source" => {
                    if quiet {
                        return;
                    }
                    if let Some(k) = nth_path_ident(&method.receiver, 0) {
                        if let Some(g) = &mut lapis.gmap.get(&k) {
                            let arg0 = method.args.first();
                            let arg1 = method.args.get(1);
                            if let (Some(arg0), Some(arg1)) = (arg0, arg1) {
                                let id = path_nodeid(arg0, lapis);
                                let chan = eval_usize(arg1, lapis);
                                if let (Some(id), Some(chan)) = (id, chan) {
                                    if g.contains(id) && chan < g.inputs_in(id) {
                                        lapis
                                            .buffer
                                            .push_str(&format!("\n// {:?}", g.source(id, chan)));
                                    }
                                }
                            }
                        }
                    }
                }
                "output_source" => {
                    if quiet {
                        return;
                    }
                    if let Some(k) = nth_path_ident(&method.receiver, 0) {
                        if let Some(g) = &mut lapis.gmap.get(&k) {
                            let arg0 = method.args.first();
                            if let Some(arg0) = arg0 {
                                let chan = eval_usize(arg0, lapis);
                                if let Some(chan) = chan {
                                    lapis
                                        .buffer
                                        .push_str(&format!("\n// {:?}", g.output_source(chan)));
                                }
                            }
                        }
                    }
                }
                _ => {
                    if !quiet {
                        if let Some(n) = method_call_float(method, lapis) {
                            lapis.buffer.push_str(&format!("\n// {:?}", n));
                            return;
                        } else if let Some(arr) = method_call_vec_ref(method, lapis) {
                            lapis.buffer.push_str(&format!("\n// {:?}", arr));
                            return;
                        } else if let Some(nodeid) = method_nodeid(&expr, lapis) {
                            lapis.buffer.push_str(&format!("\n// {:?}", nodeid));
                            return;
                        } else if let Some(event) = method_eventid(&expr, lapis) {
                            lapis.buffer.push_str(&format!("\n// {:?}", event));
                            return;
                        } else if let Some(mut g) = method_net(method, lapis) {
                            let info = g.display().replace('\n', "\n// ");
                            lapis.buffer.push_str(&format!("\n// {}", info));
                            lapis
                                .buffer
                                .push_str(&format!("Size           : {}", g.size()));
                            return;
                        }
                    }
                    wave_methods(method, lapis);
                    net_methods(method, lapis);
                    vec_methods(method, lapis);
                    shared_methods(method, lapis);
                    seq_methods(method, lapis);
                }
            },
            Expr::Assign(expr) => match *expr.left {
                Expr::Path(_) => {
                    let Some(ident) = nth_path_ident(&expr.left, 0) else {
                        return;
                    };
                    if let Some(f) = eval_float(&expr.right, lapis) {
                        if let Some(var) = lapis.fmap.get_mut(&ident) {
                            *var = f;
                        }
                    } else if lapis.gmap.contains_key(&ident) {
                        if let Some(g) = eval_net(&expr.right, lapis) {
                            lapis.gmap.insert(ident, g);
                        }
                    } else if lapis.vmap.contains_key(&ident) {
                        if let Some(a) = eval_vec(&expr.right, lapis) {
                            lapis.vmap.insert(ident, a);
                        }
                    } else if let Some(id) =
                        method_nodeid(&expr.right, lapis).or(path_nodeid(&expr.right, lapis))
                    {
                        if let Some(var) = lapis.idmap.get_mut(&ident) {
                            *var = id;
                        }
                    } else if let Some(b) = eval_bool(&expr.right, lapis) {
                        if let Some(var) = lapis.bmap.get_mut(&ident) {
                            *var = b;
                        }
                    } else if let Some(s) = eval_shared(&expr.right, lapis) {
                        if let Some(var) = lapis.smap.get_mut(&ident) {
                            *var = s;
                        }
                    } else if let Some(event) =
                        method_eventid(&expr.right, lapis).or(path_eventid(&expr.right, lapis))
                    {
                        if let Some(var) = lapis.eventmap.get_mut(&ident) {
                            *var = event;
                        }
                    }
                }
                Expr::Index(left) => {
                    if let Some(k) = nth_path_ident(&left.expr, 0) {
                        if let Some(index) = eval_usize(&left.index, lapis) {
                            if let Some(right) = eval_float(&expr.right, lapis) {
                                if let Some(vec) = lapis.vmap.get_mut(&k) {
                                    if let Some(v) = vec.get_mut(index) {
                                        *v = right;
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
                let tmp = lapis.fmap.remove(&ident);
                if let Some((r0, r1)) = bounds {
                    for i in r0..r1 {
                        lapis.fmap.insert(ident.clone(), i as f32);
                        for stmt in &expr.body.stmts {
                            eval_stmt(stmt.clone(), lapis, quiet);
                        }
                    }
                } else if let Some(arr) = arr {
                    for i in arr {
                        lapis.fmap.insert(ident.clone(), i);
                        for stmt in &expr.body.stmts {
                            eval_stmt(stmt.clone(), lapis, quiet);
                        }
                    }
                }
                if let Some(old) = tmp {
                    lapis.fmap.insert(ident, old);
                } else {
                    lapis.fmap.remove(&ident);
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
                if quiet {
                    return;
                }
                if let Some(n) = eval_float(&expr, lapis) {
                    lapis.buffer.push_str(&format!("\n// {:?}", n));
                } else if let Some(arr) = eval_vec_ref(&expr, lapis) {
                    lapis.buffer.push_str(&format!("\n// {:?}", arr));
                } else if let Some(arr) = eval_vec_cloned(&expr, lapis) {
                    lapis.buffer.push_str(&format!("\n// {:?}", arr));
                } else if let Some(mut g) = eval_net_cloned(&expr, lapis) {
                    let info = g.display().replace('\n', "\n// ");
                    lapis.buffer.push_str(&format!("\n// {}", info));
                    lapis
                        .buffer
                        .push_str(&format!("Size           : {}", g.size()));
                } else if let Some(id) = path_nodeid(&expr, lapis) {
                    lapis.buffer.push_str(&format!("\n// {:?}", id));
                } else if let Some(b) = eval_bool(&expr, lapis) {
                    lapis.buffer.push_str(&format!("\n// {:?}", b));
                } else if let Some(s) = eval_shared(&expr, lapis) {
                    lapis
                        .buffer
                        .push_str(&format!("\n// Shared({})", s.value()));
                } else if let Some(w) = path_wave(&expr, lapis) {
                    lapis.buffer.push_str(&format!(
                        "\n// Wave(ch:{}, sr:{}, len:{}, dur:{})",
                        w.channels(),
                        w.sample_rate(),
                        w.len(),
                        w.duration()
                    ));
                } else if let Some(w) = eval_wave(&expr, lapis) {
                    lapis.buffer.push_str(&format!(
                        "\n// Wave(ch:{}, sr:{}, len:{}, dur:{})",
                        w.channels(),
                        w.sample_rate(),
                        w.len(),
                        w.duration()
                    ));
                } else if let Some(seq) = path_seq(&expr, lapis).or(call_seq(&expr, lapis).as_ref())
                {
                    let info = format!(
                        "\n// Sequencer(outs: {}, has_backend: {}, replay: {})",
                        seq.outputs(),
                        seq.has_backend(),
                        seq.replay_events()
                    );
                    lapis.buffer.push_str(&info);
                } else if let Some(event) = path_eventid(&expr, lapis) {
                    lapis.buffer.push_str(&format!("\n// {:?}", event));
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
