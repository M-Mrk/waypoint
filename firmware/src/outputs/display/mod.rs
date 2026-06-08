use defmt::error;
use embedded_graphics;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::gpio;
use esp_hal::time::Rate;
use esp_hal::{Blocking, spi};
use mipidsi::interface::SpiInterface;
use mipidsi::{Builder, models::GC9A01}; // Provides the builder for Display

use esp_hal::ledc::{
    LSGlobalClkSource, Ledc, LowSpeed,
    channel::{self, ChannelIFace},
    timer::{self, TimerIFace},
};

#[embassy_executor::task]
pub async fn display_task(
    spi: spi::master::Spi<'static, Blocking>, // <-- Blocking, not Async
    cs: gpio::Output<'static>,
    dc: gpio::Output<'static>,
    reset: gpio::Output<'static>,
    blk: esp_hal::peripherals::GPIO22<'static>,
    ledc: esp_hal::peripherals::LEDC<'static>,
) {
    // pwm setup
    let mut ledc = Ledc::new(ledc);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let mut lstimer0 = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    if let Err(e) = lstimer0.configure(timer::config::Config {
        duty: timer::config::Duty::Duty5Bit,
        clock_source: timer::LSClockSource::APBClk,
        frequency: Rate::from_khz(24),
    }) {
        error!("Failed to intiate ledc timer: {}", e)
    }

    let mut channel0 = ledc.channel(channel::Number::Channel0, blk);
    if let Err(e) = channel0.configure(channel::config::Config {
        timer: &lstimer0,
        duty_pct: 10,
        drive_mode: gpio::DriveMode::PushPull,
    }) {
        error!("Failed to initate ledc channel: {}", e)
    }

    if let Err(e) = channel0.set_duty(50) {
        error!("Failed to set duty cycle for backlight: {}", e)
    }

    // display setup
    let mut buffer = [0_u8; 512];

    let spi_device = ExclusiveDevice::new(spi, cs, embassy_time::Delay).unwrap();
    let di = SpiInterface::new(spi_device, dc, &mut buffer);

    let mut display = match Builder::new(GC9A01, di)
        .reset_pin(reset)
        .init(&mut embassy_time::Delay)
    {
        Ok(dis) => dis,
        Err(e) => {
            error!("Failed to intialize display: {}", e);
            return;
        }
    };

    if let Err(e) = display.clear(Rgb565::BLACK) {
        error!("Failed to clear display: {}", e);
    }
}
