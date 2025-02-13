use crate::interaction::*;
use crate::joints::*;
use crate::lapis::*;

pub fn eval_entity(expr: &Expr, lapis: &mut Lapis) -> Option<Entity> {
    match expr {
        Expr::Call(expr) => call_entity(expr, lapis),
        Expr::Lit(expr) => lit_entity(&expr.lit),
        Expr::Path(expr) => path_entity(&expr.path, lapis),
        Expr::MethodCall(expr) => method_entity(expr, lapis),
        _ => None,
    }
}

pub fn path_lit_entity(expr: &Expr, lapis: &Lapis) -> Option<Entity> {
    match expr {
        Expr::Lit(expr) => lit_entity(&expr.lit),
        Expr::Path(expr) => path_entity(&expr.path, lapis),
        _ => None,
    }
}

fn lit_entity(expr: &Lit) -> Option<Entity> {
    match expr {
        Lit::Int(expr) => Entity::try_from_bits(expr.base10_parse::<u64>().ok()?).ok(),
        _ => None,
    }
}

fn path_entity(expr: &Path, lapis: &Lapis) -> Option<Entity> {
    let k = expr.segments.first()?.ident.to_string();
    if k == "Entity" && expr.segments.get(1)?.ident == "PLACEHOLDER" {
        return Some(Entity::PLACEHOLDER);
    }
    lapis.data.entitymap.get(&k).copied()
}

fn call_entity(expr: &ExprCall, lapis: &mut Lapis) -> Option<Entity> {
    let func = nth_path_ident(&expr.func, 0)?;
    match func.as_str() {
        "Entity" => {
            let f = nth_path_ident(&expr.func, 1)?;
            match f.as_str() {
                "from_bits" => {
                    if let Expr::Lit(expr) = expr.args.first()? {
                        return lit_entity(&expr.lit);
                    }
                    None
                }
                "from_floats" => {
                    let arr = eval_vec(expr.args.first()?, lapis)?;
                    let h = arr.first()?;
                    let l = arr.get(1)?;
                    let h = h.to_bits();
                    let l = l.to_bits();
                    let bits = ((h as u64) << u32::BITS) | (l as u64);
                    Entity::try_from_bits(bits).ok()
                }
                _ => None,
            }
        }
        "spawn" => {
            let r = eval_float(expr.args.first()?, lapis)?;
            let e = lapis.commands.spawn_empty().id();
            lapis.commands.trigger_targets(InsertDefaults(r), e);
            Some(e)
        }
        "joint" => {
            let e1 = eval_entity(expr.args.first()?, lapis);
            let e2 = eval_entity(expr.args.get(1)?, lapis);
            if let (Some(e1), Some(e2)) = (e1, e2) {
                let e = lapis.commands.spawn_empty().id();
                lapis.commands.trigger_targets(JointEntities(e1, e2), e);
                Some(e)
            } else {
                let x1 = eval_float(expr.args.first()?, lapis)?;
                let y1 = eval_float(expr.args.get(1)?, lapis)?;
                let x2 = eval_float(expr.args.get(2)?, lapis)?;
                let y2 = eval_float(expr.args.get(3)?, lapis)?;
                let e = lapis.commands.spawn_empty().id();
                let i = Vec2::new(x1, y1);
                let f = Vec2::new(x2, y2);
                lapis.commands.trigger_targets(JointPoints(i, f), e);
                Some(e)
            }
        }
        _ => None,
    }
}

fn method_entity(expr: &ExprMethodCall, lapis: &mut Lapis) -> Option<Entity> {
    let e = eval_entity(&expr.receiver, lapis)?;
    // this being here allows some nonsense like
    // let var = entity.despawn();
    // which doesn't assign anything to var but does despawn entity
    if expr.method == "despawn" {
        lapis.commands.get_entity(e)?.try_despawn();
        return None;
    } else if expr.method == "disjoint" {
        lapis.commands.trigger_targets(Disjoint, e);
        return None;
    }
    let val = eval_float(expr.args.first()?, lapis);
    let cmd = &mut lapis.commands;
    match expr.method.to_string().as_str() {
        "x" => cmd.trigger_targets(Property::X(val?), e),
        "y" => cmd.trigger_targets(Property::Y(val?), e),
        "z" => cmd.trigger_targets(Property::Z(val?), e),
        "rx" => cmd.trigger_targets(Property::Rx(val?), e),
        "ry" => cmd.trigger_targets(Property::Ry(val?), e),
        "rot" => cmd.trigger_targets(Property::Rot(val?), e),
        "mass" => cmd.trigger_targets(Property::Mass(val?), e),
        "vx" => cmd.trigger_targets(Property::Vx(val?), e),
        "vy" => cmd.trigger_targets(Property::Vy(val?), e),
        "va" => cmd.trigger_targets(Property::Va(val?), e),
        "restitution" => cmd.trigger_targets(Property::Restitution(val?), e),
        "lindamp" => cmd.trigger_targets(Property::LinDamp(val?), e),
        "angdamp" => cmd.trigger_targets(Property::AngDamp(val?), e),
        "inertia" => cmd.trigger_targets(Property::Inertia(val?), e),
        "h" => cmd.trigger_targets(Property::H(val?), e),
        "s" => cmd.trigger_targets(Property::S(val?), e),
        "l" => cmd.trigger_targets(Property::L(val?), e),
        "a" => cmd.trigger_targets(Property::A(val?), e),
        "sides" => cmd.trigger_targets(Property::Sides(val? as u32), e),
        "cmx" => cmd.trigger_targets(Property::Cmx(val?), e),
        "cmy" => cmd.trigger_targets(Property::Cmy(val?), e),
        "friction" => cmd.trigger_targets(Property::Friction(val?), e),
        "tail" => cmd.trigger_targets(Property::Tail(val? as usize), e),
        "layer" => cmd.trigger_targets(Property::Layer(val? as u32), e),
        "dynamic" => {
            let b = eval_bool(expr.args.first()?, lapis)?;
            lapis.commands.trigger_targets(Property::Dynamic(b), e);
        }
        "sensor" => {
            let b = eval_bool(expr.args.first()?, lapis)?;
            lapis.commands.trigger_targets(Property::Sensor(b), e);
        }
        "links" => {
            if let Expr::Lit(expr) = expr.args.first()? {
                if let Lit::Str(expr) = &expr.lit {
                    cmd.trigger_targets(Property::Links(expr.value()), e);
                }
            }
        }
        "code_i" => {
            if let Expr::Lit(expr) = expr.args.first()? {
                if let Lit::Str(expr) = &expr.lit {
                    cmd.trigger_targets(Property::CodeI(expr.value()), e);
                }
            }
        }
        "code_f" => {
            if let Expr::Lit(expr) = expr.args.first()? {
                if let Lit::Str(expr) = &expr.lit {
                    cmd.trigger_targets(Property::CodeF(expr.value()), e);
                }
            }
        }
        // joint methods
        "joint_type" => match val?.trunc() {
            0. => cmd.trigger_targets(ReplaceJoint(JointType::Fixed), e),
            1. => cmd.trigger_targets(ReplaceJoint(JointType::Distance), e),
            2. => cmd.trigger_targets(ReplaceJoint(JointType::Prismatic), e),
            3. => cmd.trigger_targets(ReplaceJoint(JointType::Revolute), e),
            _ => {}
        },
        "compliance" => cmd.trigger_targets(JointProperty::Compliance(val?), e),
        "anchor1" => {
            let val2 = eval_float(expr.args.get(1)?, lapis)?;
            lapis
                .commands
                .trigger_targets(JointProperty::Anchor1(val?, val2), e);
        }
        "anchor2" => {
            let val2 = eval_float(expr.args.get(1)?, lapis)?;
            lapis
                .commands
                .trigger_targets(JointProperty::Anchor2(val?, val2), e);
        }
        "limits" => {
            let val2 = eval_float(expr.args.get(1)?, lapis)?;
            lapis
                .commands
                .trigger_targets(JointProperty::Limits(val?, val2), e);
        }
        "rest" => cmd.trigger_targets(JointProperty::Rest(val?), e),
        "free_axis" => {
            let val2 = eval_float(expr.args.get(1)?, lapis)?;
            lapis
                .commands
                .trigger_targets(JointProperty::FreeAxis(val?, val2), e);
        }
        _ => return None,
    }
    Some(e)
}
