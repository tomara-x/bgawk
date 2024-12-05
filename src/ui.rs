use crate::{
    interaction::{DrawSettings, Mode, Selected},
    lapis::{Lapis, UpdateCode},
    objects::*,
};
use avian2d::prelude::*;
use bevy::{
    app::{App, Plugin, Update},
    prelude::{Query, ResMut, With},
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
    mut settings: ResMut<DrawSettings>,
    mut gravity: ResMut<Gravity>,
    mut selected: Query<(&mut Code, &mut Links), With<Selected>>,
    mut update_code: ResMut<UpdateCode>,
    mut mode: ResMut<Mode>,
) {
    let ctx = contexts.ctx_mut();
    let theme = CodeTheme::from_memory(ctx, &ctx.style());
    let mut layouter = |ui: &Ui, string: &str, wrap_width: f32| {
        let mut layout_job = highlight(ui.ctx(), ui.style(), &theme, string, "rs");
        layout_job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(layout_job))
    };
    Window::new("settings").show(ctx, |ui| {
        egui::ComboBox::from_label("mode")
            .selected_text(format!("{:?}", *mode))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut *mode, Mode::Edit, "Edit");
                ui.selectable_value(&mut *mode, Mode::Draw, "Draw");
                ui.selectable_value(&mut *mode, Mode::Joint, "Joint");
            });
        if *mode == Mode::Draw {
            ui.horizontal(|ui| {
                ui.add(DragValue::new(&mut settings.sides).range(3..=128));
                ui.label("sides");
            });
            ui.horizontal(|ui| {
                ui.color_edit_button_rgba_unmultiplied(&mut settings.color);
                ui.label("color");
            });
            egui::ComboBox::from_label("rigid body")
                .selected_text(format!("{:?}", settings.rigid_body))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut settings.rigid_body, RigidBody::Static, "Static");
                    ui.selectable_value(&mut settings.rigid_body, RigidBody::Dynamic, "Dynamic");
                });
            // TODO move to edit
            ui.horizontal(|ui| {
                ui.add(DragValue::new(&mut settings.collision_layer).range(0..=31));
                ui.label("collision layer");
            });
        } else if *mode == Mode::Edit {
            ui.horizontal(|ui| {
                ui.label("gravity");
                ui.add(DragValue::new(&mut gravity.0.x));
                ui.add(DragValue::new(&mut gravity.0.y));
            });
            ui.add(
                TextEdit::multiline(&mut update_code.0)
                    .hint_text("code here will be quietly evaluated every frame")
                    .font(TextStyle::Monospace)
                    .code_editor()
                    .lock_focus(true)
                    .layouter(&mut layouter),
            );
            lapis.quiet_eval(&update_code.0);
            ui.label("selected:");
            // TODO multiple selected entities?
            if let Ok((mut code, mut links)) = selected.get_single_mut() {
                ui.horizontal(|ui| {
                    ui.label("links");
                    ui.add(
                        TextEdit::multiline(&mut links.0)
                            .font(TextStyle::Monospace)
                            .code_editor()
                            .desired_rows(1)
                            .lock_focus(true)
                            .layouter(&mut layouter),
                    )
                    .on_hover_text(
                        "link a property of this entity to a shared var\n\n\
                    every line should follow the form:\n\
                    property > variable\n\
                    to set the variable to the property's value\n\
                    or\n\
                    property < variable\n\
                    to set the property to the variable's value\n\n\
                    properties list:\n\
                    x\n\
                    y\n\
                    rx (x radius)\n\
                    ry\n\
                    rot (rotation)\n\
                    mass\n\
                    vx (x velocity)\n\
                    vy\n\
                    va (angular velocity)\n\
                    restitution\n\
                    lindamp (linear damping)\n\
                    angdamp (angular damping)\n\
                    inertia\n\
                    h (hue)\n\
                    s (saturation)\n\
                    l (lightness)\n\
                    a (alpha)\n\
                    sides",
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("code");
                    ui.add(
                        TextEdit::multiline(&mut code.0)
                            .font(TextStyle::Monospace)
                            .code_editor()
                            .desired_rows(1)
                            .lock_focus(true)
                            .layouter(&mut layouter),
                    )
                    .on_hover_text(
                        "code that will execute on collision\n\
                    these placeholders will be substituted:\n\
                    $x for this entity's x position\n\
                    $y for y position\n\
                    $rx for x radius\n\
                    $ry for y radius\n\
                    $rot for rotation\n\
                    $vx for x velocity\n\
                    $vy for y velocity\n\
                    $va for angular velocity\n\
                    $mass for.. well the mass\n\
                    $inertia for angular inertia",
                    );
                });
            }
        } else if *mode == Mode::Joint {
        }
    });
    Window::new("lapis output")
        // TODO why pivot doesn't work?
        .default_pos(Pos2::new(1000., 0.))
        .show(ctx, |ui| {
            ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut lapis.buffer)
                        .font(TextStyle::Monospace)
                        .code_editor()
                        .desired_rows(1)
                        .lock_focus(true)
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
                                    .font(TextStyle::Monospace)
                                    .code_editor()
                                    .desired_rows(5)
                                    .lock_focus(true)
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
