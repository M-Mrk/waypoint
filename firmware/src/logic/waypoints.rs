use core::fmt::Write;

use heapless::{String, Vec};

use crate::data::waypoints as dwaypoints;

use super::{App, AppState};
use crate::{
    inputs::expander::ButtonType,
    outputs::display::{self, DISPLAY_STATE, DisplayState},
};

pub struct WaypointsState {
    current_selection: usize,
    result: Option<String<15>>,
    select_open: bool,
    select_selection: usize,
}

#[allow(unused)]
impl WaypointsState {
    pub fn new() -> Self {
        WaypointsState {
            current_selection: 0,
            result: None,
            select_open: false,
            select_selection: 0,
        }
    }

    pub async fn handle(&mut self, app: &mut App) {
        if app.inputs.button.is_none() {
            return;
        }

        match app.inputs.button.unwrap().function {
            ButtonType::Left => {
                if self.select_open {
                    self.prior_select_option().await;
                } else {
                    self.prior_main_option();
                }
            }
            ButtonType::Right => {
                if self.select_open {
                    self.next_select_option().await;
                } else {
                    self.next_main_option();
                }
            }
            ButtonType::Ok => {
                if self.select_open {
                    self.delete_selected_waypoint().await;
                } else {
                    match self.current_selection {
                        0 => self.create_waypoint_from_location(app).await,
                        1 => self.open_selection().await,
                        2 => {
                            app.set_state(AppState::Selecting);
                            return;
                        }
                        _ => {}
                    }
                }
            }
            ButtonType::Power => {
                app.set_state(AppState::Sleep);
                return;
            }
        }

        self.update_screen().await;
    }

    pub async fn initalize(&mut self, app: &mut App) {
        *self = Self::new(); // set default values
        self.update_screen().await;
    }

    pub async fn deintialize(&mut self, app: &mut App) {}

    async fn get_waypoint_names(
        &self,
    ) -> Vec<String<{ dwaypoints::MAX_NAME_LENGTH }>, { dwaypoints::MAX_WAYPOINTS }> {
        let all_waypoints = dwaypoints::get_all_waypoints().await;
        let mut waypoint_names = Vec::new();

        for waypoint in all_waypoints.iter() {
            if waypoint_names.push(waypoint.name.clone()).is_err() {
                break;
            }
        }

        waypoint_names
    }

    fn make_message(message: &str) -> String<{ dwaypoints::MAX_NAME_LENGTH }> {
        let mut text = String::new();
        let _ = text.write_str(message);
        text
    }

    fn prior_main_option(&mut self) {
        if self.current_selection == 0 {
            self.current_selection = 2;
            return;
        }

        self.current_selection = self.current_selection.saturating_sub(1);
    }

    fn next_main_option(&mut self) {
        if self.current_selection >= 2 {
            self.current_selection = 0;
            return;
        }

        self.current_selection += 1;
    }

    async fn open_selection(&mut self) {
        let waypoint_names = self.get_waypoint_names().await;
        if waypoint_names.is_empty() {
            self.result = Some(Self::make_message("No waypoints"));
            self.select_open = false;
            self.select_selection = 0;
            return;
        }

        self.select_open = true;
        self.select_selection = 1;
        self.result = Some(Self::make_message("Select waypoint"));
    }

    async fn create_waypoint_from_location(&mut self, app: &mut App) {
        let gnss = match app.inputs.gnss {
            Some(data) => data,
            None => {
                self.result = Some(Self::make_message("No GNSS fix"));
                return;
            }
        };

        let coordinate = match dwaypoints::Coordinate::from_gnss(&gnss) {
            Ok(coordinate) => coordinate,
            Err(_) => {
                self.result = Some(Self::make_message("No GNSS fix"));
                return;
            }
        };

        let waypoint_count = dwaypoints::get_num_waypoints().await;
        let mut name: String<{ dwaypoints::MAX_NAME_LENGTH }> = String::new();
        if write!(&mut name, "WP{}", waypoint_count + 1).is_err() {
            self.result = Some(Self::make_message("Name too long"));
            return;
        }

        let mut waypoints = dwaypoints::get_all_waypoints().await;
        if waypoints
            .push(dwaypoints::Waypoint {
                name: name.clone(),
                coordinate,
            })
            .is_err()
        {
            self.result = Some(Self::make_message("Waypoint full"));
            return;
        }

        dwaypoints::replace_waypoints(waypoints).await;
        self.result = Some(name);
    }

    async fn delete_selected_waypoint(&mut self) {
        let waypoint_names = self.get_waypoint_names().await;
        if waypoint_names.is_empty() {
            self.result = Some(Self::make_message("No waypoints"));
            self.select_open = false;
            self.select_selection = 0;
            return;
        }

        let selected_index = self
            .select_selection
            .saturating_sub(1)
            .min(waypoint_names.len() - 1);

        let mut waypoints = dwaypoints::get_all_waypoints().await;
        if selected_index >= waypoints.len() {
            self.result = Some(Self::make_message("Invalid item"));
            self.select_open = false;
            self.select_selection = 0;
            return;
        }

        let removed = waypoints.remove(selected_index);
        dwaypoints::replace_waypoints(waypoints).await;

        self.result = Some(Self::make_message("Deleted"));
        self.select_open = false;
        self.select_selection = 0;
    }

    async fn prior_select_option(&mut self) {
        let option_count = self.get_waypoint_names().await.len();
        if option_count == 0 {
            return;
        }

        if self.select_selection <= 1 {
            self.select_selection = option_count;
            return;
        }

        self.select_selection = self.select_selection.saturating_sub(1);
    }

    async fn next_select_option(&mut self) {
        let option_count = self.get_waypoint_names().await.len();
        if option_count == 0 {
            return;
        }

        if self.select_selection >= option_count {
            self.select_selection = 1;
            return;
        }

        self.select_selection += 1;
    }

    async fn update_screen(&mut self) {
        DISPLAY_STATE
            .sender()
            .send(DisplayState::Waypoints(display::waypoints::State {
                main_selection: self.current_selection,
                result_box: self.result.clone(),
                select_open: self.select_open,
                select_options: self.get_waypoint_names().await,
                select_selection: self.select_selection,
            }));
    }
}
