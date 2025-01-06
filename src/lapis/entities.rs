use crate::{interaction::*, lapis::*, objects::*};
use avian2d::prelude::*;
use bevy::sprite::AlphaMode2d;

//TODO joints not spawned programmatically are not accessible
//TODO do we need Vec<Entity>? can we spawn structures like grids?
//      we can avoid this need by making join take coordinates instead and
//      so it becomes a copy of the spawn_joint system

pub fn eval_entity(expr: &Expr, lapis: &Lapis, commands: &mut Commands) -> Option<Entity> {
    match expr {
        Expr::Call(expr) => call_entity(expr, lapis, commands),
        Expr::Lit(expr) => lit_entity(&expr.lit),
        Expr::Path(expr) => path_entity(&expr.path, lapis),
        Expr::MethodCall(expr) => method_entity(expr, lapis, commands),
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
    lapis.entitymap.get(&k).copied()
}

fn call_entity(expr: &ExprCall, lapis: &Lapis, commands: &mut Commands) -> Option<Entity> {
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
            let x = eval_float(expr.args.first()?, lapis)?;
            let y = eval_float(expr.args.get(1)?, lapis)?;
            let r = eval_float(expr.args.get(2)?, lapis)?;
            let e = commands.spawn_empty().id();
            commands.trigger_targets(InsertDefaults(x, y, r), e);
            Some(e)
        }
        "joint" => {
            let e1 = eval_entity(expr.args.first()?, lapis, commands)?;
            let e2 = eval_entity(expr.args.get(1)?, lapis, commands)?;
            let joint_type = nth_path_ident(expr.args.get(2)?, 0)?;
            match joint_type.as_str() {
                "fixed" => Some(commands.spawn(FixedJoint::new(e1, e2)).id()),
                "distance" => Some(commands.spawn(DistanceJoint::new(e1, e2)).id()),
                "prismatic" => Some(commands.spawn(PrismaticJoint::new(e1, e2)).id()),
                "revolute" => Some(commands.spawn(RevoluteJoint::new(e1, e2)).id()),
                _ => None,
            }
            // TODO: trigger to read the joint settings resource like spawn does
        }
        _ => None,
    }
}

fn method_entity(expr: &ExprMethodCall, lapis: &Lapis, commands: &mut Commands) -> Option<Entity> {
    let e = eval_entity(&expr.receiver, lapis, commands)?;
    // this being here allows some nonsense like
    // let var = entity.despawn();
    // which doesn't assign anything to var but does despawn entity
    if expr.method == "despawn" {
        commands.get_entity(e)?.try_despawn();
        return None;
    }
    let val = eval_float(expr.args.first()?, lapis);
    match expr.method.to_string().as_str() {
        "x" => commands.trigger_targets(Property::X(val?), e),
        "y" => commands.trigger_targets(Property::Y(val?), e),
        "rx" => commands.trigger_targets(Property::Rx(val?), e),
        "ry" => commands.trigger_targets(Property::Ry(val?), e),
        "rot" => commands.trigger_targets(Property::Rot(val?), e),
        "mass" => commands.trigger_targets(Property::Mass(val?), e),
        "vx" => commands.trigger_targets(Property::Vx(val?), e),
        "vy" => commands.trigger_targets(Property::Vy(val?), e),
        "va" => commands.trigger_targets(Property::Va(val?), e),
        "restitution" => commands.trigger_targets(Property::Restitution(val?), e),
        "lindamp" => commands.trigger_targets(Property::LinDamp(val?), e),
        "angdamp" => commands.trigger_targets(Property::AngDamp(val?), e),
        "inertia" => commands.trigger_targets(Property::Inertia(val?), e),
        "h" => commands.trigger_targets(Property::H(val?), e),
        "s" => commands.trigger_targets(Property::S(val?), e),
        "l" => commands.trigger_targets(Property::L(val?), e),
        "a" => commands.trigger_targets(Property::A(val?), e),
        "sides" => commands.trigger_targets(Property::Sides(val? as u32), e),
        "cmx" => commands.trigger_targets(Property::Cmx(val?), e),
        "cmy" => commands.trigger_targets(Property::Cmy(val?), e),
        "friction" => commands.trigger_targets(Property::Friction(val?), e),
        "tail" => commands.trigger_targets(Property::Tail(val? as usize), e),
        "layer" => commands.trigger_targets(Property::Layer(val? as u32), e),
        "dynamic" => {
            let b = eval_bool(expr.args.first()?, lapis)?;
            commands.trigger_targets(Property::Dynamic(b), e);
        }
        "sensor" => {
            let b = eval_bool(expr.args.first()?, lapis)?;
            commands.trigger_targets(Property::Sensor(b), e);
        }
        "links" => {
            if let Expr::Lit(expr) = expr.args.first()? {
                if let Lit::Str(expr) = &expr.lit {
                    commands.trigger_targets(Property::Links(expr.value()), e);
                }
            }
        }
        "code_i" => {
            if let Expr::Lit(expr) = expr.args.first()? {
                if let Lit::Str(expr) = &expr.lit {
                    commands.trigger_targets(Property::CodeI(expr.value()), e);
                }
            }
        }
        "code_f" => {
            if let Expr::Lit(expr) = expr.args.first()? {
                if let Lit::Str(expr) = &expr.lit {
                    commands.trigger_targets(Property::CodeF(expr.value()), e);
                }
            }
        }
        _ => return None,
    }
    Some(e)
}

// ---- observers ----

#[derive(Event)]
pub enum Property {
    X(f32),
    Y(f32),
    Rx(f32),
    Ry(f32),
    Rot(f32),
    Mass(f32),
    Vx(f32),
    Vy(f32),
    Va(f32),
    Restitution(f32),
    LinDamp(f32),
    AngDamp(f32),
    Inertia(f32),
    H(f32),
    S(f32),
    L(f32),
    A(f32),
    Sides(u32),
    Cmx(f32),
    Cmy(f32),
    Friction(f32),
    Tail(usize),
    Layer(u32),
    Dynamic(bool),
    Sensor(bool),
    Links(String),
    CodeI(String),
    CodeF(String),
}

pub fn set_observer(
    trig: Trigger<Property>,
    mut trans_query: Query<&mut Transform, With<RigidBody>>,
    mut commands: Commands,
    mut lin_velocity_query: Query<&mut LinearVelocity>,
    mesh_ids: Query<&Mesh2d>,
    mut meshes: ResMut<Assets<Mesh>>,
    material_ids: Query<&MeshMaterial2d<ColorMaterial>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut cm_query: Query<&mut CenterOfMass>,
    mut tail_query: Query<&mut Tail>,
    mut code_query: Query<&mut Code>,
) {
    let e = trig.entity();
    match *trig.event() {
        Property::X(val) => {
            if let Ok(mut t) = trans_query.get_mut(e) {
                t.translation.x = val;
            }
        }
        Property::Y(val) => {
            if let Ok(mut t) = trans_query.get_mut(e) {
                t.translation.y = val;
            }
        }
        Property::Rx(val) => {
            if let Ok(mut t) = trans_query.get_mut(e) {
                t.scale.x = val;
            }
        }
        Property::Ry(val) => {
            if let Ok(mut t) = trans_query.get_mut(e) {
                t.scale.y = val;
            }
        }
        Property::Rot(val) => {
            if let Ok(mut t) = trans_query.get_mut(e) {
                t.rotation = Quat::from_rotation_z(val);
            }
        }
        Property::Mass(val) => {
            if trans_query.contains(e) {
                commands.entity(e).insert(Mass(val));
            }
        }
        Property::Vx(val) => {
            if let Ok(mut v) = lin_velocity_query.get_mut(e) {
                v.x = val;
            }
        }
        Property::Vy(val) => {
            if let Ok(mut v) = lin_velocity_query.get_mut(e) {
                v.y = val;
            }
        }
        Property::Va(val) => {
            if trans_query.contains(e) {
                commands.entity(e).insert(AngularVelocity(val));
            }
        }
        Property::Restitution(val) => {
            if trans_query.contains(e) {
                commands.entity(e).insert(Restitution::new(val));
            }
        }
        Property::LinDamp(val) => {
            if trans_query.contains(e) {
                commands.entity(e).insert(LinearDamping(val));
            }
        }
        Property::AngDamp(val) => {
            if trans_query.contains(e) {
                commands.entity(e).insert(AngularDamping(val));
            }
        }
        Property::Inertia(val) => {
            if trans_query.contains(e) {
                commands.entity(e).insert(AngularInertia(val));
            }
        }
        Property::H(val) => {
            if let Ok(mat_id) = material_ids.get(e) {
                let mat = materials.get_mut(mat_id).unwrap();
                let mut hsla: Hsla = mat.color.into();
                hsla.hue = val;
                mat.color = hsla.into();
            }
        }
        Property::S(val) => {
            if let Ok(mat_id) = material_ids.get(e) {
                let mat = materials.get_mut(mat_id).unwrap();
                let mut hsla: Hsla = mat.color.into();
                hsla.saturation = val;
                mat.color = hsla.into();
            }
        }
        Property::L(val) => {
            if let Ok(mat_id) = material_ids.get(e) {
                let mat = materials.get_mut(mat_id).unwrap();
                let mut hsla: Hsla = mat.color.into();
                hsla.lightness = val;
                mat.color = hsla.into();
            }
        }
        Property::A(val) => {
            if let Ok(mat_id) = material_ids.get(e) {
                let mat = materials.get_mut(mat_id).unwrap();
                let mut hsla: Hsla = mat.color.into();
                hsla.alpha = val;
                mat.color = hsla.into();
            }
        }
        Property::Sides(val) => {
            let sides = (val).clamp(3, 512);
            if let Ok(mesh_id) = mesh_ids.get(e) {
                let mesh = meshes.get_mut(mesh_id).unwrap();
                *mesh = RegularPolygon::new(1., sides).into();
                commands
                    .entity(e)
                    .insert((Sides(val), Collider::regular_polygon(1., val)));
            }
        }
        Property::Cmx(val) => {
            if let Ok(mut v) = cm_query.get_mut(e) {
                v.x = val;
            }
        }
        Property::Cmy(val) => {
            if let Ok(mut v) = cm_query.get_mut(e) {
                v.y = val;
            }
        }
        Property::Friction(val) => {
            if trans_query.contains(e) {
                commands.entity(e).insert(Friction::new(val));
            }
        }
        Property::Tail(val) => {
            if let Ok(mut tail) = tail_query.get_mut(e) {
                tail.len = val;
            }
        }
        Property::Layer(val) => {
            if trans_query.contains(e) {
                let layer = 1 << val;
                commands
                    .entity(e)
                    .insert(CollisionLayers::from_bits(layer, layer));
            }
        }
        Property::Dynamic(val) => {
            if trans_query.contains(e) {
                if val {
                    commands.entity(e).insert(RigidBody::Dynamic);
                } else {
                    commands.entity(e).insert(RigidBody::Static);
                }
            }
        }
        Property::Sensor(val) => {
            if trans_query.contains(e) {
                if val {
                    commands.entity(e).insert(Sensor);
                } else {
                    commands.entity(e).remove::<Sensor>();
                }
            }
        }
        Property::Links(ref val) => {
            if trans_query.contains(e) {
                commands.entity(e).insert(Links(val.clone()));
            }
        }
        Property::CodeI(ref val) => {
            if let Ok(mut c) = code_query.get_mut(e) {
                c.0 = val.clone();
            }
        }
        Property::CodeF(ref val) => {
            if let Ok(mut c) = code_query.get_mut(e) {
                c.1 = val.clone();
            }
        }
    }
}

#[derive(Event)]
pub struct InsertDefaults(f32, f32, f32);

pub fn insert_defaults(
    trig: Trigger<InsertDefaults>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    settings: Res<DrawSettings>,
) {
    let e = trig.entity();
    let InsertDefaults(x, y, r) = *trig.event();
    let material = ColorMaterial {
        color: Srgba::from_u8_array(settings.color).into(),
        alpha_mode: AlphaMode2d::Blend,
        ..default()
    };
    let mesh_handle = meshes.add(RegularPolygon::new(1., settings.sides));
    let mat_handle = materials.add(material);
    let layer = 1 << settings.collision_layer;
    commands.entity(e).insert((
        Mesh2d(mesh_handle),
        MeshMaterial2d(mat_handle),
        settings.rigid_body,
        Links(settings.links.clone()),
        Code(settings.code.0.clone(), settings.code.1.clone()),
        Mass(r * r * r),
        AngularInertia(r * r * r),
        CenterOfMass(settings.center_of_mass),
        Collider::regular_polygon(1., settings.sides),
        CollisionLayers::from_bits(layer, layer),
        (
            LinearDamping(settings.lin_damp),
            AngularDamping(settings.ang_damp),
            Restitution::new(settings.restitution),
            Friction::new(settings.friction),
        ),
        Transform {
            translation: Vec3::new(x, y, 0.),
            scale: Vec3::new(r, r, 1.),
            ..default()
        },
        Sides(settings.sides),
        Tail {
            len: settings.tail,
            ..default()
        },
    ));
}
