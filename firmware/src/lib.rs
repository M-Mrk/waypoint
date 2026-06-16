#![no_std]
pub mod inputs;
pub mod logic;
pub mod navigation;
pub mod outputs;
pub mod power;
pub mod data;

use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use esp_hal::i2c::master as hardware_i2c;

pub type SharedI2c =
    I2cDevice<'static, CriticalSectionRawMutex, hardware_i2c::I2c<'static, esp_hal::Blocking>>;
