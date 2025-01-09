use crate::interaction::*;
use avian2d::prelude::*;
use bevy::{math::Affine2, prelude::*, render::view::VisibleEntities};

pub struct JointsPlugin;

impl Plugin for JointsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            spawn_joint
                .after(update_cursor_info)
                .run_if(resource_equals(EguiFocused(false)))
                .run_if(resource_equals(Mode::Joint)),
        )
        .add_observer(disjoint_observer)
        .add_observer(replace_joint)
        .add_observer(set_joint_property)
        .add_observer(joint_points);
    }
}

fn spawn_joint(
    mut commands: Commands,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    trans_query: Query<&Transform>,
    visible: Query<&VisibleEntities>,
    cursor: Res<CursorInfo>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    egui_focused: Res<EguiFocused>,
    settings: Res<JointSettings>,
    mut src: Local<Option<(Entity, Transform)>>,
) {
    if keyboard_input.pressed(KeyCode::Space) || egui_focused.is_changed() {
        return;
    }
    if mouse_button_input.just_pressed(MouseButton::Left) {
        for e in visible.single().get::<With<Mesh2d>>() {
            let t = trans_query.get(*e).unwrap();
            if cursor.i.distance_squared(t.translation.xy()) < t.scale.x * t.scale.x {
                *src = Some((*e, *t));
                break;
            }
        }
    } else if mouse_button_input.just_released(MouseButton::Left) {
        let mut snk = None;
        for e in visible.single().get::<With<Mesh2d>>() {
            let t = trans_query.get(*e).unwrap();
            if cursor.f.distance_squared(t.translation.xy()) < t.scale.x * t.scale.x {
                snk = Some(*e);
                break;
            }
        }
        if let (Some((src, src_trans)), Some(snk)) = (*src, snk) {
            if src == snk {
                return;
            }
            let anchors = if settings.custom_anchors {
                (settings.local_anchor_1, settings.local_anchor_2)
            } else {
                // doing this to account for rotation
                // might be missing something obvious here
                let src_rot = src_trans.rotation.to_euler(EulerRot::XYZ).2;
                let l1 = Affine2::from_angle_translation(src_rot, src_trans.translation.xy())
                    .inverse()
                    .transform_point2(cursor.i);
                let snk_trans = trans_query.get(snk).unwrap();
                let snk_rot = snk_trans.rotation.to_euler(EulerRot::XYZ).2;
                let l2 = Affine2::from_angle_translation(snk_rot, snk_trans.translation.xy())
                    .inverse()
                    .transform_point2(cursor.f);
                (l1, l2)
            };
            let compliance = settings.compliance / 100000.;
            match settings.joint_type {
                JointType::Fixed => {
                    commands.spawn(
                        FixedJoint::new(src, snk)
                            .with_compliance(compliance)
                            .with_local_anchor_1(anchors.0)
                            .with_local_anchor_2(anchors.1),
                    );
                }
                JointType::Distance => {
                    commands.spawn(
                        DistanceJoint::new(src, snk)
                            .with_compliance(compliance)
                            .with_local_anchor_1(anchors.0)
                            .with_local_anchor_2(anchors.1)
                            .with_limits(settings.dist_limits.0, settings.dist_limits.1)
                            .with_rest_length(settings.dist_rest),
                    );
                }
                JointType::Prismatic => {
                    commands.spawn(
                        PrismaticJoint::new(src, snk)
                            .with_compliance(compliance)
                            .with_local_anchor_1(anchors.0)
                            .with_local_anchor_2(anchors.1)
                            .with_free_axis(settings.prismatic_axis)
                            .with_limits(settings.prismatic_limits.0, settings.prismatic_limits.1),
                    );
                }
                JointType::Revolute => {
                    commands.spawn(
                        RevoluteJoint::new(src, snk)
                            .with_compliance(compliance)
                            .with_local_anchor_1(anchors.0)
                            .with_local_anchor_2(anchors.1)
                            .with_angle_limits(settings.angle_limits.0, settings.angle_limits.1),
                    );
                }
            }
        }
    }
}

// ---- observers ----

#[derive(Event, Clone)]
pub enum JointProperty {
    Compliance(f32),
    Anchor1(f32, f32),
    Anchor2(f32, f32),
    Limits(f32, f32),
    Rest(f32),
    FreeAxis(f32, f32),
}

pub fn set_joint_property(
    trig: Trigger<JointProperty>,
    mut fixed: Query<&mut FixedJoint>,
    mut distance: Query<&mut DistanceJoint>,
    mut revolute: Query<&mut RevoluteJoint>,
    mut prismatic: Query<&mut PrismaticJoint>,
) {
    let e = trig.entity();
    match *trig.event() {
        JointProperty::Compliance(val) => {
            if let Ok(mut j) = fixed.get_mut(e) {
                j.compliance = val / 100000.;
            } else if let Ok(mut j) = distance.get_mut(e) {
                j.compliance = val / 100000.;
            } else if let Ok(mut j) = revolute.get_mut(e) {
                j.compliance = val / 100000.;
            } else if let Ok(mut j) = prismatic.get_mut(e) {
                j.compliance = val / 100000.;
            }
        }
        JointProperty::Anchor1(x, y) => {
            if let Ok(mut j) = fixed.get_mut(e) {
                j.local_anchor1 = Vec2::new(x, y);
            } else if let Ok(mut j) = distance.get_mut(e) {
                j.local_anchor1 = Vec2::new(x, y);
            } else if let Ok(mut j) = revolute.get_mut(e) {
                j.local_anchor1 = Vec2::new(x, y);
            } else if let Ok(mut j) = prismatic.get_mut(e) {
                j.local_anchor1 = Vec2::new(x, y);
            }
        }
        JointProperty::Anchor2(x, y) => {
            if let Ok(mut j) = fixed.get_mut(e) {
                j.local_anchor2 = Vec2::new(x, y);
            } else if let Ok(mut j) = distance.get_mut(e) {
                j.local_anchor2 = Vec2::new(x, y);
            } else if let Ok(mut j) = revolute.get_mut(e) {
                j.local_anchor2 = Vec2::new(x, y);
            } else if let Ok(mut j) = prismatic.get_mut(e) {
                j.local_anchor2 = Vec2::new(x, y);
            }
        }
        JointProperty::Limits(min, max) => {
            if let Ok(mut j) = distance.get_mut(e) {
                j.length_limits = Some(DistanceLimit::new(min, max));
            } else if let Ok(mut j) = prismatic.get_mut(e) {
                j.free_axis_limits = Some(DistanceLimit::new(min, max));
            } else if let Ok(mut j) = revolute.get_mut(e) {
                j.angle_limit = Some(AngleLimit::new(min, max));
            }
        }
        JointProperty::Rest(val) => {
            if let Ok(mut j) = distance.get_mut(e) {
                j.rest_length = val;
            }
        }
        JointProperty::FreeAxis(x, y) => {
            if let Ok(mut j) = prismatic.get_mut(e) {
                j.free_axis = Vec2::new(x, y);
            }
        }
    }
}

#[derive(Event)]
pub struct ReplaceJoint(pub JointType);

fn replace_joint(
    trig: Trigger<ReplaceJoint>,
    mut commands: Commands,
    fixed: Query<&FixedJoint>,
    distance: Query<&DistanceJoint>,
    revolute: Query<&RevoluteJoint>,
    prismatic: Query<&PrismaticJoint>,
) {
    let e = trig.entity();
    let joint_type = &trig.event().0;
    let e1;
    let e2;
    let anchors;
    let compliance;
    if let Ok(j) = fixed.get(e) {
        e1 = j.entity1;
        e2 = j.entity2;
        anchors = (j.local_anchor1, j.local_anchor2);
        compliance = j.compliance;
    } else if let Ok(j) = distance.get(e) {
        e1 = j.entity1;
        e2 = j.entity2;
        anchors = (j.local_anchor1, j.local_anchor2);
        compliance = j.compliance;
    } else if let Ok(j) = prismatic.get(e) {
        e1 = j.entity1;
        e2 = j.entity2;
        anchors = (j.local_anchor1, j.local_anchor2);
        compliance = j.compliance;
    } else if let Ok(j) = revolute.get(e) {
        e1 = j.entity1;
        e2 = j.entity2;
        anchors = (j.local_anchor1, j.local_anchor2);
        compliance = j.compliance;
    } else {
        return;
    }
    match joint_type {
        JointType::Fixed => {
            commands.entity(e).clear().insert(
                FixedJoint::new(e1, e2)
                    .with_compliance(compliance)
                    .with_local_anchor_1(anchors.0)
                    .with_local_anchor_2(anchors.1),
            );
        }
        JointType::Distance => {
            commands.entity(e).clear().insert(
                DistanceJoint::new(e1, e2)
                    .with_compliance(compliance)
                    .with_local_anchor_1(anchors.0)
                    .with_local_anchor_2(anchors.1),
            );
        }
        JointType::Prismatic => {
            commands.entity(e).clear().insert(
                PrismaticJoint::new(e1, e2)
                    .with_compliance(compliance)
                    .with_local_anchor_1(anchors.0)
                    .with_local_anchor_2(anchors.1),
            );
        }
        JointType::Revolute => {
            commands.entity(e).clear().insert(
                RevoluteJoint::new(e1, e2)
                    .with_compliance(compliance)
                    .with_local_anchor_1(anchors.0)
                    .with_local_anchor_2(anchors.1),
            );
        }
    }
}

#[derive(Event)]
pub struct Disjoint;

fn disjoint_observer(
    trig: Trigger<Disjoint>,
    mut commands: Commands,
    fixed: Query<(Entity, &FixedJoint)>,
    distance: Query<(Entity, &DistanceJoint)>,
    revolute: Query<(Entity, &RevoluteJoint)>,
    prismatic: Query<(Entity, &PrismaticJoint)>,
) {
    let object = trig.entity();
    for (e, j) in fixed.iter() {
        if j.entity1 == object || j.entity2 == object {
            commands.entity(e).despawn();
        }
    }
    for (e, j) in distance.iter() {
        if j.entity1 == object || j.entity2 == object {
            commands.entity(e).despawn();
        }
    }
    for (e, j) in revolute.iter() {
        if j.entity1 == object || j.entity2 == object {
            commands.entity(e).despawn();
        }
    }
    for (e, j) in prismatic.iter() {
        if j.entity1 == object || j.entity2 == object {
            commands.entity(e).despawn();
        }
    }
}

#[derive(Event)]
pub struct JointPoints(pub Vec2, pub Vec2);

fn joint_points(
    trig: Trigger<JointPoints>,
    mut commands: Commands,
    objects_query: Query<(Entity, &Transform), With<RigidBody>>,
    settings: Res<JointSettings>,
) {
    let joint_entity = trig.entity();
    let JointPoints(i, f) = *trig.event();
    let mut src = None;
    let mut snk = None;
    for (e, t) in objects_query.iter() {
        if i.distance_squared(t.translation.xy()) < t.scale.x * t.scale.x {
            src = Some((e, *t));
        } else if f.distance_squared(t.translation.xy()) < t.scale.x * t.scale.x {
            snk = Some((e, *t));
        }
    }
    if let (Some((src, src_trans)), Some((snk, snk_trans))) = (src, snk) {
        let anchors = if settings.custom_anchors {
            (settings.local_anchor_1, settings.local_anchor_2)
        } else {
            let src_rot = src_trans.rotation.to_euler(EulerRot::XYZ).2;
            let l1 = Affine2::from_angle_translation(src_rot, src_trans.translation.xy())
                .inverse()
                .transform_point2(i);
            let snk_rot = snk_trans.rotation.to_euler(EulerRot::XYZ).2;
            let l2 = Affine2::from_angle_translation(snk_rot, snk_trans.translation.xy())
                .inverse()
                .transform_point2(f);
            (l1, l2)
        };
        let compliance = settings.compliance / 100000.;
        match settings.joint_type {
            JointType::Fixed => {
                commands.entity(joint_entity).insert(
                    FixedJoint::new(src, snk)
                        .with_compliance(compliance)
                        .with_local_anchor_1(anchors.0)
                        .with_local_anchor_2(anchors.1),
                );
            }
            JointType::Distance => {
                commands.entity(joint_entity).insert(
                    DistanceJoint::new(src, snk)
                        .with_compliance(compliance)
                        .with_local_anchor_1(anchors.0)
                        .with_local_anchor_2(anchors.1)
                        .with_limits(settings.dist_limits.0, settings.dist_limits.1)
                        .with_rest_length(settings.dist_rest),
                );
            }
            JointType::Prismatic => {
                commands.entity(joint_entity).insert(
                    PrismaticJoint::new(src, snk)
                        .with_compliance(compliance)
                        .with_local_anchor_1(anchors.0)
                        .with_local_anchor_2(anchors.1)
                        .with_free_axis(settings.prismatic_axis)
                        .with_limits(settings.prismatic_limits.0, settings.prismatic_limits.1),
                );
            }
            JointType::Revolute => {
                commands.entity(joint_entity).insert(
                    RevoluteJoint::new(src, snk)
                        .with_compliance(compliance)
                        .with_local_anchor_1(anchors.0)
                        .with_local_anchor_2(anchors.1)
                        .with_angle_limits(settings.angle_limits.0, settings.angle_limits.1),
                );
            }
        }
    } else {
        commands.entity(joint_entity).despawn();
    }
}
