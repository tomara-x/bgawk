use crate::{interaction::*, lapis::*, objects::*};
use avian2d::prelude::*;
use bevy::{
    app::{App, Plugin, Update},
    prelude::{GizmoConfigStore, Query, Res, ResMut, Resource, Time, Virtual, With},
};
use bevy_egui::{EguiContexts, EguiPlugin};
use egui::*;
use egui_extras::syntax_highlighting::*;
use std::sync::Arc;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .init_resource::<InsertComponents>()
            .insert_resource(ZoomFactor(1.))
            .add_systems(Update, egui_ui)
            .add_systems(Update, update_scale_factor);
    }
}

#[derive(Resource, Default)]
struct InsertComponents {
    links: String,
    code: (String, String),
}

#[derive(Resource)]
struct ZoomFactor(f32);

fn update_scale_factor(
    mut win: Query<&mut bevy::prelude::Window>,
    zoom_factor: Res<ZoomFactor>,
) {
    win.single_mut().resolution.set_scale_factor(zoom_factor.0);
}

fn egui_ui(
    mut contexts: EguiContexts,
    mut lapis: ResMut<Lapis>,
    mut draw: ResMut<DrawSettings>,
    mut gravity: ResMut<Gravity>,
    mut selected: Query<(&mut Code, &mut Links), With<Selected>>,
    mut update_code: ResMut<UpdateCode>,
    mut mode: ResMut<Mode>,
    mut attraction_factor: ResMut<AttractionFactor>,
    mut joint: ResMut<JointSettings>,
    mut time: ResMut<Time<Virtual>>,
    mut quiet: ResMut<QuietCollisionEval>,
    mut insert: ResMut<InsertComponents>,
    cursor: Res<CursorInfo>,
    mut config_store: ResMut<GizmoConfigStore>,
    mut zoom_factor: ResMut<ZoomFactor>,
) {
    let ctx = contexts.ctx_mut();
    let theme = CodeTheme::from_memory(ctx, &ctx.style());
    let mut layouter = |ui: &Ui, string: &str, wrap_width: f32| {
        let mut layout_job = highlight(ui.ctx(), ui.style(), &theme, string, "rs");
        layout_job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(layout_job))
    };
    Window::new("mode").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut *mode, Mode::Edit, "Edit")
                .on_hover_text("ctrl+1");
            ui.selectable_value(&mut *mode, Mode::Draw, "Draw")
                .on_hover_text("ctrl+2");
            ui.selectable_value(&mut *mode, Mode::Joint, "Joint")
                .on_hover_text("ctrl+3");
        });
        ui.separator();
        if *mode == Mode::Draw {
            ui.horizontal(|ui| {
                ui.label("rigid body");
                ui.selectable_value(&mut draw.rigid_body, RigidBody::Static, "Static");
                ui.selectable_value(&mut draw.rigid_body, RigidBody::Dynamic, "Dynamic");
            });
            // TODO move to edit?
            ui.horizontal(|ui| {
                ui.label("collision layer");
                ui.add(DragValue::new(&mut draw.collision_layer).range(0..=31));
            });
            ui.horizontal(|ui| {
                ui.label("sides");
                ui.add(DragValue::new(&mut draw.sides).range(3..=512));
            });
            ui.horizontal(|ui| {
                ui.label("color");
                ui.color_edit_button_srgba_unmultiplied(&mut draw.color);
            });
            ui.horizontal(|ui| {
                ui.toggle_value(&mut draw.custom_mass, "custom mass?")
                    .on_hover_text("if not selected, mass = radius ^ 3");
                ui.add_enabled(draw.custom_mass, DragValue::new(&mut draw.mass));
            });
            ui.horizontal(|ui| {
                ui.toggle_value(&mut draw.custom_inertia, "custom inertia?")
                    .on_hover_text("if not selected, inertia = radius ^ 3");
                ui.add_enabled(draw.custom_inertia, DragValue::new(&mut draw.inertia));
            });
            ui.horizontal(|ui| {
                ui.label("center of mass");
                ui.add(DragValue::new(&mut draw.center_of_mass.x));
                ui.add(DragValue::new(&mut draw.center_of_mass.y));
            });
            ui.horizontal(|ui| {
                ui.label("friction");
                ui.add(DragValue::new(&mut draw.friction).speed(0.01));
            });
            ui.horizontal(|ui| {
                ui.label("restitution");
                ui.add(DragValue::new(&mut draw.restitution).speed(0.01));
            });
            ui.toggle_value(&mut draw.sensor, "sensor?")
                .on_hover_text("allows other bodies to pass through");
            ui.horizontal(|ui| {
                ui.label("linear damping");
                ui.add(DragValue::new(&mut draw.lin_damp).speed(0.01));
            });
            ui.horizontal(|ui| {
                ui.label("angular damping");
                ui.add(DragValue::new(&mut draw.ang_damp).speed(0.01));
            });
            links_line(ui, &mut draw.links);
            code_line(ui, &mut draw.code.0, &mut layouter, "on collision start");
            code_line(ui, &mut draw.code.1, &mut layouter, "on collision end");
        } else if *mode == Mode::Edit {
            if time.is_paused() {
                if ui.button("resume").clicked() {
                    time.unpause();
                }
            } else if ui.button("pause").clicked() {
                time.pause();
            }
            ui.horizontal(|ui| {
                ui.label("gravity");
                ui.add(DragValue::new(&mut gravity.0.x));
                ui.add(DragValue::new(&mut gravity.0.y));
            });
            ui.horizontal(|ui| {
                ui.label("attraction");
                ui.add(DragValue::new(&mut attraction_factor.0).speed(0.01));
            });
            ui.separator();
            let n = selected.iter().len();
            ui.label(format!("selected: {}", n));
            match n {
                0 => {}
                1 => {
                    let (mut code, mut links) = selected.single_mut();
                    links_line(ui, &mut links.0);
                    code_line(ui, &mut code.0, &mut layouter, "on collision start");
                    code_line(ui, &mut code.1, &mut layouter, "on collision end");
                }
                _ => {
                    links_line(ui, &mut insert.links);
                    code_line(ui, &mut insert.code.0, &mut layouter, "on collision start");
                    code_line(ui, &mut insert.code.1, &mut layouter, "on collision end");
                    if ui.button("apply to selected").clicked() {
                        for (mut code, mut links) in selected.iter_mut() {
                            code.0 = insert.code.0.clone();
                            code.1 = insert.code.1.clone();
                            links.0 = insert.links.clone();
                        }
                    }
                }
            }
        } else if *mode == Mode::Joint {
            ui.horizontal(|ui| {
                ui.label("joint type");
                ui.selectable_value(&mut joint.joint_type, JointType::Distance, "Distance");
                ui.selectable_value(&mut joint.joint_type, JointType::Prismatic, "Prismatic");
                ui.selectable_value(&mut joint.joint_type, JointType::Revolute, "Revolute");
                ui.selectable_value(&mut joint.joint_type, JointType::Fixed, "Fixed");
            });
            ui.horizontal(|ui| {
                ui.label("compliance");
                ui.add(
                    DragValue::new(&mut joint.compliance)
                        .range(0.0..=f32::INFINITY)
                        .speed(0.00001),
                );
            });
            ui.horizontal(|ui| {
                ui.label("local anchor 1");
                ui.add(DragValue::new(&mut joint.local_anchor_1.x).speed(0.01));
                ui.add(DragValue::new(&mut joint.local_anchor_1.y).speed(0.01));
            });
            ui.horizontal(|ui| {
                ui.label("local anchor 2");
                ui.add(DragValue::new(&mut joint.local_anchor_2.x).speed(0.01));
                ui.add(DragValue::new(&mut joint.local_anchor_2.y).speed(0.01));
            });
            match joint.joint_type {
                JointType::Distance => {
                    ui.horizontal(|ui| {
                        ui.label("limits");
                        ui.add(
                            DragValue::new(&mut joint.dist_limits.0)
                                .range(0.0..=f32::INFINITY)
                                .speed(0.01),
                        );
                        ui.add(
                            DragValue::new(&mut joint.dist_limits.1)
                                .range(0.0..=f32::INFINITY)
                                .speed(0.01),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("rest length");
                        ui.add(
                            DragValue::new(&mut joint.dist_rest)
                                .range(0.0..=f32::INFINITY)
                                .speed(0.01),
                        );
                    });
                }
                JointType::Prismatic => {
                    ui.horizontal(|ui| {
                        ui.label("limits");
                        ui.add(DragValue::new(&mut joint.prismatic_limits.0).speed(0.01));
                        ui.add(DragValue::new(&mut joint.prismatic_limits.1).speed(0.01));
                    });
                    ui.horizontal(|ui| {
                        ui.label("free axis");
                        ui.add(DragValue::new(&mut joint.prismatic_axis.x).speed(0.01));
                        ui.add(DragValue::new(&mut joint.prismatic_axis.y).speed(0.01));
                    });
                }
                JointType::Revolute => {
                    ui.horizontal(|ui| {
                        ui.label("limits");
                        ui.add(DragValue::new(&mut joint.angle_limits.0).speed(0.01));
                        ui.add(DragValue::new(&mut joint.angle_limits.1).speed(0.01));
                    });
                }
                _ => {}
            }
        }
    });
    Window::new("lapis output")
        // TODO why pivot doesn't work?
        .default_pos([900., 10.])
        .show(ctx, |ui| {
            ui.toggle_value(&mut quiet.0, "quiet?")
                .on_hover_text("don't log collision evaluation");
            ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut lapis.buffer)
                        .code_editor()
                        .desired_rows(1)
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter),
                );
            });
        });
    Window::new("lapis input")
        .default_pos([900., 560.])
        .show(ctx, |ui| {
            ui.collapsing("update code", |ui| {
                ui.add(
                    TextEdit::multiline(&mut update_code.0)
                        .hint_text("code here is quietly evaluated every frame")
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter),
                );
            });
            lapis.quiet_eval(&update_code.0);
            ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                        let execute = ui.button("e");
                        let input_focused = ui
                            .add(
                                TextEdit::multiline(&mut lapis.input)
                                    .hint_text("type code then press ctrl+enter")
                                    .code_editor()
                                    .desired_rows(5)
                                    .desired_width(f32::INFINITY)
                                    .layouter(&mut layouter),
                            )
                            .has_focus();
                        let shortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Enter);
                        if input_focused && ctx.input_mut(|i| i.consume_shortcut(&shortcut))
                            || execute.clicked()
                        {
                            lapis.eval_input();
                        }
                    });
                });
            });
        });
    Window::new("info")
        .default_open(false)
        .default_pos([10., 580.])
        .show(ctx, |ui| {
            let (conf, _) = config_store.config_mut::<PhysicsGizmos>();
            ui.toggle_value(&mut conf.enabled, "debug");
            ui.label(format!("i: ({}, {})", cursor.i.x, cursor.i.y));
            ui.label(format!("f: ({}, {})", cursor.f.x, cursor.f.y));
            ui.label(format!("distance: {}", cursor.i.distance(cursor.f)));
            ui.horizontal(|ui| {
                ui.label("zoom factor");
                ui.add(
                    DragValue::new(&mut zoom_factor.0)
                        .range(0.5..=4.)
                        .speed(0.1),
                );
            });
        });
}

fn links_line(ui: &mut Ui, buffer: &mut String) {
    ui.horizontal(|ui| {
        ui.label("links");
        ui.add(
            TextEdit::multiline(buffer)
                .code_editor()
                .desired_rows(1)
                .desired_width(f32::INFINITY),
        )
        .on_hover_text(LINKS_TOOLTIP);
    });
}

fn code_line(
    ui: &mut Ui,
    buffer: &mut String,
    layouter: &mut dyn FnMut(&Ui, &str, f32) -> Arc<Galley>,
    hint: &str,
) {
    ui.horizontal(|ui| {
        ui.label("code");
        ui.add(
            TextEdit::multiline(buffer)
                .hint_text(hint)
                .code_editor()
                .desired_rows(1)
                .desired_width(f32::INFINITY)
                .layouter(layouter),
        )
        .on_hover_text(CODE_TOOLTIP);
    });
}

const LINKS_TOOLTIP: &str = "link a property of this entity to a shared var\n
every line should follow the form:
property > variable
to set the variable to the property's value
or
property < variable
to set the property to the variable's value\n
properties list:
x
y
rx (x radius)
ry
rot (rotation)
mass
vx (x velocity)
vy
va (angular velocity)
restitution
lindamp (linear damping)
angdamp (angular damping)
inertia
h (hue)
s (saturation)
l (lightness)
a (alpha)
sides
cmx (center of mass x)
cmy (center of mass y)
friction";

const CODE_TOOLTIP: &str = "evaluated when this object starts/stops colliding with another\n
these placeholders will be substituted:
$x for this object's x position
$y for y position
$rx for x radius
$ry for y radius
$rot for rotation
$vx for x velocity
$vy for y velocity
$va for angular velocity
$mass for.. well, the mass
$inertia for angular inertia";
