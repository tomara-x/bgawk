//use avian2d::prelude::*;
use bevy::prelude::*;

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Code(pub String);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Links(pub String);

//LinkY(String)
//LinkH(String)
//LinkS(String)
//LinkL(String)
//LinkA(String)
//LinkRadius(String)
//LinkSides(String)
//LinkRotation(String)
//LinkMass(String)
//LinkVelX(String)
