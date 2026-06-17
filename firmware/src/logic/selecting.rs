use super::{App, AppState};
use crate::outputs::display;
use crate::outputs::display::selecting::Item;
use crate::{
    inputs::expander::ButtonType,
    outputs::display::{DISPLAY_STATE, DisplayState},
    power::BATTERY_WATCH,
};
use defmt::error;
pub struct SelectingState {
    current_index: usize,
    all_items: [Item; 3],
}

impl SelectingState {
    pub fn new() -> Self {
        SelectingState {
            current_index: 0,
            all_items: [Item::Navigation, Item::Settings, Item::Waypoints],
        }
    }

    fn prior_item(&mut self) {
        if self.current_index == 0 {
            self.current_index = self.all_items.len() - 1;
            return;
        }
        self.current_index = self.current_index.saturating_sub(1);
    }

    fn next_item(&mut self) {
        if self.current_index + 1 >= self.all_items.len() {
            self.current_index = 0;
            return;
        }
        self.current_index += 1;
    }

    fn get_item(&self) -> &Item {
        let i = self.current_index.clamp(0, self.all_items.len() - 1);
        return &self.all_items[i];
    }

    fn enter_item(&self, app: &mut App) {
        let new_state = match *self.get_item() {
            Item::Navigation => AppState::Navigating,
            Item::Settings => AppState::Settings,
            Item::Waypoints => AppState::Waypoints,
        };
        app.set_state(new_state);
    }

    pub async fn handle(&mut self, app: &mut App) {
        if app.inputs.button.is_none() {
            // Only act on button press
            return;
        }

        match app.inputs.button.unwrap().function {
            ButtonType::Left => self.prior_item(),
            ButtonType::Right => self.next_item(),
            ButtonType::Ok => {
                self.enter_item(app);
                return;
            }
            ButtonType::Power => {
                app.set_state(AppState::Sleep);
                return;
            }
        }

        self.update_screen().await;
    }

    #[allow(unused)]
    pub async fn initalize(&mut self, app: &mut App) {
        self.update_screen().await;
    }

    #[allow(unused)]
    pub async fn deintialize(&mut self, app: &mut App) {} // Nothing to do

    async fn update_screen(&self) {
        let display = DISPLAY_STATE.sender();
        display.send(DisplayState::MainPage(display::selecting::State {
            battery: {
                match BATTERY_WATCH.receiver() {
                    None => {
                        error!("Failed to update screen: battery watch unavailable");
                        return;
                    }
                    Some(mut receiver) => receiver.get().await,
                }
            },

            current_item: self.get_item().clone(),
        }));
    }
}
