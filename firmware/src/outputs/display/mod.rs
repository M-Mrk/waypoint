use defmt::error;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_graphics_framebuf::FrameBuf;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::gpio;
use esp_hal::time::Rate;
use esp_hal::{Blocking, spi};
use mipidsi::interface::SpiInterface;
use mipidsi::{Builder, models::GC9A01}; // Provides the builder for Display
use static_cell::StaticCell;

use esp_hal::ledc::{
    LSGlobalClkSource, Ledc, LowSpeed,
    channel::{self, ChannelIFace},
    timer::{self, TimerIFace},
};

use crate::power::BATTERY_WATCH;

type DisplayDriver = mipidsi::Display<
    SpiInterface<
        'static,
        ExclusiveDevice<
            spi::master::Spi<'static, Blocking>,
            gpio::Output<'static>,
            embassy_time::Delay,
        >,
        gpio::Output<'static>,
    >,
    GC9A01,
    gpio::Output<'static>,
>;

type FrameBuffer = FrameBuf<Rgb565, &'static mut FbData>;

type FbData = [Rgb565; 240 * 240];
static FB: StaticCell<FbData> = StaticCell::new();

#[derive(Clone, Copy, PartialEq)]
pub enum DisplayState {
    MainPage,
}

pub static DISPLAY_STATE: Watch<CriticalSectionRawMutex, DisplayState, 2> = Watch::new();

mod main_page;

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
    static BUFFER_CELL: StaticCell<[u8; 512]> = StaticCell::new();
    let buffer = BUFFER_CELL.init([0_u8; 512]);

    let spi_device = ExclusiveDevice::new(spi, cs, embassy_time::Delay).unwrap();
    let di = SpiInterface::new(spi_device, dc, buffer);

    let mut display: DisplayDriver = match Builder::new(GC9A01, di)
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

    let fb_data = FB.init([Rgb565::BLACK; 240 * 240]);
    let mut fb: FrameBuffer = FrameBuf::new(fb_data, 240, 240);

    let mut state_rx = DISPLAY_STATE.receiver().unwrap();

    loop {
        let new_state = state_rx.changed().await;

        fb.clear(Rgb565::BLACK);

        match new_state {
            DisplayState::MainPage => {
                let battery_state = BATTERY_WATCH.try_get().unwrap();
                let values = main_page::State {
                    bat_percent: battery_state.percent,
                    charging: battery_state.charging,
                    current_item: main_page::Item::Navigation,
                };
                main_page::draw(&mut fb, &values).await;
            }
        }
    }
}
