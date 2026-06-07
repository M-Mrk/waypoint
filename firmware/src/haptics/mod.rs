use defmt::{Format, error};
use defmt::{debug, info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embedded_hal_compat::ReverseCompat;

use drv2605l::Effect as HapticEffect;

use crate::SharedI2c;

#[derive(PartialEq, Format)]
pub enum HapticsState {
    Idle,
    Single,
    Double,
    Long,
    Standby,
}

pub static HAPTIC_CMD: Channel<CriticalSectionRawMutex, HapticsState, 4> = Channel::new();

// motor => https://de.aliexpress.com/item/1005008858705264.html

#[embassy_executor::task]
pub async fn haptics_task(i2c_bus: SharedI2c) {
    let mut in_standby = false;

    let motor_params = drv2605l::CalibrationParams::default();
    let mut drv = match drv2605l::Drv2605l::new(
        i2c_bus.reverse(),
        drv2605l::Calibration::Auto(motor_params),
        true,
    ) {
        Ok(d) => d,
        Err(_) => {
            error!("Failed to create drv2605l");
            return;
        }
    };

    match drv.calibration() {
        Ok(_) => info!("Calibrated drv2605l"),
        Err(_) => {
            error!("Failed to calibrate drv2605l");
            return;
        }
    }

    loop {
        let state = HAPTIC_CMD.receive().await;
        debug!("Received new HapticsState {}", &state);
        if in_standby && state != HapticsState::Standby {
            if drv.set_standby(false).is_err() {
                error!("Failed to awake drv2605l");
                continue;
            }
            in_standby = false;
        }

        let effect: HapticEffect = match state {
            HapticsState::Idle => {
                continue;
            }
            HapticsState::Single => HapticEffect::SharpClick100,
            HapticsState::Double => HapticEffect::DoubleClick60,
            HapticsState::Long => HapticEffect::LongBuzzForProgrammaticStopping100,
            HapticsState::Standby => {
                in_standby = true;
                drv.set_standby(true)
                    .unwrap_or_else(|_| error!("Failed to put drv2605l to sleep"));
                continue;
            }
        };
        drv.set_rom_single(effect)
            .unwrap_or_else(|_| error!("Failed to set effect"));
        drv.set_go()
            .unwrap_or_else(|_| error!("Failed to start effect"));
    }
}
