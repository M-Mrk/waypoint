use defmt::{debug, error, warn};
use embassy_time::{Duration, Timer};
use lsm6ds3tr::{LSM6DS3TR, interface::I2cInterface};

use super::IMU_WATCH;
use super::{ImuData, ThreeD};
use crate::SharedI2c;

// const I2C_ADDRESS: u8 = 0x6A;

#[embassy_executor::task]
pub async fn imu_task(i2c_bus: SharedI2c) {
    let mut imu = LSM6DS3TR::new(I2cInterface::new(i2c_bus));
    if imu.init().is_err() {
        error!("Failed to initialize IMU");
        return;
    }

    let sender = IMU_WATCH.sender();
    let mut data = ImuData {
        acceleration: ThreeD {
            x: 0f32,
            y: 0f32,
            z: 0f32,
        },
        gyroscope: ThreeD {
            x: 0f32,
            y: 0f32,
            z: 0f32,
        },
        temperature: 0f32,
    };

    loop {
        match imu.read_temp() {
            Ok(temp) => data.temperature = temp,
            Err(_) => warn!("Failed to read IMU temperature"),
        }

        match imu.read_accel() {
            Ok(accel) => data.acceleration = ThreeD::from_xyz(accel),
            Err(_) => warn!("Failed to read IMU acceleration"),
        }

        match imu.read_gyro() {
            Ok(gyro) => data.gyroscope = ThreeD::from_xyz(gyro),
            Err(_) => warn!("Failed to read IMU gyroscope"),
        }

        debug!("IMU: {}", &data);
        sender.send(data);
        Timer::after(Duration::from_secs(1)).await;
    }
}
