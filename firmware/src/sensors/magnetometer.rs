use defmt::{info, warn};
use embassy_time::{Duration, Timer};
use iis2mdc_rs::blocking::{
    I2CAddress, IIS2MDC_ID, Iis2mdc, from_lsb_to_celsius, from_lsb_to_mgauss,
    register::main::{Md, Odr},
};

use crate::SharedI2c;

use super::MAG_WATCH;
use super::{MagData, ThreeD};

#[embassy_executor::task]
pub async fn magnetometer_task(i2c_bus: SharedI2c) {
    let mut magneto = Iis2mdc::new_i2c(i2c_bus, I2CAddress::I2cAdd, embassy_time::Delay);

    let sender = MAG_WATCH.sender();
    let mut data = MagData {
        magnetic: ThreeD {
            x: 0f32,
            y: 0f32,
            z: 0f32,
        },
        temperature: 0f32,
    };

    match magneto.device_id_get() {
        Ok(id) if id == IIS2MDC_ID => info!("Magnetometer detected: 0x{:x}", id),
        Ok(id) => warn!("Unexpected magnetometer ID: 0x{:x}", id),
        Err(_) => warn!("Failed to read magnetometer device ID"),
    };

    if magneto.data_rate_set(Odr::_10hz).is_err() {
        warn!("Failed to set magnetometer data rate");
    }
    if magneto.operating_mode_set(Md::ContinuousMode).is_err() {
        warn!("Failed to set magnetometer mode");
    }

    loop {
        match magneto.mag_data_ready_get() {
            Ok(1) => match magneto.magnetic_raw_get() {
                Ok([x, y, z]) => {
                    data.magnetic = ThreeD {
                        x: from_lsb_to_mgauss(x),
                        y: from_lsb_to_mgauss(y),
                        z: from_lsb_to_mgauss(z),
                    }
                }
                Err(_) => warn!("Failed to read magnetometer axes"),
            },
            Ok(_) => {}
            Err(_) => warn!("Failed to check magnetometer data-ready"),
        }

        match magneto.temperature_raw_get() {
            Ok(temp) => data.temperature = from_lsb_to_celsius(temp),
            Err(_) => warn!("Failed to read magnetometer temperature"),
        }

        sender.send(data);
        Timer::after(Duration::from_millis(500)).await;
    }
}
