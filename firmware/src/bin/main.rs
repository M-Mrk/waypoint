#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::cell::RefCell;
use defmt::{error, info};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use static_cell::StaticCell;

use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use esp_backtrace as _;
use esp_hal::analog::adc;
use esp_hal::clock::CpuClock;
use esp_hal::gpio;
use esp_hal::i2c::master as hardware_i2c;
use esp_hal::rmt;
use esp_hal::spi;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::uart;
use esp_println as _;

extern crate alloc;

use firmware::{inputs, logic, outputs, power};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

static I2C_BUS: StaticCell<
    Mutex<CriticalSectionRawMutex, RefCell<hardware_i2c::I2c<'static, esp_hal::Blocking>>>,
> = StaticCell::new();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32c6 -o esp32c6-wroom-1 -o unstable-hal -o embassy -o defmt -o esp-backtrace -o vscode -o neovim

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The following pins are used to bootstrap the chip. They are available
    // for use, but check the datasheet of the module for more information on them.
    // - GPIO4
    // - GPIO5
    // - GPIO8
    // - GPIO9
    // - GPIO15
    // These GPIO pins are in use by some feature of the module and should not be used.
    let _ = peripherals.GPIO24;
    let _ = peripherals.GPIO25;
    let _ = peripherals.GPIO26;
    let _ = peripherals.GPIO27;
    let _ = peripherals.GPIO28;
    let _ = peripherals.GPIO29;
    let _ = peripherals.GPIO30;

    let main_power_en = gpio::Output::new(
        peripherals.GPIO9,
        gpio::Level::High,
        gpio::OutputConfig::default(),
    );

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    // TODO: Spawn some tasks
    let _ = spawner;

    // power
    let mut built_in_led = gpio::Output::new(
        peripherals.GPIO15,
        gpio::Level::Low,
        gpio::OutputConfig::default(),
    );

    let gnss_power_en = gpio::Output::new(
        peripherals.GPIO8,
        gpio::Level::Low,
        gpio::OutputConfig::default(),
    );

    let leds_power_en = gpio::Output::new(
        peripherals.GPIO0,
        gpio::Level::Low,
        gpio::OutputConfig::default(),
    );

    let power_en_pins = firmware::power::PowerPins {
        main_en: main_power_en,
        gnss_en: gnss_power_en,
        leds_en: leds_power_en,
    };

    let mut battery_adc_config = adc::AdcConfig::new();
    let batt_pin = battery_adc_config.enable_pin(peripherals.GPIO1, adc::Attenuation::_11dB);
    let batt_sense = adc::Adc::new(peripherals.ADC1, battery_adc_config).into_async();
    if let Ok(token) = firmware::power::power_management_task(power_en_pins, batt_sense, batt_pin) {
        spawner.spawn(token);
    } else {
        error!("Failed to create power task");
    }

    // i2c setup
    let i2c = hardware_i2c::I2c::new(peripherals.I2C0, hardware_i2c::Config::default())
        .unwrap()
        .with_scl(peripherals.GPIO10)
        .with_sda(peripherals.GPIO11);

    let i2c_bus = I2C_BUS.init(Mutex::new(RefCell::new(i2c)));

    // IMU
    let imu_i2c = I2cDevice::new(i2c_bus);
    if let Ok(token) = inputs::sensors::imu::imu_task(imu_i2c) {
        spawner.spawn(token);
    } else {
        error!("Failed to create IMU task");
    }

    // Magnetometer
    let mag_i2c = I2cDevice::new(i2c_bus);
    if let Ok(token) = inputs::sensors::magnetometer::magnetometer_task(mag_i2c) {
        spawner.spawn(token);
    } else {
        error!("Failed to create Magnetometer task")
    }

    // Haptics
    let haptic_i2c = I2cDevice::new(i2c_bus);
    if let Ok(token) = outputs::haptics::haptics_task(haptic_i2c) {
        spawner.spawn(token);
    } else {
        error!("Failed to create Haptics task")
    }

    // I/O Expander
    let expander_int_pin = gpio::Input::new(peripherals.GPIO3, gpio::InputConfig::default());
    let expander_i2c = I2cDevice::new(i2c_bus);
    if let Ok(token) = inputs::expander::expander_task(expander_i2c, expander_int_pin) {
        spawner.spawn(token);
    } else {
        error!("Failed to create expander interrupt task")
    }

    // gnss
    let gnss_uart = uart::Uart::new(
        peripherals.UART1,
        uart::Config::default().with_baudrate(9600),
    )
    .unwrap()
    .with_rx(peripherals.GPIO18)
    .with_tx(peripherals.GPIO19)
    .into_async();

    if let Ok(token) = inputs::gnss::gnss_task(gnss_uart) {
        spawner.spawn(token);
    } else {
        error!("Failed to create gnss task")
    }

    // leds
    let rmt = rmt::Rmt::new(peripherals.RMT, esp_hal::time::Rate::from_mhz(80))
        .unwrap()
        .into_async();
    if let Ok(token) = outputs::leds::leds_task(rmt, peripherals.GPIO23) {
        spawner.spawn(token);
    } else {
        error!("Failed to create led task")
    }

    // display
    let spi = spi::master::Spi::new(peripherals.SPI2, spi::master::Config::default())
        .unwrap()
        .with_mosi(peripherals.GPIO7)
        .with_sck(peripherals.GPIO6);

    let cs = gpio::Output::new(
        peripherals.GPIO20,
        gpio::Level::Low,
        gpio::OutputConfig::default(),
    );
    let dc = gpio::Output::new(
        peripherals.GPIO21,
        gpio::Level::Low,
        gpio::OutputConfig::default(),
    );
    let reset = gpio::Output::new(
        peripherals.GPIO2,
        gpio::Level::Low,
        gpio::OutputConfig::default(),
    );
    if let Ok(token) =
        outputs::display::display_task(spi, cs, dc, reset, peripherals.GPIO22, peripherals.LEDC)
    {
        spawner.spawn(token);
    } else {
        error!("Failed to create display task")
    }

    loop {
        Timer::after(Duration::from_secs(1)).await;
        built_in_led.toggle();
    }
}
