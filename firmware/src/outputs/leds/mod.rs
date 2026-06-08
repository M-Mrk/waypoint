use core::iter;
use defmt::error;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use esp_hal::rmt;
use esp_hal_smartled::{RmtSmartLeds, Sk68xxTiming, buffer_size, color_order};
use smart_leds::{RGB8, SmartLedsWriteAsync, brightness};

use crate::power::POWER_CMD;

#[derive(PartialEq)]
pub enum LedCommands {
    Off,
    SetBrightness(u8),
    Blink(RGB8),
    Single(RGB8, u8),
    FillTo(RGB8, u8),
}

const NUM_LEDS: usize = 20;

type LedDriver = RmtSmartLeds<
    'static,
    { buffer_size::<RGB8>(NUM_LEDS) },
    esp_hal::Async,
    smart_leds::RGB<u8>,
    color_order::Grb,
    Sk68xxTiming,
>;

async fn all_black(led: &mut LedDriver) {
    let all_off = brightness(iter::repeat(RGB8::default()).take(NUM_LEDS), 0);
    if let Err(e) = led.write(all_off).await {
        error!("Failed to write leds: {}", e)
    }
}

async fn set_power_state(powered: bool) {
    if powered {
        POWER_CMD
            .send(crate::power::PowerCommand::Enable(
                crate::power::PowerDomain::Leds,
            ))
            .await;
    } else {
        POWER_CMD
            .send(crate::power::PowerCommand::Disable(
                crate::power::PowerDomain::Leds,
            ))
            .await;
    }
}

/*
    Clockwise:
    16  -> 0º
    1   -> 90º
    6   -> 180º
    11  -> 270º
*/

pub static LED_CMD: Channel<CriticalSectionRawMutex, LedCommands, 4> = Channel::new();
#[embassy_executor::task]
pub async fn leds_task(
    rmt: rmt::Rmt<'static, esp_hal::Async>,
    pin: esp_hal::peripherals::GPIO23<'static>,
) {
    let mut led = match RmtSmartLeds::<
        { buffer_size::<RGB8>(NUM_LEDS) },
        _,
        RGB8,
        color_order::Grb,
        Sk68xxTiming,
    >::new(rmt.channel0, pin)
    {
        Ok(led) => led,
        Err(err) => {
            error!("Failed to create led driver object: {}", &err);
            return;
        }
    };

    let mut saved_brightness: u8 = 125;
    let mut powered_on: bool = false;
    loop {
        let received_cmd = LED_CMD.receive().await;
        if received_cmd == LedCommands::Off {
            all_black(&mut led).await;
            set_power_state(false).await;
            powered_on = false;
            continue;
        }
        if !powered_on {
            set_power_state(true).await;
            powered_on = true;
            // Timer::after(Duration::from_nanos(50)).await; // Grace period for leds to power up
        }

        match LED_CMD.receive().await {
            LedCommands::SetBrightness(bright) => {
                saved_brightness = bright;
            }
            LedCommands::Blink(color) => {
                let pixels = brightness(iter::repeat(color).take(NUM_LEDS), saved_brightness);
                if let Err(e) = led.write(pixels).await {
                    error!("Failed to write leds: {}", e)
                }
                Timer::after(Duration::from_millis(500)).await;
                all_black(&mut led).await;
            }
            LedCommands::Single(color, position) => {
                let position = position.clamp(1, 20);
                let mut raw_pixels = [RGB8::default(); NUM_LEDS];
                raw_pixels[(position - 1) as usize] = color;

                let pixels = brightness(raw_pixels.iter().cloned(), saved_brightness);
                if let Err(e) = led.write(pixels).await {
                    error!("Failed to write leds: {}", e)
                }
            }
            LedCommands::FillTo(color, end) => {
                let end = end.clamp(1, 20);
                let mut raw_pixels = [RGB8::default(); NUM_LEDS];
                raw_pixels[0..(end as usize)].fill(color);

                let pixels = brightness(raw_pixels.iter().cloned(), saved_brightness);
                if let Err(e) = led.write(pixels).await {
                    error!("Failed to write leds: {}", e)
                }
            }
            _ => {}
        }
    }
}
