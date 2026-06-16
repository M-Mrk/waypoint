use super::{App, AppState};
use crate::inputs::expander::{ButtonAction, ButtonCall, ButtonType};

pub struct WaypointsState {}

#[allow(unused)]
impl WaypointsState {
    pub fn new() -> Self {
        WaypointsState {}
    }

    pub async fn handle(&mut self, app: &mut App) {
        app.set_state(AppState::Selecting);
    }

    pub async fn initalize(&mut self, app: &mut App) {}

    pub async fn deintialize(&mut self, app: &mut App) {}
}
