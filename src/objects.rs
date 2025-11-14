use crate::{
    interaction::*,
    lapis::{Lapis, floats::eval_float_f32},
};
use avian2d::prelude::*;
use bevy::{prelude::*, sprite::AlphaMode2d};
use std::collections::VecDeque;
use syn::{Expr, parse_str};

pub struct ObjectsPlugin;

impl Plugin for ObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            spawn
                .run_if(resource_equals(EguiFocused(false)))
                .run_if(resource_equals(Mode::Draw)),
        )
        .add_systems(PhysicsSchedule, attract.in_set(PhysicsStepSet::Last))
        .add_systems(Update, eval_collisions)
        .add_systems(PostUpdate, sync_links)
        .add_systems(Update, update_tail)
        .insert_resource(AttractionFactor(0.01))
        .add_observer(set_property)
        .add_observer(insert_defaults);
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Code(pub String, pub String);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Links(pub String);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Sides(pub u32);

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct AttractionFactor(pub f32);

#[derive(Component, Default)]
pub struct Tail {
    pub len: usize,
    pub points: VecDeque<(Vec2, Color)>,
}

fn update_tail(
    mut tail_query: Query<(Entity, &Transform, &mut Tail)>,
    mut gizmos: Gizmos,
    material_ids: Query<&MeshMaterial2d<ColorMaterial>>,
    materials: ResMut<Assets<ColorMaterial>>,
    time: ResMut<Time<Virtual>>,
) {
    for (e, trans, mut tail) in tail_query.iter_mut() {
        let mut prev = trans.translation.xy();
        for (pos, col) in &tail.points {
            gizmos.line_2d(prev, *pos, *col);
            prev = *pos;
        }
        if !time.is_paused() {
            let mat_id = material_ids.get(e).unwrap();
            let mat = materials.get(mat_id).unwrap();
            tail.points.push_front((trans.translation.xy(), mat.color));
            while tail.points.len() > tail.len {
                tail.points.pop_back();
            }
        }
    }
}

fn spawn(
    mut commands: Commands,
    cursor: Res<CursorInfo>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    settings: Res<DrawSettings>,
    egui_focused: Res<EguiFocused>,
) {
    if !keyboard_input.pressed(KeyCode::Space)
        && mouse_button_input.just_released(MouseButton::Left)
        // avoid spawning when dragging outside of egui
        && !egui_focused.is_changed()
    {
        let r = cursor.f.distance(cursor.i).max(1.);
        let material = ColorMaterial {
            color: Srgba::from_u8_array(settings.color).into(),
            alpha_mode: AlphaMode2d::Blend,
            ..default()
        };
        let mesh_handle = meshes.add(RegularPolygon::new(1., settings.sides));
        let mat_handle = materials.add(material);
        let layer = 1 << settings.collision_layer;
        let mut e = commands.spawn((
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
                CollisionEventsEnabled,
                LinearDamping(settings.lin_damp),
                AngularDamping(settings.ang_damp),
                Restitution::new(settings.restitution),
                Friction::new(settings.friction),
            ),
            Transform {
                translation: cursor.i.extend(0.),
                scale: Vec3::new(r, r, 1.),
                ..default()
            },
            Sides(settings.sides),
            Tail {
                len: settings.tail,
                ..default()
            },
            SleepingDisabled,
        ));
        if settings.sensor {
            e.insert(Sensor);
        }
        if settings.custom_mass {
            e.insert(Mass(settings.mass));
        }
        if settings.custom_inertia {
            e.insert(AngularInertia(settings.inertia));
        }
    }
}

fn attract(
    layers: Query<(Entity, &CollisionLayers)>,
    mut query: Query<(&Mass, &Position, &mut LinearVelocity)>,
    factor: Res<AttractionFactor>,
) {
    if !factor.0.is_normal() {
        return;
    }
    let mut combinations = layers.iter_combinations();
    while let Some([(e1, l1), (e2, l2)]) = combinations.fetch_next() {
        if l1 == l2 {
            let [mut e1, mut e2] = query.get_many_mut([e1, e2]).unwrap();
            let m1 = e1.0.0;
            let m2 = e2.0.0;
            let p1 = e1.1.0;
            let p2 = e2.1.0;
            let r = p1.distance_squared(p2);
            if r > 1. {
                e1.2.0 += (p2 - p1) * m2 / r * factor.0;
                e2.2.0 += (p1 - p2) * m1 / r * factor.0;
            }
        }
    }
}

fn replace(code: &str, e1: Entity, e2: Entity) -> String {
    code.replace("$id", &format!("{}", e1.to_bits()))
        .replace("$other", &format!("{}", e2.to_bits()))
}

fn eval_collisions(
    code: Query<&Code>,
    mut lapis: Lapis,
    mut started: EventReader<CollisionStarted>,
    mut ended: EventReader<CollisionEnded>,
) {
    if lapis.data.quiet {
        for CollisionStarted(e1, e2) in started.read() {
            if let Ok(c) = code.get(*e1) {
                lapis.quiet_eval(&replace(&c.0, *e1, *e2));
            }
            if let Ok(c) = code.get(*e2) {
                lapis.quiet_eval(&replace(&c.0, *e2, *e1));
            }
        }
        for CollisionEnded(e1, e2) in ended.read() {
            if let Ok(c) = code.get(*e1) {
                lapis.quiet_eval(&replace(&c.1, *e1, *e2));
            }
            if let Ok(c) = code.get(*e2) {
                lapis.quiet_eval(&replace(&c.1, *e2, *e1));
            }
        }
    } else {
        for CollisionStarted(e1, e2) in started.read() {
            if let Ok(c) = code.get(*e1) {
                lapis.eval(&replace(&c.0, *e1, *e2));
            }
            if let Ok(c) = code.get(*e2) {
                lapis.eval(&replace(&c.0, *e2, *e1));
            }
        }
        for CollisionEnded(e1, e2) in ended.read() {
            if let Ok(c) = code.get(*e1) {
                lapis.eval(&replace(&c.1, *e1, *e2));
            }
            if let Ok(c) = code.get(*e2) {
                lapis.eval(&replace(&c.1, *e2, *e1));
            }
        }
    }
}

fn sync_links(links_query: Query<(Entity, &Links)>, mut lapis: Lapis) {
    for (e, Links(links)) in links_query.iter() {
        for link in links.lines() {
            // links are in the form "property > var" or "property < var"
            let mut link = link.split_ascii_whitespace();
            let s0 = link.next();
            let s1 = link.next();
            let s2 = link.next();
            let (Some(property), Some(dir), Some(var)) = (s0, s1, s2) else {
                continue;
            };
            if let Some(var) = lapis.data.smap.get(var) {
                let cmd = &mut lapis.commands;
                match property {
                    "x" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::X(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.trans_query.get(e).unwrap().translation.x);
                        }
                    }
                    "y" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Y(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.trans_query.get(e).unwrap().translation.y);
                        }
                    }
                    "z" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Z(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.trans_query.get(e).unwrap().translation.z);
                        }
                    }
                    "rx" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Rx(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.trans_query.get(e).unwrap().scale.x);
                        }
                    }
                    "ry" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Ry(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.trans_query.get(e).unwrap().scale.y);
                        }
                    }
                    "rot" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Rot(var.value()), e);
                        } else if dir == ">" {
                            let trans = lapis.trans_query.get(e).unwrap();
                            let rot = trans.rotation.to_euler(EulerRot::XYZ).2;
                            var.set(rot);
                        }
                    }
                    "mass" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Mass(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.mass_query.get_mut(e).unwrap().0);
                        }
                    }
                    "vx" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Vx(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.lin_velocity_query.get(e).unwrap().x);
                        }
                    }
                    "vy" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Vy(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.lin_velocity_query.get(e).unwrap().y);
                        }
                    }
                    "va" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Va(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.ang_velocity_query.get(e).unwrap().0);
                        }
                    }
                    "vm" => {
                        let v = lapis.lin_velocity_query.get(e).unwrap();
                        if dir == "<" {
                            let m = var.value();
                            let p = v.y.atan2(v.x);
                            cmd.trigger_targets(Property::Vx(m * p.cos()), e);
                            cmd.trigger_targets(Property::Vy(m * p.sin()), e);
                        } else {
                            var.set(v.x.hypot(v.y));
                        }
                    }
                    "vp" => {
                        let v = lapis.lin_velocity_query.get(e).unwrap();
                        if dir == "<" {
                            let m = v.x.hypot(v.y);
                            let p = var.value();
                            cmd.trigger_targets(Property::Vx(m * p.cos()), e);
                            cmd.trigger_targets(Property::Vy(m * p.sin()), e);
                        } else {
                            var.set(v.y.atan2(v.x));
                        }
                    }
                    "restitution" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Restitution(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.restitution_query.get(e).unwrap().coefficient);
                        }
                    }
                    "lindamp" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::LinDamp(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.lin_damp_query.get(e).unwrap().0);
                        }
                    }
                    "angdamp" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::AngDamp(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.ang_damp_query.get(e).unwrap().0);
                        }
                    }
                    "inertia" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Inertia(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.inertia_query.get(e).unwrap().0);
                        }
                    }
                    "h" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::H(var.value()), e);
                        } else if dir == ">" {
                            let mat_id = lapis.material_ids.get(e).unwrap();
                            let mat = lapis.materials.get(mat_id).unwrap();
                            let hsla: Hsla = mat.color.into();
                            var.set(hsla.hue);
                        }
                    }
                    "s" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::S(var.value()), e);
                        } else if dir == ">" {
                            let mat_id = lapis.material_ids.get(e).unwrap();
                            let mat = lapis.materials.get(mat_id).unwrap();
                            let hsla: Hsla = mat.color.into();
                            var.set(hsla.saturation);
                        }
                    }
                    "l" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::L(var.value()), e);
                        } else if dir == ">" {
                            let mat_id = lapis.material_ids.get(e).unwrap();
                            let mat = lapis.materials.get(mat_id).unwrap();
                            let hsla: Hsla = mat.color.into();
                            var.set(hsla.lightness);
                        }
                    }
                    "a" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::A(var.value()), e);
                        } else if dir == ">" {
                            let mat_id = lapis.material_ids.get(e).unwrap();
                            let mat = lapis.materials.get(mat_id).unwrap();
                            let hsla: Hsla = mat.color.into();
                            var.set(hsla.alpha);
                        }
                    }
                    "sides" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Sides(var.value() as u32), e);
                        } else if dir == ">" {
                            var.set(lapis.sides_query.get(e).unwrap().0 as f32);
                        }
                    }
                    "cmx" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Cmx(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.cm_query.get(e).unwrap().0.x);
                        }
                    }
                    "cmy" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Cmy(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.cm_query.get(e).unwrap().0.y);
                        }
                    }
                    "friction" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Friction(var.value()), e);
                        } else if dir == ">" {
                            var.set(lapis.friction_query.get(e).unwrap().dynamic_coefficient);
                        }
                    }
                    "tail" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Tail(var.value() as usize), e);
                        } else if dir == ">" {
                            var.set(lapis.tail_query.get(e).unwrap().len as f32);
                        }
                    }
                    "layer" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Layer(var.value() as u32), e);
                        } else if dir == ">" {
                            let l = lapis.layer_query.get(e).unwrap().memberships.0;
                            var.set(l.ilog2() as f32);
                        }
                    }
                    "dynamic" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Dynamic(var.value() > 0.), e);
                        } else if dir == ">" {
                            let body = lapis.body_query.get(e).unwrap();
                            var.set((*body == RigidBody::Dynamic).into());
                        }
                    }
                    "sensor" => {
                        if dir == "<" {
                            cmd.trigger_targets(Property::Sensor(var.value() > 0.), e);
                        } else if dir == ">" {
                            var.set(lapis.sensor_query.contains(e).into());
                        }
                    }
                    _ => {}
                }
            // assign a float expression
            } else if (dir == "<" || dir == "=")
                && let Ok(expr) = parse_str::<Expr>(var)
                && let Some(f) = eval_float_f32(&expr, &lapis)
            {
                let cmd = &mut lapis.commands;
                match property {
                    "x" => cmd.trigger_targets(Property::X(f), e),
                    "y" => cmd.trigger_targets(Property::Y(f), e),
                    "z" => cmd.trigger_targets(Property::Z(f), e),
                    "rx" => cmd.trigger_targets(Property::Rx(f), e),
                    "ry" => cmd.trigger_targets(Property::Ry(f), e),
                    "rot" => cmd.trigger_targets(Property::Rot(f), e),
                    "mass" => cmd.trigger_targets(Property::Mass(f), e),
                    "vx" => cmd.trigger_targets(Property::Vx(f), e),
                    "vy" => cmd.trigger_targets(Property::Vy(f), e),
                    "va" => cmd.trigger_targets(Property::Va(f), e),
                    "vm" => {
                        let v = lapis.lin_velocity_query.get(e).unwrap();
                        let m = f;
                        let p = v.y.atan2(v.x);
                        cmd.trigger_targets(Property::Vx(m * p.cos()), e);
                        cmd.trigger_targets(Property::Vy(m * p.sin()), e);
                    }
                    "vp" => {
                        let v = lapis.lin_velocity_query.get(e).unwrap();
                        let m = v.x.hypot(v.y);
                        let p = f;
                        cmd.trigger_targets(Property::Vx(m * p.cos()), e);
                        cmd.trigger_targets(Property::Vy(m * p.sin()), e);
                    }
                    "restitution" => cmd.trigger_targets(Property::Restitution(f), e),
                    "lindamp" => cmd.trigger_targets(Property::LinDamp(f), e),
                    "angdamp" => cmd.trigger_targets(Property::AngDamp(f), e),
                    "inertia" => cmd.trigger_targets(Property::Inertia(f), e),
                    "h" => cmd.trigger_targets(Property::H(f), e),
                    "s" => cmd.trigger_targets(Property::S(f), e),
                    "l" => cmd.trigger_targets(Property::L(f), e),
                    "a" => cmd.trigger_targets(Property::A(f), e),
                    "sides" => cmd.trigger_targets(Property::Sides(f as u32), e),
                    "cmx" => cmd.trigger_targets(Property::Cmx(f), e),
                    "cmy" => cmd.trigger_targets(Property::Cmy(f), e),
                    "friction" => cmd.trigger_targets(Property::Friction(f), e),
                    "tail" => cmd.trigger_targets(Property::Tail(f as usize), e),
                    "layer" => cmd.trigger_targets(Property::Layer(f as u32), e),
                    "dynamic" => cmd.trigger_targets(Property::Dynamic(f > 0.), e),
                    "sensor" => cmd.trigger_targets(Property::Sensor(f > 0.), e),
                    _ => {}
                }
            }
        }
    }
}

// ---- observers ----

#[derive(Event, Clone)]
pub enum Property {
    X(f32),
    Y(f32),
    Z(f32),
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

pub fn set_property(
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
    selected_query: Query<Entity, With<Selected>>,
) {
    let e = trig.target();
    // methods applied to PLACEHOLDER affect the selected entities
    if e == Entity::PLACEHOLDER {
        let mut targets = Vec::new();
        for e in selected_query.iter() {
            targets.push(e);
        }
        if !targets.is_empty() {
            commands.trigger_targets(trig.event().clone(), targets);
        }
        return;
    }
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
        Property::Z(val) => {
            if let Ok(mut t) = trans_query.get_mut(e) {
                t.translation.z = val;
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
            let val = val.clamp(3, 512);
            if let Ok(mesh_id) = mesh_ids.get(e) {
                // hack to make sure the new mesh is in VisibleEntities (selectable)
                trans_query.get_mut(e).unwrap().set_changed();
                let mesh = meshes.get_mut(mesh_id).unwrap();
                *mesh = RegularPolygon::new(1., val).into();
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
                let layer = 1 << val.clamp(0, 31);
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
pub struct InsertDefaults(pub f32);

pub fn insert_defaults(
    trig: Trigger<InsertDefaults>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    settings: Res<DrawSettings>,
) {
    let e = trig.target();
    let r = trig.event().0;
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
            CollisionEventsEnabled,
            LinearDamping(settings.lin_damp),
            AngularDamping(settings.ang_damp),
            Restitution::new(settings.restitution),
            Friction::new(settings.friction),
        ),
        Transform::from_scale(Vec3::new(r, r, 1.)),
        Sides(settings.sides),
        Tail {
            len: settings.tail,
            ..default()
        },
        SleepingDisabled,
    ));
    if settings.sensor {
        commands.entity(e).insert(Sensor);
    }
}
