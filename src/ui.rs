use crate::{interaction::*, lapis::*, objects::*};
use avian2d::prelude::*;
use bevy::{
    app::{App, Plugin, Update},
    prelude::{Query, ResMut, Time, Virtual, With},
};
use bevy_egui::{EguiContexts, EguiPlugin};
use egui::*;
use egui_extras::syntax_highlighting::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin).add_systems(Update, egui_ui);
    }
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
) {
    let ctx = contexts.ctx_mut();
    let theme = CodeTheme::from_memory(ctx, &ctx.style());
    let mut layouter = |ui: &Ui, string: &str, wrap_width: f32| {
        let mut layout_job = highlight(ui.ctx(), ui.style(), &theme, string, "rs");
        layout_job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(layout_job))
    };
    Window::new("settings").show(ctx, |ui| {
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
                ui.add(DragValue::new(&mut draw.sides).range(3..=128));
            });
            ui.horizontal(|ui| {
                ui.label("color");
                ui.color_edit_button_srgba_unmultiplied(&mut draw.color);
            });
            ui.horizontal(|ui| {
                ui.toggle_value(&mut draw.custom_mass, "custom mass?")
                    .on_hover_text("if not selected, mass = radius * 100");
                ui.add_enabled(draw.custom_mass, DragValue::new(&mut draw.mass));
            });
            ui.horizontal(|ui| {
                ui.toggle_value(&mut draw.custom_inertia, "custom inertia?")
                    .on_hover_text("if not selected, inertia = radius * 100");
                ui.add_enabled(draw.custom_inertia, DragValue::new(&mut draw.inertia));
            });
            ui.horizontal(|ui| {
                ui.label("center of mass");
                ui.add(DragValue::new(&mut draw.center_of_mass.x));
                ui.add(DragValue::new(&mut draw.center_of_mass.y));
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
            ui.horizontal(|ui| {
                ui.label("links");
                ui.add(
                    TextEdit::multiline(&mut draw.links)
                        .code_editor()
                        .desired_rows(1)
                        .desired_width(f32::INFINITY),
                )
                .on_hover_text(LINKS_TOOLTIP);
            });
            ui.horizontal(|ui| {
                ui.label("code");
                ui.add(
                    TextEdit::multiline(&mut draw.code)
                        .code_editor()
                        .desired_rows(1)
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter),
                )
                .on_hover_text(CODE_TOOLTIP);
            });
        } else if *mode == Mode::Edit {
            ui.horizontal(|ui| {
                ui.label("gravity");
                ui.add(DragValue::new(&mut gravity.0.x));
                ui.add(DragValue::new(&mut gravity.0.y));
            });
            ui.horizontal(|ui| {
                ui.label("attraction");
                ui.add(DragValue::new(&mut attraction_factor.0).speed(0.01));
            });
            ui.add(
                TextEdit::multiline(&mut update_code.0)
                    .hint_text("code here will be quietly evaluated every frame")
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter),
            );
            lapis.quiet_eval(&update_code.0);
            if time.is_paused() {
                if ui.button("resume").clicked() {
                    time.unpause();
                }
            } else if ui.button("pause").clicked() {
                time.pause();
            }
            ui.label("selected:");
            // TODO multiple selected entities?
            if let Ok((mut code, mut links)) = selected.get_single_mut() {
                ui.horizontal(|ui| {
                    ui.label("links");
                    ui.add(
                        TextEdit::multiline(&mut links.0)
                            .code_editor()
                            .desired_rows(1)
                            .desired_width(f32::INFINITY),
                    )
                    .on_hover_text(LINKS_TOOLTIP);
                });
                ui.horizontal(|ui| {
                    ui.label("code");
                    ui.add(
                        TextEdit::multiline(&mut code.0)
                            .code_editor()
                            .desired_rows(1)
                            .desired_width(f32::INFINITY)
                            .layouter(&mut layouter),
                    )
                    .on_hover_text(CODE_TOOLTIP);
                });
            }
        } else if *mode == Mode::Joint {
            ui.horizontal(|ui| {
                ui.label("joint type");
                ui.selectable_value(&mut joint.joint_type, JointType::Fixed, "Fixed");
                ui.selectable_value(&mut joint.joint_type, JointType::Distance, "Distance");
                ui.selectable_value(&mut joint.joint_type, JointType::Prismatic, "Prismatic");
                ui.selectable_value(&mut joint.joint_type, JointType::Revolute, "Revolute");
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
        .default_pos(Pos2::new(1000., 0.))
        .show(ctx, |ui| {
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
        .default_pos(Pos2::new(1000., 1000.))
        .show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                        let execute = ui.button("e");
                        let input_focused = ui
                            .add(
                                TextEdit::multiline(&mut lapis.input)
                                    .hint_text("type a statement then press ctrl+enter")
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
                            let input = std::mem::take(&mut lapis.input);
                            lapis.eval(&input);
                        }
                    });
                });
            });
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
cmy (center of mass y)";

const CODE_TOOLTIP: &str = "code that will execute on collision
these placeholders will be substituted:
$x for this entity's x position
$y for y position
$rx for x radius
$ry for y radius
$rot for rotation
$vx for x velocity
$vy for y velocity
$va for angular velocity
$mass for.. well, the mass
$inertia for angular inertia";
