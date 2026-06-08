use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::watch::Watch;
use embassy_time::{Duration, Ticker};
use esp_hal::analog::adc;
use esp_hal::gpio;

use crate::inputs::expander::{CHARGING, POWER_GOOD};

pub enum PowerDomain {
    Main,
    Leds,
    Gnss,
}

pub enum PowerCommand {
    Enable(PowerDomain),
    Disable(PowerDomain),
    Sleep,
}

#[derive(Clone, Copy)]
pub struct BatteryState {
    pub mili_voltage: f32,
    pub percent: u8,
    pub charging: bool,
    pub input_power: bool,
}

pub static POWER_CMD: Channel<CriticalSectionRawMutex, PowerCommand, 4> = Channel::new();
pub static BATTERY_WATCH: Watch<CriticalSectionRawMutex, BatteryState, 3> = Watch::new();

pub type AdcType = adc::Adc<'static, esp_hal::peripherals::ADC1<'static>, esp_hal::Async>;

pub type AdcPin =
    adc::AdcPin<esp_hal::peripherals::GPIO1<'static>, esp_hal::peripherals::ADC1<'static>>;

pub struct PowerPins {
    pub main_en: gpio::Output<'static>,
    pub gnss_en: gpio::Output<'static>,
    pub leds_en: gpio::Output<'static>,
}

#[embassy_executor::task]
pub async fn power_management_task(
    mut en_pins: PowerPins,
    mut battery_adc: AdcType,
    mut battery_pin: AdcPin,
) {
    let bat_sender = BATTERY_WATCH.sender();

    let mut battery_ticker = Ticker::every(Duration::from_secs(30));

    loop {
        match select(POWER_CMD.receive(), battery_ticker.next()).await {
            Either::First(cmd) => handle_command(cmd, &mut en_pins),
            Either::Second(_) => {
                let state = read_battery(&mut battery_adc, &mut battery_pin).await;
                bat_sender.send(state);
            }
        }
    }
}

async fn read_battery(bat_adc: &mut AdcType, bat_pin: &mut AdcPin) -> BatteryState {
    let raw = bat_adc.read_oneshot(bat_pin).await;
    const R1: u32 = 10000; // 10kR
    const R2: u32 = 30000; // 30kR

    let raw_voltage = (raw as u32 * 1100) / 4095; // Vout
    let bat_voltage = (raw_voltage as f32 * (R1 + R2) as f32) / R2 as f32;

    const MAX_BAT: f32 = 4200f32; // 4.2V at fully charged
    const MIN_BAT: f32 = 3500f32; // 3.5V at cutoff

    let percent = if bat_voltage >= MAX_BAT {
        100
    } else if bat_voltage <= MIN_BAT {
        0
    } else {
        (((bat_voltage - MIN_BAT) * 100f32) / (MAX_BAT - MIN_BAT)) as u8
    };

    return BatteryState {
        mili_voltage: bat_voltage,
        percent: percent,
        charging: CHARGING.receiver().unwrap().get().await,
        input_power: POWER_GOOD.receiver().unwrap().get().await,
    };
}

fn handle_command(cmd: PowerCommand, pins: &mut PowerPins) {
    match cmd {
        PowerCommand::Enable(domain) => set_state_powerdomain(gpio::Level::High, domain, pins),
        PowerCommand::Disable(domain) => set_state_powerdomain(gpio::Level::Low, domain, pins),
        PowerCommand::Sleep => {
            set_state_powerdomain(gpio::Level::Low, PowerDomain::Gnss, pins);
            set_state_powerdomain(gpio::Level::Low, PowerDomain::Leds, pins);
        }
    }
}

fn set_state_powerdomain(level: gpio::Level, domain: PowerDomain, pins: &mut PowerPins) {
    let pin = match domain {
        PowerDomain::Main => &mut pins.main_en,
        PowerDomain::Leds => &mut pins.leds_en,
        PowerDomain::Gnss => &mut pins.gnss_en,
    };
    pin.set_level(level);
}
