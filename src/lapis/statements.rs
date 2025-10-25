use super::{
    arrays::*, atomics::*, bools::*, entities::*, floats::*, helpers::*, ints::*, nets::*,
    sequencers::*, sources::*, waves::*, Lapis,
};
use crate::audio::*;
use crate::objects::*;
use avian2d::prelude::*;
use bevy::prelude::*;
use crossbeam_channel::bounded;
use fundsp::hacker32::*;
use std::{sync::Arc, thread, time::Duration};
use syn::*;

pub fn eval_stmt(s: Stmt, lapis: &mut Lapis) -> String {
    let mut buffer = String::new();
    match s {
        Stmt::Local(expr) => {
            eval_local(&expr, lapis);
        }
        Stmt::Expr(expr, _) => match expr {
            Expr::Assign(expr) => eval_assign(&expr, lapis),
            Expr::ForLoop(expr) => eval_for_loop(&expr, lapis, &mut buffer),
            Expr::Block(expr) => eval_block(expr, lapis, &mut buffer),
            Expr::If(expr) => eval_if(expr, lapis, &mut buffer),
            expr => eval_expr(expr, lapis, &mut buffer),
        },
        _ => {}
    }
    buffer
}

fn eval_expr(expr: Expr, lapis: &mut Lapis, buffer: &mut String) {
    if let Some(n) = eval_float(&expr, lapis) {
        buffer.push_str(&format!("\n// {n:?}"));
    } else if let Some(arr) = eval_vec(&expr, lapis) {
        buffer.push_str(&format!("\n// {arr:?}"));
    } else if let Some(mut g) = eval_net_cloned(&expr, lapis) {
        let info = g.display().replace('\n', "\n// ");
        buffer.push_str(&format!("\n// {info}"));
        buffer.push_str(&format!("Size           : {}", g.size()));
    } else if let Some(id) = eval_nodeid(&expr, lapis) {
        buffer.push_str(&format!("\n// {id:?}"));
    } else if let Some(b) = eval_bool(&expr, lapis) {
        buffer.push_str(&format!("\n// {b:?}"));
    } else if let Some(s) = eval_shared(&expr, lapis) {
        buffer.push_str(&format!("\n// Shared({})", s.value()));
    } else if let Some(w) = path_wave(&expr, lapis) {
        let info = format!(
            "\n// Wave(ch:{}, sr:{}, len:{}, dur:{})",
            w.channels(),
            w.sample_rate(),
            w.len(),
            w.duration()
        );
        buffer.push_str(&info);
    } else if let Some(w) = eval_wave(&expr, lapis) {
        buffer.push_str(&format!(
            "\n// Wave(ch:{}, sr:{}, len:{}, dur:{})",
            w.channels(),
            w.sample_rate(),
            w.len(),
            w.duration()
        ));
    } else if let Some(seq) = path_seq(&expr, lapis).or(call_seq(&expr, lapis).as_ref()) {
        let info = format!(
            "\n// Sequencer(outs: {}, ins: {}, has_backend: {}, replay: {}, loop: ({}, {}))",
            seq.outputs(),
            seq.inputs(),
            seq.has_backend(),
            seq.replay_events(),
            seq.loop_start(),
            seq.loop_end(),
        );
        buffer.push_str(&info);
    } else if let Some(source) = eval_source(&expr, lapis) {
        buffer.push_str(&format!("\n// {source:?}"));
    } else if let Some(event) = eval_eventid(&expr, lapis) {
        buffer.push_str(&format!("\n// {event:?}"));
    } else if let Some(entity) = eval_entity(&expr, lapis) {
        buffer.push_str(&format!("\n// {entity:?}"));
    } else if let Expr::Binary(expr) = expr {
        float_bin_assign(&expr, lapis);
    } else if let Expr::Call(expr) = expr {
        gravity_commands(&expr, lapis);
        function_calls(&expr, lapis, buffer);
    } else if let Expr::Break(_) = expr {
        buffer.push_str("#B");
    } else if let Expr::Continue(_) = expr {
        buffer.push_str("#C");
    } else if let Expr::MethodCall(expr) = expr {
        match expr.method.to_string().as_str() {
            "play" => {
                if let Some(mut g) = eval_net(&expr.receiver, lapis) {
                    let slot_outputs = lapis.audio_out.outputs();
                    if g.inputs() == 0 && g.outputs() == slot_outputs {
                        if let Some(config) = &lapis.out_stream_config.0 {
                            g.allocate();
                            g.set_sample_rate(config.sample_rate.0 as f64);
                            lapis.audio_out.set(Fade::Smooth, 0.01, Box::new(g));
                        }
                    }
                }
            }
            "drop" => {
                if let Some(k) = nth_path_ident(&expr.receiver, 0) {
                    lapis.drop(&k);
                }
            }
            "error" => {
                if let Some(k) = nth_path_ident(&expr.receiver, 0) {
                    if let Some(g) = lapis.data.gmap.get_mut(&k) {
                        let error = format!("\n// {:?}", g.error());
                        buffer.push_str(&error);
                    }
                }
            }
            "pause" => {
                if let Some(k) = nth_path_ident(&expr.receiver, 0) {
                    if k == "time" {
                        lapis.time.pause();
                    }
                }
            }
            "resume" | "unpause" => {
                if let Some(k) = nth_path_ident(&expr.receiver, 0) {
                    if k == "time" {
                        lapis.time.unpause();
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

fn eval_if(expr: ExprIf, lapis: &mut Lapis, buffer: &mut String) {
    if let Some(cond) = eval_bool(&expr.cond, lapis) {
        if cond {
            let expr = Expr::Block(ExprBlock {
                attrs: Vec::new(),
                label: None,
                block: expr.then_branch,
            });
            let s = eval_stmt(Stmt::Expr(expr, None), lapis);
            buffer.push_str(&s);
        } else if let Some((_, else_branch)) = expr.else_branch {
            let s = eval_stmt(Stmt::Expr(*else_branch, None), lapis);
            buffer.push_str(&s);
        }
    }
}

fn eval_block(expr: ExprBlock, lapis: &mut Lapis, buffer: &mut String) {
    for stmt in expr.block.stmts {
        buffer.push_str(&eval_stmt(stmt, lapis));
    }
}

fn eval_local(expr: &syn::Local, lapis: &mut Lapis) -> Option<()> {
    if let Some(k) = pat_ident(&expr.pat) {
        if let Some(expr) = &expr.init {
            if let Some(v) = eval_float(&expr.expr, lapis) {
                lapis.drop(&k);
                lapis.data.fmap.insert(k, v);
            } else if let Some(v) = eval_net(&expr.expr, lapis) {
                lapis.drop(&k);
                lapis.data.gmap.insert(k, v);
            } else if let Some(arr) = eval_vec(&expr.expr, lapis) {
                lapis.drop(&k);
                lapis.data.vmap.insert(k, arr);
            } else if let Some(table) = eval_atomic_table(&expr.expr, lapis) {
                lapis.drop(&k);
                lapis.data.atomic_table_map.insert(k, Arc::new(table));
            } else if let Some(id) = eval_nodeid(&expr.expr, lapis) {
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
            } else if let Some(event) = eval_eventid(&expr.expr, lapis) {
                lapis.drop(&k);
                lapis.data.eventmap.insert(k, event);
            } else if let Some(entity) = eval_entity(&expr.expr, lapis) {
                lapis.drop(&k);
                lapis.data.entitymap.insert(k, entity);
            }
        }
    } else if let Pat::Tuple(pat) = &expr.pat {
        if let Some(init) = &expr.init {
            if let Expr::Call(call) = &*init.expr {
                let f = nth_path_ident(&call.func, 0)?;
                if f == "bounded" {
                    let p0 = pat_ident(pat.elems.first()?)?;
                    let p1 = pat_ident(pat.elems.get(1)?)?;
                    let cap = eval_usize(call.args.first()?, lapis)?;
                    let (s, r) = bounded(cap.clamp(0, 1000000));
                    let s = Net::wrap(Box::new(An(BuffIn::new(s))));
                    let r = Net::wrap(Box::new(An(BuffOut::new(r))));
                    lapis.drop(&p0);
                    lapis.data.gmap.insert(p0, s);
                    lapis.drop(&p1);
                    lapis.data.gmap.insert(p1, r);
                } else if f == "buffer" {
                    let p0 = pat_ident(pat.elems.first()?)?;
                    let p1 = pat_ident(pat.elems.get(1)?)?;
                    let cap = eval_usize(call.args.first()?, lapis)?;
                    let (s, r) = fundsp::misc_nodes::buffer(cap.clamp(0, 1000000));
                    let s = Net::wrap(Box::new(s));
                    let r = Net::wrap(Box::new(r));
                    lapis.drop(&p0);
                    lapis.data.gmap.insert(p0, s);
                    lapis.drop(&p1);
                    lapis.data.gmap.insert(p1, r);
                } else if f == "Net" {
                    let f = nth_path_ident(&call.func, 1)?;
                    if f == "wrap_id" {
                        let p0 = pat_ident(pat.elems.first()?)?;
                        let p1 = pat_ident(pat.elems.get(1)?)?;
                        let initial = eval_net(call.args.first()?, lapis)?;
                        let (net, id) = Net::wrap_id(Box::new(initial));
                        lapis.drop(&p0);
                        lapis.data.gmap.insert(p0, net);
                        lapis.drop(&p1);
                        lapis.data.idmap.insert(p1, id);
                    }
                }
            }
        }
    }
    None
}

#[allow(clippy::map_entry)]
fn eval_assign(expr: &ExprAssign, lapis: &mut Lapis) {
    match &*expr.left {
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
            } else if let Some(id) = eval_nodeid(&expr.right, lapis) {
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
            } else if let Some(event) = eval_eventid(&expr.right, lapis) {
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
            if let Lit::Str(left) = &left.lit {
                if let Some(b) = eval_bool(&expr.right, lapis) {
                    match left.value().as_str() {
                        "keys" => lapis.data.keys_active = b,
                        "quiet" => lapis.data.quiet = b,
                        _ => {}
                    }
                } else if let Expr::Lit(right) = &*expr.right {
                    if let Some(shortcut) = parse_shortcut(left.value()) {
                        lapis.data.keys.remove(&shortcut);
                        if let Lit::Str(right) = &right.lit {
                            let key = shortcut.1.name();
                            let code = right.value().replace("$key", key);
                            if !code.is_empty() {
                                lapis.data.keys.insert(shortcut, code);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn eval_for_loop(expr: &ExprForLoop, lapis: &mut Lapis, buffer: &mut String) {
    if let Some(ident) = pat_ident(&expr.pat) {
        let bounds = range_bounds(&expr.expr, lapis);
        let arr = eval_vec(&expr.expr, lapis);
        let tmp = lapis.data.fmap.remove(&ident);
        if let Some((r0, r1)) = bounds {
            'main_loop: for i in r0..r1 {
                lapis.data.fmap.insert(ident.clone(), i as f32);
                for stmt in &expr.body.stmts {
                    let s = eval_stmt(stmt.clone(), lapis);
                    buffer.push_str(&s);
                    // NOTE amy.. you've out lazied yourself (proud of you)
                    if buffer.ends_with("#B") {
                        buffer.pop();
                        buffer.pop();
                        break 'main_loop;
                    } else if buffer.ends_with("#C") {
                        buffer.pop();
                        buffer.pop();
                        continue 'main_loop;
                    }
                }
            }
        } else if let Some(arr) = arr {
            'main_loop: for i in arr {
                lapis.data.fmap.insert(ident.clone(), i);
                for stmt in &expr.body.stmts {
                    let s = eval_stmt(stmt.clone(), lapis);
                    buffer.push_str(&s);
                    if buffer.ends_with("#B") {
                        buffer.pop();
                        buffer.pop();
                        break 'main_loop;
                    } else if buffer.ends_with("#C") {
                        buffer.pop();
                        buffer.pop();
                        continue 'main_loop;
                    }
                }
            }
        }
        if let Some(old) = tmp {
            lapis.data.fmap.insert(ident, old);
        } else {
            lapis.data.fmap.remove(&ident);
        }
    }
}

// TODO move this somewhere?
fn gravity_commands(expr: &ExprCall, lapis: &mut Lapis) -> Option<()> {
    let func = nth_path_ident(&expr.func, 0)?;
    if func == "gravity" {
        let x = eval_float(expr.args.first()?, lapis)?;
        let y = eval_float(expr.args.get(1)?, lapis)?;
        lapis.commands.insert_resource(Gravity(Vec2::new(x, y)));
    } else if func == "attraction" {
        let a = eval_float(expr.args.first()?, lapis)?;
        lapis.commands.insert_resource(AttractionFactor(a));
    }
    None
}

fn function_calls(expr: &ExprCall, lapis: &mut Lapis, buffer: &mut String) -> Option<()> {
    let func = nth_path_ident(&expr.func, 0)?;
    match func.as_str() {
        "list_in_devices" => {
            let list = list_in_devices().trim_end().replace('\n', "\n//");
            buffer.push_str(&format!("\n//{list}"));
        }
        "list_out_devices" => {
            let list = list_out_devices().trim_end().replace('\n', "\n//");
            buffer.push_str(&format!("\n//{list}"));
        }
        "set_in_device" => {
            // underscores will evaluate to None
            let host = eval_usize(expr.args.first()?, lapis);
            let device = eval_usize(expr.args.get(1)?, lapis);
            let channels = eval_usize(expr.args.get(2)?, lapis).map(|x| x as u16);
            let sr = eval_usize(expr.args.get(3)?, lapis).map(|x| x as u32);
            let buffer = eval_usize(expr.args.get(4)?, lapis).map(|x| x as u32);
            lapis.commands.trigger(SetInDevice {
                host,
                device,
                channels,
                sr,
                buffer,
            });
        }
        "set_out_device" => {
            let host = eval_usize(expr.args.first()?, lapis);
            let device = eval_usize(expr.args.get(1)?, lapis);
            let channels = eval_usize(expr.args.get(2)?, lapis).map(|x| x as u16);
            let sr = eval_usize(expr.args.get(3)?, lapis).map(|x| x as u32);
            let buffer = eval_usize(expr.args.get(4)?, lapis).map(|x| x as u32);
            lapis.commands.trigger(SetOutDevice {
                host,
                device,
                channels,
                sr,
                buffer,
            });
        }
        "drop_in_stream" => lapis.commands.trigger(DropInStream),
        "drop_out_stream" => lapis.commands.trigger(DropOutStream),
        "sleep" => {
            let d = eval_float(expr.args.first()?, lapis)?;
            let d = Duration::try_from_secs_f32(d).ok()?;
            thread::sleep(d);
        }
        "panic" => panic!(),
        _ => {}
    }
    None
}
