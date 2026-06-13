use defmt::error;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::gpio;
use esp_hal::time::Rate;
use esp_hal::{Blocking, spi};
use heapless::{String, format};
use static_cell::StaticCell;

use embedded_graphics::{
    mono_font,
    pixelcolor::Rgb565,
    prelude::*,
    primitives,
    text::{Alignment, Text},
};
use embedded_graphics_framebuf::FrameBuf;
use mipidsi::interface::SpiInterface;
use mipidsi::{Builder, models::GC9A01}; // Provides the builder for Display

use esp_hal::ledc::{
    LSGlobalClkSource, Ledc, LowSpeed,
    channel::{self, ChannelIFace},
    timer::{self, TimerIFace},
};

use crate::power::BatteryState;

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

mod main_page;
mod navigate_page;

#[derive(Clone, PartialEq)]
pub enum DisplayState {
    MainPage(main_page::State),
    Navigation(navigate_page::State),
}

pub static DISPLAY_STATE: Watch<CriticalSectionRawMutex, DisplayState, 2> = Watch::new();

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
            DisplayState::MainPage(state) => {
                main_page::draw(&mut fb, &state).await;
            }
            DisplayState::Navigation(state) => {
                navigate_page::draw(&mut fb, &state).await;
            }
        }
    }
}

pub fn map(x: u32, in_min: u32, in_max: u32, out_min: u32, out_max: u32) -> u32 {
    return (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min;
}

pub fn battery(display: &mut FrameBuffer, state: &BatteryState) -> Result<(), ()> {
    // Battery
    const BAT_LEFT_X: i32 = 41;
    const BAT_HEIGHT: i32 = 30;
    let text_style =
        mono_font::MonoTextStyle::new(&mono_font::ascii::FONT_9X15_BOLD, Rgb565::WHITE);
    const HALF_TEXT_HEIGHT: i32 = 5;

    // background
    primitives::Rectangle::new(
        Point {
            x: BAT_LEFT_X,
            y: 0,
        },
        Size {
            width: (240 - (BAT_LEFT_X * 2)) as u32,
            height: BAT_HEIGHT as u32,
        },
    )
    .into_styled(primitives::PrimitiveStyle::with_fill(Rgb565::CSS_GRAY))
    .draw(display);

    // bar
    let percent: String<5> = format!("{}%", state.percent).unwrap();
    let charge_width = map(
        state.percent as u32,
        0,
        100,
        0,
        (240 - BAT_LEFT_X * 2) as u32,
    );
    let mut color = match state.percent {
        low if low < 15 => Rgb565::CSS_RED,
        _ => Rgb565::CSS_DARK_SEA_GREEN,
    };
    if state.charging {
        color = Rgb565::CSS_GOLDENROD;
    }

    primitives::Rectangle::new(
        Point {
            x: BAT_LEFT_X,
            y: 0,
        },
        Size {
            width: charge_width,
            height: BAT_HEIGHT as u32,
        },
    )
    .into_styled(primitives::PrimitiveStyle::with_fill(color))
    .draw(display)
    .map_err(|_| ())?;

    Text::with_alignment(
        &percent,
        Point {
            x: 120,
            y: (BAT_HEIGHT / 2 + HALF_TEXT_HEIGHT),
        },
        text_style,
        Alignment::Center,
    )
    .draw(display)
    .map_err(|_| ())?;

    Ok(())
}
