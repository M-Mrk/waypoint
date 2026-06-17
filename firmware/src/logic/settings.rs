use crate::outputs::display::{self, DISPLAY_STATE, DisplayState};

use super::{App, AppState};

pub struct SettingsState {}

#[allow(unused)]
impl SettingsState {
    pub fn new() -> Self {
        SettingsState {}
    }

    pub async fn handle(&mut self, app: &mut App) {
        if app.inputs.button.is_none() {
            return;
        }
        app.set_state(AppState::Selecting);
    }

    pub async fn initalize(&mut self, app: &mut App) {
        self.update_display().await;
    }

    pub async fn deintialize(&mut self, app: &mut App) {}

    async fn update_display(&self) {
        DISPLAY_STATE.sender().send(DisplayState::Settings(display::settings::State{}));
    }
}
