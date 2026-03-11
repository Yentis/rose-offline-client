use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct InterfaceConfig {
    pub targeting: TargetingType,
}

impl Default for InterfaceConfig {
    fn default() -> Self {
        Self {
            targeting: TargetingType::DoubleClick,
        }
    }
}

#[derive(Deserialize, Serialize, Copy, Clone, PartialEq)]
pub enum TargetingType {
    DoubleClick,
    SingleClick,
}
