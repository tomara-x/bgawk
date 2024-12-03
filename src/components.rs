use bevy::prelude::*;

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Code(pub String);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Links(pub String);
