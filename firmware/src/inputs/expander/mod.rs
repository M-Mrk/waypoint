use crate::SharedI2c;
use defmt::{Format, error, info};
use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_futures::select::{Either, select};
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::once_lock::OnceLock;
use embassy_sync::watch::Watch;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant};
use esp_hal::gpio::Input;
use esp_hal::i2c::master::I2c as HardwareI2c;
use tca9534_driver_rs::{self as expander, PinConfig, PinLevel};

type ExpanderDevice = tca9534_driver_rs::TCA9534Sync<
    I2cDevice<'static, CriticalSectionRawMutex, HardwareI2c<'static, esp_hal::Blocking>>,
>;

static EXPANDER_CONFIG: OnceLock<Mutex<CriticalSectionRawMutex, CurrentConfig>> = OnceLock::new();

const NUM_PINS: usize = 8;
#[repr(u8)]
#[derive(Clone, Debug)]
pub enum ExpanderPinMapping {
    ChargingIndicator = 0,
    PowerGoodIndicator = 1,
    LeftSwitch = 2,
    OkSwitch = 3,
    ImuInit = 4,
    GnssEnable = 5,
    PowerSwitch = 6,
    RightSwitch = 7,
}
impl ExpanderPinMapping {
    pub fn as_u8(self) -> u8 {
        return self as u8;
    }

    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::ChargingIndicator),
            1 => Some(Self::PowerGoodIndicator),
            2 => Some(Self::LeftSwitch),
            3 => Some(Self::OkSwitch),
            4 => Some(Self::ImuInit),
            5 => Some(Self::GnssEnable),
            6 => Some(Self::PowerSwitch),
            7 => Some(Self::RightSwitch),
            _ => None,
        }
    }

    pub fn as_usize(self) -> usize {
        return self as usize;
    }

    pub fn as_button_type(self) -> ButtonType {
        match self {
            Self::LeftSwitch => ButtonType::Left,
            Self::OkSwitch => ButtonType::Ok,
            Self::RightSwitch => ButtonType::Right,
            Self::PowerSwitch => ButtonType::Power,
            _ => {
                panic!("Unable to convert self to ButtonType: {:?}", self)
            }
        }
    }
}

pub struct CurrentConfig {
    pins: [PinConfig; NUM_PINS],
    expander: ExpanderDevice,
}

#[derive(Format, Clone, Copy)]
pub enum ButtonType {
    // TODO: implement these
    Left,
    Ok,
    Right,
    Power,
}

#[derive(Format, Clone, Copy)]
pub enum ButtonAction {
    Single,
    Double,
    Long,
}

#[derive(Format, Clone, Copy)]
pub struct ButtonCall {
    pub function: ButtonType,
    pub action: ButtonAction,
}

pub static BTNS: Signal<CriticalSectionRawMutex, ButtonCall> = Signal::new();

pub static IMU_INTERRUPT: Signal<CriticalSectionRawMutex, PinLevel> = Signal::new();
pub static CHARGING: Watch<CriticalSectionRawMutex, bool, 3> = Watch::new();
pub static POWER_GOOD: Watch<CriticalSectionRawMutex, bool, 3> = Watch::new();

pub enum ExpanderCommand {
    SetGnssEnable(PinLevel),
}

pub static EXPANDER_CMD: Channel<CriticalSectionRawMutex, ExpanderCommand, 4> = Channel::new();

async fn initialize(i2c_bus: SharedI2c) {
    let mut expander = expander::TCA9534Sync::new_with_default_address(i2c_bus);
    if expander.init().is_err() {
        error!("Failed to initialize io expander");
        return;
    }

    let pins = [PinConfig::Input; NUM_PINS];
    let config = CurrentConfig {
        pins: pins,
        expander: expander,
    };

    if EXPANDER_CONFIG.init(Mutex::new(config)).is_err() {
        error!("Failed to initialize mutex with expander object");
    }
}

async fn set_default_states() {
    let mutex = EXPANDER_CONFIG.get().await;
    let mut guard = mutex.lock().await;
    let exp_conf = &mut *guard;
    if exp_conf
        .expander
        .set_pin_config(ExpanderPinMapping::GnssEnable.as_u8(), PinConfig::Output)
        .is_err()
    {
        error!("Failed to set GnssEnable pin to output");
        return;
    };
    if exp_conf
        .expander
        .set_pin_output(ExpanderPinMapping::GnssEnable.as_u8(), PinLevel::Low)
        .is_err()
    {
        error!("Failed to set GnssEnable pin to LOW");
        return;
    };
    exp_conf.pins[ExpanderPinMapping::GnssEnable.as_usize()] = PinConfig::Output;
}

fn pin_level_to_bool(level: PinLevel) -> bool {
    match level {
        PinLevel::High => true,
        PinLevel::Low => false,
    }
}

fn is_debounced(last_trigger: &mut Instant) -> bool {
    const DEBOUNCE_DELAY: Duration = Duration::from_millis(20);
    Instant::now() - *last_trigger > DEBOUNCE_DELAY
}

#[embassy_executor::task]
pub async fn expander_task(i2c: SharedI2c, mut int_pin: Input<'static>) {
    initialize(i2c).await;
    set_default_states().await;
    let mutex = EXPANDER_CONFIG.get().await;
    let mut guard = mutex.lock().await;
    let exp_conf = &mut *guard;

    let mut last_left = Instant::now();
    let mut last_ok = Instant::now();
    let mut last_right = Instant::now();
    let mut last_power = Instant::now();

    'main: loop {
        match select(int_pin.wait_for_rising_edge(), EXPANDER_CMD.receive()).await {
            Either::First(_) => {
                let mut causing_pin: Option<u8> = None;
                let mut causing_state: Option<PinLevel> = None;

                'check: for pin in 0..NUM_PINS {
                    if exp_conf.pins[pin] != PinConfig::Input {
                        continue 'check;
                    }
                    let state = match exp_conf.expander.read_pin_input(pin as u8) {
                        Ok(state) => state,
                        Err(_) => {
                            error!("Failed to read from pin {} after Interrupt", &pin);
                            continue 'main;
                        }
                    };
                    if int_pin.is_low() {
                        // Interrupt got disabled, meaning read pin was the cause
                        causing_pin = Some(pin as u8);
                        causing_state = Some(state);
                    }
                }

                match (causing_pin, causing_state) {
                    (Some(pin), Some(state)) => {
                        let pin = match ExpanderPinMapping::from_u8(pin) {
                            Some(n) => n,
                            None => {
                                error!("Impossible causing pin reached");
                                continue;
                            }
                        };

                        let last_trigger = match pin {
                            ExpanderPinMapping::LeftSwitch => Some(&mut last_left),
                            ExpanderPinMapping::OkSwitch => Some(&mut last_ok),
                            ExpanderPinMapping::RightSwitch => Some(&mut last_right),
                            ExpanderPinMapping::PowerSwitch => Some(&mut last_power),
                            _ => None,
                        };
                        if last_trigger.is_some() {
                            // needs to be debounced
                            let last_trigger = last_trigger.unwrap();
                            if is_debounced(last_trigger) {
                                *last_trigger = Instant::now(); // Reset debouncing timer
                            } else {
                                continue; // Button is bouncing
                            }
                        }
                        match pin {
                            ExpanderPinMapping::ImuInit => IMU_INTERRUPT.signal(state),
                            ExpanderPinMapping::ChargingIndicator => {
                                CHARGING.sender().send(pin_level_to_bool(state));
                                continue;
                            }
                            ExpanderPinMapping::PowerGoodIndicator => {
                                POWER_GOOD.sender().send(pin_level_to_bool(state));
                                continue;
                            }
                            _ => {}
                        }

                        // Only buttons from here on
                        if state == PinLevel::Low {
                            continue; // Don't care about buttons stop being pressed
                        }
                        let button_type = pin.as_button_type();
                        BTNS.signal(ButtonCall {
                            function: button_type,
                            action: ButtonAction::Single,
                        });
                    }
                    _ => info!("Interrupt triggered but no active low input pin found"),
                }
            }

            Either::Second(cmd) => match cmd {
                ExpanderCommand::SetGnssEnable(level) => {
                    if exp_conf
                        .expander
                        .set_pin_output(ExpanderPinMapping::GnssEnable.as_u8(), level)
                        .is_err()
                    {
                        error!("Failed to set GnssEnable level over expander")
                    };
                }
            },
        }
    }
}
