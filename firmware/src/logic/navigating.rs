use defmt::{error, warn};
use libm::roundf;
use smart_leds::RGB8;

use crate::inputs::gnss::GNSS_WATCH;
use crate::inputs::sensors::{IMU_WATCH, MAG_WATCH};
use crate::navigation::{WaypointDifference, calculate_waypoint_data, calculate_current_bearing};
use crate::outputs::display;
use crate::outputs::leds::{LED_CMD, LedCommands};
use crate::{
    inputs::expander::ButtonType,
    outputs::display::{DISPLAY_STATE},
    power::BATTERY_WATCH,
};

use crate::data::waypoints::{Waypoint, get_num_waypoints, get_waypoint_at_index};

use super::{App, AppState};

pub struct NavigatingState {
    current_index: usize,
}

#[allow(unused)]
impl NavigatingState {
    pub fn new() -> Self {
        NavigatingState {
            current_index: 0,
        }
    }

    async fn next_waypoint(&mut self) {
        if self.current_index+1 >= get_num_waypoints().await{
           self.current_index = 0;
           return; 
        }
        self.current_index += 1;
    }

    async fn prior_waypoint(&mut self) {
        if self.current_index == 0 {
            self.current_index = get_num_waypoints().await - 1;
            return;
        }
        self.current_index = self.current_index.saturating_sub(1);
    }

    async fn get_waypoint(&self) -> Option<Waypoint> {
        let i = self.current_index.clamp(0, get_num_waypoints().await);
        return get_waypoint_at_index(i).await;
    }

    pub async fn handle(&mut self, app: &mut App) {
        let inputs = &app.inputs;
        if inputs.gnss.is_some() || inputs.imu.is_some() || inputs.magn.is_some() {
            self.update_all().await;
            return;
        }

        if inputs.button.is_none() {
            warn!("No update given");
            return;
        }
        match inputs.button.unwrap().function {
            ButtonType::Left => self.prior_waypoint().await,
            ButtonType::Ok => app.set_state(AppState::Selecting),
            ButtonType::Right => self.next_waypoint().await,
            ButtonType::Power => app.set_state(AppState::Sleep),
        }
        self.update_all().await;
    }

    pub async fn initalize(&mut self, app: &mut App) {
        self.update_all().await;
    }

    pub async fn deintialize(&mut self, app: &mut App) {}

    async fn update_display(&self, diff: &WaypointDifference) {
        let cur_way = self.get_waypoint().await;
        if cur_way.is_none() {
            error!("Failed to get current waypoint");
            return;
        }
        let cur_way = cur_way.unwrap();

        let display = DISPLAY_STATE.sender();
        display.send(display::DisplayState::Navigation(
            display::navigating::State {
                battery: {
                    match BATTERY_WATCH.receiver() {
                        None => {
                            error!("Failed to update screen: battery watch unavailable");
                            return;
                        }
                        Some(mut receiver) => receiver.get().await,
                    }
                },

                // goal data
                waypoint_name: cur_way.name.clone(),
                latitude: cur_way.coordinate.lat,
                longitude: cur_way.coordinate.long,

                // journey data
                distance: diff.distance_m,
                height_delta: {
                    if diff.height_delta.is_none() {

                    }
                    diff.height_delta.unwrap() as i32
                },
            },
        ))
    }

    async fn update_leds(&self, diff: &WaypointDifference) {
        let supposed_bearing = diff.needed_bearing as f32;
        
        let imu_data = IMU_WATCH.receiver().unwrap().get().await;
        let mag_data = MAG_WATCH.receiver().unwrap().get().await;
        let current_bearing: f32 = calculate_current_bearing(&mag_data, &imu_data);
        let delta_bearing = (current_bearing-supposed_bearing);
        
        let degrees_to_turn = if delta_bearing < 0f32 {
            360_f32 - delta_bearing
        } else {
            delta_bearing
        };
        let normalized = degrees_to_turn % 360.0;
        let normalized = if normalized < 0.0 { 
            normalized + 360.0 
        } else { 
            normalized 
        };
        
        // When horizontal and buttons on the right:
        // 0°   -> 17
        // 90°  -> 0
        // 180° -> 5
        // 270° -> 10
        // Each index represents 18° (360 / 20)
        
        let offset_degrees = (normalized - 90.0 + 360.0) % 360.0;
        
        let index = roundf((offset_degrees / 18.0)) as i32;
        
        // Wrap to 0-19 range
        let led_position = ((index % 20 + 20) % 20) as u8;
        if led_position > 19 {
            error!("LED position calculation is wrong");
            return;
        }

        LED_CMD.sender().send(LedCommands::Single(RGB8::new(255, 25, 25), led_position)).await;
    }

    async fn update_all(&self) {
        let gnss_receiver = GNSS_WATCH.receiver();
        if gnss_receiver.is_none() {
            warn!("Failed to get receiver from GNSS_WATCH");
            return;
        }
        let last_fix = gnss_receiver.unwrap().get().await;

        let cur_way = self.get_waypoint().await;
        if cur_way.is_none() {
            error!("Failed to get current waypoint");
            return;
        }
        let cur_way = cur_way.unwrap();

        let diff = calculate_waypoint_data(&cur_way, &last_fix);
        if diff.is_err() {
            error!("Failed to calculate diff");
            return;
        }
        let diff = diff.unwrap();

        self.update_display(&diff).await;
        self.update_leds(&diff).await;
    }
}
