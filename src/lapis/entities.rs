use crate::{interaction::*, lapis::*, objects::*};
use avian2d::prelude::*;
use bevy::sprite::AlphaMode2d;

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
            commands.trigger(InsertDefaults(e, x, y, r));
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

pub fn method_entity(
    expr: &ExprMethodCall,
    lapis: &Lapis,
    commands: &mut Commands,
) -> Option<Entity> {
    let e = eval_entity(&expr.receiver, lapis, commands)?;
    match expr.method.to_string().as_str() {
        "x" => {
            let x = eval_float(expr.args.first()?, lapis)?;
            commands.trigger(SetX(e, x));
            Some(e)
        }
        "y" => None,
        "h" => None,
        "s" => None,
        // and so on..
        // TODO
        _ => None,
    }
}

pub fn entity_methods(expr: &ExprMethodCall, lapis: &Lapis, commands: &mut Commands) {
    if let Some(e) = eval_entity(&expr.receiver, lapis, commands) {
        if expr.method == "despawn" {
            if let Some(mut e) = commands.get_entity(e) {
                e.try_despawn();
            }
        }
    }
}

// ---- observers ----

#[derive(Event)]
pub struct SetX(Entity, f32);

pub fn set_x(trig: Trigger<SetX>, mut trans_query: Query<&mut Transform>) {
    trans_query.get_mut(trig.event().0).unwrap().translation.x = trig.event().1;
}

#[derive(Event)]
pub struct InsertDefaults(Entity, f32, f32, f32);

pub fn insert_defaults(
    trig: Trigger<InsertDefaults>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    settings: Res<DrawSettings>,
) {
    let InsertDefaults(e, x, y, r) = *trig.event();
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
    if settings.sensor {
        commands.entity(e).insert(Sensor);
    }
    if settings.custom_mass {
        commands.entity(e).insert(Mass(settings.mass));
    }
    if settings.custom_inertia {
        commands.entity(e).insert(AngularInertia(settings.inertia));
    }
}
