use crate::inputs::expander::ButtonType;
use crate::outputs::display::{DISPLAY_STATE, DisplayState};
use crate::power::{POWER_CMD, PowerCommand, PowerDomain};

use super::App;

pub struct SleepState {}

#[allow(unused)]
impl SleepState {
    pub fn new() -> Self {
        SleepState {}
    }

    pub async fn handle(&mut self, app: &mut App) {
        if app.inputs.button.is_none() {
            // Only act on button press
            return;
        }

        match app.inputs.button.unwrap().function {
            ButtonType::Power => app.set_state(app.last),
            _ => {}
        }
    }

    pub async fn initalize(&mut self, app: &mut App) {
        let power = POWER_CMD.sender();
        power.send(PowerCommand::Sleep).await;

        let display = DISPLAY_STATE.sender();
        display.send(DisplayState::Off);
    }

    pub async fn deintialize(&mut self, app: &mut App) {
        let power = POWER_CMD.sender();
        power.send(PowerCommand::Enable(PowerDomain::Gnss)).await;

        // No need to wake display, is handled automatically
    }
}
