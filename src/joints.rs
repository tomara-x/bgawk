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
        );
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
