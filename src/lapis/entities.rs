use crate::lapis::*;

//TODO joints not spawned programmatically are not accessible
//TODO do we need Vec<Entity>? can we spawn structures like grids?
//      we can avoid this need by making join take coordinates instead and
//      so it becomes a copy of the spawn_joint system

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
            let e1 = eval_entity(expr.args.first()?, lapis)?;
            let e2 = eval_entity(expr.args.get(1)?, lapis)?;
            let joint_type = nth_path_ident(expr.args.get(2)?, 0)?;
            match joint_type.as_str() {
                "fixed" => Some(lapis.commands.spawn(FixedJoint::new(e1, e2)).id()),
                "distance" => Some(lapis.commands.spawn(DistanceJoint::new(e1, e2)).id()),
                "prismatic" => Some(lapis.commands.spawn(PrismaticJoint::new(e1, e2)).id()),
                "revolute" => Some(lapis.commands.spawn(RevoluteJoint::new(e1, e2)).id()),
                _ => None,
            }
            // TODO: trigger to read the joint settings resource like spawn does
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
    }
    let val = eval_float(expr.args.first()?, lapis);
    match expr.method.to_string().as_str() {
        "x" => lapis.commands.trigger_targets(Property::X(val?), e),
        "y" => lapis.commands.trigger_targets(Property::Y(val?), e),
        "rx" => lapis.commands.trigger_targets(Property::Rx(val?), e),
        "ry" => lapis.commands.trigger_targets(Property::Ry(val?), e),
        "rot" => lapis.commands.trigger_targets(Property::Rot(val?), e),
        "mass" => lapis.commands.trigger_targets(Property::Mass(val?), e),
        "vx" => lapis.commands.trigger_targets(Property::Vx(val?), e),
        "vy" => lapis.commands.trigger_targets(Property::Vy(val?), e),
        "va" => lapis.commands.trigger_targets(Property::Va(val?), e),
        "restitution" => lapis
            .commands
            .trigger_targets(Property::Restitution(val?), e),
        "lindamp" => lapis.commands.trigger_targets(Property::LinDamp(val?), e),
        "angdamp" => lapis.commands.trigger_targets(Property::AngDamp(val?), e),
        "inertia" => lapis.commands.trigger_targets(Property::Inertia(val?), e),
        "h" => lapis.commands.trigger_targets(Property::H(val?), e),
        "s" => lapis.commands.trigger_targets(Property::S(val?), e),
        "l" => lapis.commands.trigger_targets(Property::L(val?), e),
        "a" => lapis.commands.trigger_targets(Property::A(val?), e),
        "sides" => lapis
            .commands
            .trigger_targets(Property::Sides(val? as u32), e),
        "cmx" => lapis.commands.trigger_targets(Property::Cmx(val?), e),
        "cmy" => lapis.commands.trigger_targets(Property::Cmy(val?), e),
        "friction" => lapis.commands.trigger_targets(Property::Friction(val?), e),
        "tail" => lapis
            .commands
            .trigger_targets(Property::Tail(val? as usize), e),
        "layer" => lapis
            .commands
            .trigger_targets(Property::Layer(val? as u32), e),
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
                    lapis
                        .commands
                        .trigger_targets(Property::Links(expr.value()), e);
                }
            }
        }
        "code_i" => {
            if let Expr::Lit(expr) = expr.args.first()? {
                if let Lit::Str(expr) = &expr.lit {
                    lapis
                        .commands
                        .trigger_targets(Property::CodeI(expr.value()), e);
                }
            }
        }
        "code_f" => {
            if let Expr::Lit(expr) = expr.args.first()? {
                if let Lit::Str(expr) = &expr.lit {
                    lapis
                        .commands
                        .trigger_targets(Property::CodeF(expr.value()), e);
                }
            }
        }
        _ => return None,
    }
    Some(e)
}
