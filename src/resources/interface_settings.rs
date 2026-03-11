use bevy::prelude::{Resource};
use serde::Deserialize;

#[derive(Resource)]
pub struct InterfaceSettings {
    pub targeting: TargetingType,
}

#[derive(Deserialize, Copy, Clone, PartialEq)]
pub enum TargetingType {
    DoubleClick,
    SingleClick,
}
