use defmt::Format;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::Watch;
use lsm6ds3tr::XYZ;

pub mod imu;
pub mod magnetometer;

#[derive(Clone, Copy, Debug, Format)]
pub struct ThreeD {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
impl ThreeD {
    pub fn from_xyz(xyz: XYZ<f32>) -> Self {
        return ThreeD {
            x: xyz.x,
            y: xyz.y,
            z: xyz.z,
        };
    }
}

#[derive(Clone, Copy, Debug, Format)]
pub struct ImuData {
    pub acceleration: ThreeD,
    pub gyroscope: ThreeD,
    pub temperature: f32,
}
pub static IMU_WATCH: Watch<CriticalSectionRawMutex, ImuData, 2> = Watch::new();

#[derive(Clone, Copy, Debug, Format)]
pub struct MagData {
    pub magnetic: ThreeD,
    pub temperature: f32,
}
pub static MAG_WATCH: Watch<CriticalSectionRawMutex, MagData, 2> = Watch::new();
