use defmt::{Format, info};
use embassy_futures::select::{Either4, select4};

use crate::inputs::{
    expander::{BTNS, ButtonCall},
    gnss::{GNSS_WATCH, GnssData},
    sensors::{IMU_WATCH, ImuData, MAG_WATCH, MagData},
};

mod navigating;
mod selecting;
mod settings;
mod sleep;
mod waypoints;

#[derive(Format, Clone, Copy)]
enum AppState {
    Sleep,
    Selecting,
    Navigating,
    Settings,
    Waypoints,
}

#[derive(Format, Clone, Copy)]
struct Inputs {
    pub button: Option<ButtonCall>,
    pub imu: Option<ImuData>,
    pub magn: Option<MagData>,
    pub gnss: Option<GnssData>,
}
impl Inputs {
    pub fn none() -> Self {
        Self {
            button: None,
            imu: None,
            magn: None,
            gnss: None,
        }
    }
}

#[derive(Format, Clone, Copy)]
struct App {
    pub current: AppState,
    pub last: AppState,
    pub changed: bool,

    pub inputs: Inputs,
}
impl App {
    pub fn new() -> Self {
        App {
            current: AppState::Selecting,
            last: AppState::Sleep,
            changed: true,

            inputs: Inputs::none(),
        }
    }

    pub fn clear_updates(&mut self) {
        self.inputs = Inputs::none();
    }

    pub fn set_state(&mut self, state: AppState) {
        self.last = self.current;
        self.current = state;
        self.changed = true;
    }
}

#[embassy_executor::task]
pub async fn logic_task() {
    // let mut perm_storage = FlashStorage::new(flash);
    let mut app = App::new();

    let mut sleep_state = sleep::SleepState::new();
    let mut selecting_state = selecting::SelectingState::new();
    let mut navigating_state = navigating::NavigatingState::new();
    let mut settings_state = settings::SettingsState::new();
    let mut waypoints_state = waypoints::WaypointsState::new();

    loop {
        if app.changed {
            // Run init and denit for states if in transition
            match app.last {
                AppState::Sleep => sleep_state.deintialize(&mut app).await,
                _ => info!("def"),
            }

            info!("AppState changed to {}", &&app.current);
            match app.current {
                AppState::Sleep => sleep_state.initalize(&mut app).await,
                _ => info!("AppState changed to {}", &&app.current),
            }
            app.changed = false;
        }

        match select4(
            // Await all possible changes
            BTNS.wait(),
            IMU_WATCH.receiver().unwrap().changed(),
            MAG_WATCH.receiver().unwrap().changed(),
            GNSS_WATCH.receiver().unwrap().changed(),
        )
        .await
        {
            Either4::First(button_call) => {
                app.inputs.button = Some(button_call);
            }
            Either4::Second(imu_data) => {
                app.inputs.imu = Some(imu_data);
            }
            Either4::Third(mag_data) => {
                app.inputs.magn = Some(mag_data);
            }
            Either4::Fourth(gnss_data) => {
                app.inputs.gnss = Some(gnss_data);
            }
        }

        match app.current {
            // Give update to current states handler
            AppState::Sleep => sleep_state.handle(&mut app).await,
            AppState::Selecting => selecting_state.handle(&mut app).await,
            AppState::Navigating => navigating_state.handle(&mut app).await,
            AppState::Settings => settings_state.handle(&mut app).await,
            AppState::Waypoints => waypoints_state.handle(&mut app).await,
        }

        app.clear_updates();
    }
}
