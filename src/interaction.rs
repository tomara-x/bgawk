use avian2d::prelude::*;
use bevy::{
    color::palettes::tailwind::{GRAY_50, GREEN_500, RED_500},
    prelude::*,
    render::view::VisibleEntities,
};
use bevy_egui::EguiContexts;
use bevy_pancam::*;
use std::any::TypeId;
use std::f32::consts::TAU;

pub struct InteractPlugin;

impl Plugin for InteractPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CursorInfo::default())
            .init_resource::<Mode>()
            .insert_resource(ClickedOnSpace(true))
            .insert_resource(EguiFocused(false))
            .insert_resource(DrawSettings::default())
            .insert_resource(JointSettings::default())
            .add_systems(Update, toggle_pan)
            .add_systems(Update, check_egui_focus)
            .add_systems(Update, update_cursor_info)
            .add_systems(Update, switch_modes)
            .add_systems(
                Update,
                select_all
                    .run_if(resource_equals(EguiFocused(false)))
                    .run_if(resource_equals(Mode::Edit)),
            )
            .add_systems(
                Update,
                update_selection
                    .after(update_cursor_info)
                    .run_if(resource_equals(EguiFocused(false)))
                    .run_if(resource_equals(Mode::Edit)),
            )
            .add_systems(
                Update,
                move_selected
                    .after(update_selection)
                    .run_if(resource_equals(EguiFocused(false)))
                    .run_if(resource_equals(Mode::Edit)),
            )
            .add_systems(
                Update,
                update_indicator.run_if(resource_equals(EguiFocused(false))),
            )
            .add_systems(Update, highlight_selected)
            .add_systems(
                Update,
                delete_selected.run_if(resource_equals(EguiFocused(false))),
            )
            .add_systems(Update, follow_object)
            .add_systems(
                Update,
                copy_selection.run_if(resource_equals(EguiFocused(false))),
            );
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Resource)]
pub enum Mode {
    #[default]
    Edit,
    Draw,
    Joint,
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct DrawSettings {
    pub sides: u32,
    pub color: [u8; 4],
    pub rigid_body: RigidBody,
    pub collision_layer: u32,
    pub sensor: bool,
    pub links: String,
    pub code: (String, String),
    pub custom_mass: bool,
    pub mass: f32,
    pub custom_inertia: bool,
    pub inertia: f32,
    pub center_of_mass: Vec2,
    pub restitution: f32,
    pub lin_damp: f32,
    pub ang_damp: f32,
    pub friction: f32,
    pub tail: usize,
}

impl Default for DrawSettings {
    fn default() -> Self {
        DrawSettings {
            sides: 32,
            color: [255, 172, 171, 255],
            rigid_body: RigidBody::Dynamic,
            collision_layer: 0,
            sensor: false,
            links: String::new(),
            code: (String::new(), String::new()),
            custom_mass: false,
            mass: 1000.,
            custom_inertia: false,
            inertia: 1000.,
            center_of_mass: Vec2::ZERO,
            restitution: 0.5,
            lin_damp: 0.,
            ang_damp: 0.,
            friction: 0.5,
            tail: 90,
        }
    }
}

#[derive(PartialEq, Debug, Reflect)]
pub enum JointType {
    Fixed,
    Distance,
    Prismatic,
    Revolute,
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct JointSettings {
    pub joint_type: JointType,
    pub compliance: f32,
    pub custom_anchors: bool,
    pub local_anchor_1: Vec2,
    pub local_anchor_2: Vec2,
    pub dist_limits: (f32, f32),
    pub dist_rest: f32,
    pub prismatic_axis: Vec2,
    pub prismatic_limits: (f32, f32),
    pub angle_limits: (f32, f32),
}

impl Default for JointSettings {
    fn default() -> Self {
        JointSettings {
            joint_type: JointType::Distance,
            compliance: 0.00000001,
            custom_anchors: false,
            local_anchor_1: Vec2::ZERO,
            local_anchor_2: Vec2::ZERO,
            dist_limits: (0., 200.),
            dist_rest: 150.,
            prismatic_axis: Vec2::ONE,
            prismatic_limits: (100., 500.),
            angle_limits: (-TAU, TAU),
        }
    }
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct ClickedOnSpace(bool);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Selected;

// initial, final, delta
#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
pub struct CursorInfo {
    pub i: Vec2,
    pub f: Vec2,
    pub d: Vec2,
}

fn switch_modes(keyboard_input: Res<ButtonInput<KeyCode>>, mut mode: ResMut<Mode>) {
    if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        if keyboard_input.just_pressed(KeyCode::Digit1) {
            *mode = Mode::Edit;
        } else if keyboard_input.just_pressed(KeyCode::Digit2) {
            *mode = Mode::Draw;
        } else if keyboard_input.just_pressed(KeyCode::Digit3) {
            *mode = Mode::Joint;
        }
    }
}

pub fn update_cursor_info(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut cursor: ResMut<CursorInfo>,
    mut last_pos: Local<Vec2>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let (cam, cam_transform) = camera_query.single().unwrap();
        if let Some(cursor_pos) = windows.single().unwrap().cursor_position() {
            if let Ok(point) = cam.viewport_to_world_2d(cam_transform, cursor_pos) {
                cursor.i = point;
            }
        }
    }
    if mouse_button_input.pressed(MouseButton::Left) {
        let (cam, cam_transform) = camera_query.single().unwrap();
        if let Some(cursor_pos) = windows.single().unwrap().cursor_position() {
            if let Ok(point) = cam.viewport_to_world_2d(cam_transform, cursor_pos) {
                cursor.f = point;
                cursor.d = point - *last_pos;
                *last_pos = point;
            }
        }
    }
    if mouse_button_input.just_released(MouseButton::Left) {
        cursor.d = Vec2::ZERO;
        //*last_pos = -cursor.f;
    }
}

fn update_indicator(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    cursor: Res<CursorInfo>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    clicked_on_space: Res<ClickedOnSpace>,
    mut gizmos: Gizmos,
    settings: Res<DrawSettings>,
    mode: Res<Mode>,
) {
    if mouse_button_input.pressed(MouseButton::Left)
        && !mouse_button_input.just_pressed(MouseButton::Left)
        && !keyboard_input.pressed(KeyCode::Space)
    {
        match *mode {
            Mode::Draw => {
                let iso = Isometry2d::from_translation(cursor.i);
                let rad = cursor.i.distance(cursor.f);
                gizmos
                    .circle_2d(iso, rad, Color::WHITE)
                    .resolution(settings.sides);
            }
            Mode::Edit if clicked_on_space.0 => {
                let iso = Isometry2d::from_translation((cursor.i + cursor.f) / 2.);
                let size = (cursor.f - cursor.i).abs();
                if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
                    gizmos.rect_2d(iso, size, RED_500);
                } else if keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
                    gizmos.rect_2d(iso, size, GREEN_500);
                } else {
                    gizmos.rect_2d(iso, size, GRAY_50);
                };
            }
            Mode::Joint => gizmos.line_2d(cursor.i, cursor.f, Color::WHITE),
            _ => {}
        }
    }
}

fn highlight_selected(selected_query: Query<&Transform, With<Selected>>, mut gizmos: Gizmos) {
    for t in selected_query.iter() {
        let center = t.translation.xy();
        let xy = t.scale.xy();
        let xyp = xy.perp();
        gizmos.ray_2d(center + xy, xy, Color::WHITE);
        gizmos.ray_2d(center - xy, -xy, Color::WHITE);
        gizmos.ray_2d(center + xyp, xyp, Color::WHITE);
        gizmos.ray_2d(center - xyp, -xyp, Color::WHITE);
    }
}

fn delete_selected(
    mut commands: Commands,
    selected_query: Query<Entity, With<Selected>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    fixed: Query<(Entity, &FixedJoint)>,
    distance: Query<(Entity, &DistanceJoint)>,
    revolute: Query<(Entity, &RevoluteJoint)>,
    prismatic: Query<(Entity, &PrismaticJoint)>,
) {
    if keyboard_input.just_pressed(KeyCode::Delete) {
        let shift = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        if shift {
            for (e, j) in fixed.iter() {
                if selected_query.contains(j.entity1) || selected_query.contains(j.entity2) {
                    commands.entity(e).despawn();
                }
            }
            for (e, j) in distance.iter() {
                if selected_query.contains(j.entity1) || selected_query.contains(j.entity2) {
                    commands.entity(e).despawn();
                }
            }
            for (e, j) in revolute.iter() {
                if selected_query.contains(j.entity1) || selected_query.contains(j.entity2) {
                    commands.entity(e).despawn();
                }
            }
            for (e, j) in prismatic.iter() {
                if selected_query.contains(j.entity1) || selected_query.contains(j.entity2) {
                    commands.entity(e).despawn();
                }
            }
        } else {
            for e in selected_query.iter() {
                commands.entity(e).despawn();
            }
        }
    }
}

fn select_all(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    all: Query<Entity, With<Mesh2d>>,
    mut commands: Commands,
) {
    let ctrl = keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if ctrl && keyboard_input.just_pressed(KeyCode::KeyA) {
        for e in all.iter() {
            commands.entity(e).insert(Selected);
        }
    }
}

fn update_selection(
    mut commands: Commands,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    trans_query: Query<&Transform>,
    collider_query: Query<&Collider>,
    visible: Query<&VisibleEntities>,
    selected: Query<Entity, With<Selected>>,
    cursor: Res<CursorInfo>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut clicked_entity: Local<Option<Entity>>,
    mut clicked_on_space: ResMut<ClickedOnSpace>,
) {
    if keyboard_input.pressed(KeyCode::Space) {
        return;
    }
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let shift = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let ctrl = keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        *clicked_entity = None;
        let mut depth = f32::NEG_INFINITY;
        for e in visible.single().unwrap().get(TypeId::of::<Mesh2d>()) {
            let t = trans_query.get(*e).unwrap();
            let collider = collider_query.get(*e).unwrap();
            if t.translation.z > depth
                && collider.contains_point(t.translation.xy(), t.rotation, cursor.i)
                && !t.translation.x.is_nan()
                && !t.translation.y.is_nan()
            {
                *clicked_entity = Some(*e);
                depth = t.translation.z
            }
        }
        if let Some(e) = *clicked_entity {
            clicked_on_space.0 = false;
            if !selected.contains(e) {
                if shift {
                    commands.entity(e).insert(Selected);
                } else {
                    for entity in selected.iter() {
                        commands.entity(entity).remove::<Selected>();
                    }
                    commands.entity(e).insert(Selected);
                }
            } else if ctrl {
                commands.entity(e).remove::<Selected>();
            }
        } else {
            clicked_on_space.0 = true;
        }
    } else if mouse_button_input.just_released(MouseButton::Left) && clicked_entity.is_none() {
        let shift = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let ctrl = keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        if !(shift || ctrl) {
            for entity in selected.iter() {
                commands.entity(entity).remove::<Selected>();
            }
        }
        let (min_x, max_x) = if cursor.i.x < cursor.f.x {
            (cursor.i.x, cursor.f.x)
        } else {
            (cursor.f.x, cursor.i.x)
        };
        let (min_y, max_y) = if cursor.i.y < cursor.f.y {
            (cursor.i.y, cursor.f.y)
        } else {
            (cursor.f.y, cursor.i.y)
        };
        for e in visible.single().unwrap().get(TypeId::of::<Mesh2d>()) {
            if let Ok(t) = trans_query.get(*e) {
                if (min_x < t.translation.x && t.translation.x < max_x)
                    && (min_y < t.translation.y && t.translation.y < max_y)
                {
                    if ctrl {
                        commands.entity(*e).remove::<Selected>();
                    } else {
                        commands.entity(*e).insert(Selected);
                    }
                }
            }
        }
    }
}

fn move_selected(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    cursor: Res<CursorInfo>,
    mut selected_query: Query<(&mut Transform, &mut LinearVelocity), With<Selected>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if mouse_button_input.pressed(MouseButton::Left)
        && !mouse_button_input.just_pressed(MouseButton::Left)
        && cursor.i.distance_squared(cursor.f) > 1.
        && !keyboard_input.pressed(KeyCode::Space)
        && !keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
        && !keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
    {
        for (mut t, mut v) in selected_query.iter_mut() {
            v.x = 0.;
            v.y = 0.;
            t.translation.x += cursor.d.x;
            t.translation.y += cursor.d.y;
        }
    }
}

fn toggle_pan(
    mut query: Query<&mut PanCam>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    egui_focused: Res<EguiFocused>,
) {
    query.single_mut().unwrap().enabled = keyboard_input.pressed(KeyCode::Space) && !egui_focused.0;
}

fn follow_object(
    selected_query: Query<&Transform, (With<Selected>, Without<Camera>)>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
) {
    if mouse_button_input.pressed(MouseButton::Right) {
        if let Ok(t) = selected_query.single() {
            let mut cam = camera_query.single_mut().unwrap();
            cam.translation.x = t.translation.x;
            cam.translation.y = t.translation.y;
        }
    }
}

fn copy_selection(
    mut contexts: EguiContexts,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    selected_query: Query<Entity, With<Selected>>,
    lapis: crate::lapis::Lapis,
    links_query: Query<&crate::objects::Links>,
    code_query: Query<&crate::objects::Code>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyC)
        && keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
    {
        let mut selection = String::new();
        for e in selected_query.iter() {
            let t = lapis.trans_query.get(e).unwrap();
            let (x, y, z) = (t.translation.x, t.translation.y, t.translation.z);
            let (rx, ry) = (t.scale.x, t.scale.y);
            let rot = t.rotation.to_euler(EulerRot::XYZ).2;
            let mass = lapis.mass_query.get(e).unwrap().0;
            let linv = lapis.lin_velocity_query.get(e).unwrap();
            let (vx, vy) = (linv.x, linv.y);
            let va = lapis.ang_velocity_query.get(e).unwrap().0;
            let restitution = lapis.restitution_query.get(e).unwrap().coefficient;
            let lindamp = lapis.lin_damp_query.get(e).unwrap().0;
            let angdamp = lapis.ang_damp_query.get(e).unwrap().0;
            let inertia = lapis.inertia_query.get(e).unwrap().0;
            let mat_id = lapis.material_ids.get(e).unwrap();
            let mat = lapis.materials.get(mat_id).unwrap();
            let hsla: Hsla = mat.color.into();
            let (h, s, l, a) = (hsla.hue, hsla.saturation, hsla.lightness, hsla.alpha);
            let sides = lapis.sides_query.get(e).unwrap().0;
            let cm = lapis.cm_query.get(e).unwrap();
            let (cmx, cmy) = (cm.x, cm.y);
            let friction = lapis.friction_query.get(e).unwrap().dynamic_coefficient;
            let tail = lapis.tail_query.get(e).unwrap().len;
            let layer = lapis.layer_query.get(e).unwrap().memberships.0.ilog2();
            let sensor = lapis.sensor_query.contains(e);
            let dynamic = *lapis.body_query.get(e).unwrap() == RigidBody::Dynamic;
            let links = &links_query.get(e).unwrap().0;
            let code = code_query.get(e).unwrap();
            let (ci, cf) = (&code.0, &code.1);
            let line = format!("let _ = spawn({rx}).x({x}).y({y}).z({z}).ry({ry}).rot({rot}).mass({mass}).inertia({inertia}).vx({vx}).vy({vy}).va({va}).restitution({restitution}).lindamp({lindamp}).angdamp({angdamp}).h({h}).s({s}).l({l}).a({a}).sides({sides}).cmx({cmx}).cmy({cmy}).friction({friction}).tail({tail}).layer({layer}).dynamic({dynamic}).sensor({sensor}).links(\"{links}\").code_i(\"{ci}\").code_f(\"{cf}\");\n");
            selection.push_str(&line);
        }
        for j in lapis.fixed_query.iter() {
            if selected_query.contains(j.entity1) && selected_query.contains(j.entity2) {
                let t1 = lapis.trans_query.get(j.entity1).unwrap().translation;
                let t2 = lapis.trans_query.get(j.entity2).unwrap().translation;
                let line = format!(
                    "let _ = joint({},{},{},{}).joint_type(0).compliance({}).anchor1({},{}).anchor2({},{});\n",
                    t1.x, t1.y, t2.x, t2.y, j.compliance,
                    j.local_anchor1.x, j.local_anchor1.y, j.local_anchor2.x, j.local_anchor2.y,
                );
                selection.push_str(&line);
            }
        }
        for j in lapis.distance_query.iter() {
            if selected_query.contains(j.entity1) && selected_query.contains(j.entity2) {
                let t1 = lapis.trans_query.get(j.entity1).unwrap().translation;
                let t2 = lapis.trans_query.get(j.entity2).unwrap().translation;
                let limits = j.length_limits.unwrap();
                let line = format!(
                    "let _ = joint({},{},{},{}).joint_type(1).limits({},{}).compliance({}).anchor1({},{}).anchor2({},{}).rest({});\n",
                    t1.x, t1.y, t2.x, t2.y, limits.min, limits.max, j.compliance,
                    j.local_anchor1.x, j.local_anchor1.y, j.local_anchor2.x, j.local_anchor2.y, j.rest_length,
                );
                selection.push_str(&line);
            }
        }
        for j in lapis.prismatic_query.iter() {
            if selected_query.contains(j.entity1) && selected_query.contains(j.entity2) {
                let t1 = lapis.trans_query.get(j.entity1).unwrap().translation;
                let t2 = lapis.trans_query.get(j.entity2).unwrap().translation;
                let limits = j.free_axis_limits.unwrap();
                let line = format!(
                    "let _ = joint({},{},{},{}).joint_type(2).limits({},{}).compliance({}).anchor1({},{}).anchor2({},{}).free_axis({},{});\n",
                    t1.x, t1.y, t2.x, t2.y, limits.min, limits.max, j.compliance,
                    j.local_anchor1.x, j.local_anchor1.y, j.local_anchor2.x, j.local_anchor2.y,
                    j.free_axis.x, j.free_axis.y,
                );
                selection.push_str(&line);
            }
        }
        for j in lapis.revolute_query.iter() {
            if selected_query.contains(j.entity1) && selected_query.contains(j.entity2) {
                let t1 = lapis.trans_query.get(j.entity1).unwrap().translation;
                let t2 = lapis.trans_query.get(j.entity2).unwrap().translation;
                let limits = j.angle_limit.unwrap();
                let line = format!(
                    "let _ = joint({},{},{},{}).joint_type(3).limits({},{}).compliance({}).anchor1({},{}).anchor2({},{});\n",
                    t1.x, t1.y, t2.x, t2.y, limits.min, limits.max, j.compliance,
                    j.local_anchor1.x, j.local_anchor1.y, j.local_anchor2.x, j.local_anchor2.y,
                );
                selection.push_str(&line);
            }
        }
        if !selection.is_empty() {
            if let Ok(ctx) = contexts.ctx_mut() {
                ctx.copy_text(selection);
            }
        }
    }
}

// this system was stolen from bevy_pancam (then refactored)
#[derive(Resource, Deref, DerefMut, PartialEq, Default)]
pub struct EguiFocused(pub bool);

fn check_egui_focus(mut contexts: EguiContexts, mut egui_focused: ResMut<EguiFocused>) {
    if let Ok(ctx) = contexts.ctx_mut() {
        let focused = ctx.wants_pointer_input() || ctx.wants_keyboard_input();
        egui_focused.set_if_neq(EguiFocused(focused));
    }
}
