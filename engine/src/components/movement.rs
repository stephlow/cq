use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum MoveModifier {
    StartForward,
    StopForward,
    StartBackward,
    StopBackward,
    StartRight,
    StopRight,
    StartLeft,
    StopLeft,
}

#[derive(Default, Component)]
pub struct Movement {
    pub forward: bool,
    pub backward: bool,
    pub right: bool,
    pub left: bool,
}

impl Movement {
    pub fn modify(&mut self, modifier: MoveModifier) {
        match modifier {
            MoveModifier::StartForward => self.forward = true,
            MoveModifier::StopForward => self.forward = false,
            MoveModifier::StartBackward => self.backward = true,
            MoveModifier::StopBackward => self.backward = false,
            MoveModifier::StartRight => self.right = true,
            MoveModifier::StopRight => self.right = false,
            MoveModifier::StartLeft => self.left = true,
            MoveModifier::StopLeft => self.left = false,
        }
    }
}
