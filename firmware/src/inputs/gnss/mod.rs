use defmt::{error, info, warn};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use esp_hal::uart;
use nmea::{Nmea, SentenceType};

#[derive(Clone, Copy, defmt::Format)]
pub struct GnssData {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude_m: Option<f32>,
    pub speed_knots: Option<f32>,
    pub course_deg: Option<f32>,
    pub satellites: Option<u32>,
    pub fix_valid: bool,
}

impl GnssData {
    pub const fn none() -> Self {
        Self {
            latitude: None,
            longitude: None,
            altitude_m: None,
            speed_knots: None,
            course_deg: None,
            satellites: None,
            fix_valid: false,
        }
    }

    pub fn has_valid_fix(&self) -> bool {
        self.fix_valid && self.latitude.is_some() && self.longitude.is_some()
    }
}

pub static GNSS_WATCH: Watch<CriticalSectionRawMutex, GnssData, 3> = Watch::new();

#[embassy_executor::task]
pub async fn gnss_task(mut gnss_uart: uart::Uart<'static, esp_hal::Async>) {
    let mut nm = Nmea::create_for_navigation(&[SentenceType::GGA, SentenceType::RMC]).unwrap();

    let sender = GNSS_WATCH.sender();
    sender.send(GnssData::none());

    let mut line_buf = [0u8; 128];
    let mut pos = 0usize;

    loop {
        let mut byte = [0u8; 1];
        match gnss_uart.read_async(&mut byte).await {
            Err(err) => {
                error!("UART read error: {}", err);
                continue;
            }
            Ok(_) => {}
        }

        match byte[0] {
            b'\r' => {}

            b'\n' if pos > 0 => {
                if let Ok(sentence) = core::str::from_utf8(&line_buf[..pos]) {
                    match nm.parse(sentence) {
                        Ok(_) => {
                            let data = GnssData {
                                latitude: nm.latitude,
                                longitude: nm.longitude,
                                altitude_m: nm.altitude,
                                speed_knots: nm.speed_over_ground,
                                course_deg: nm.true_course,
                                satellites: nm.num_of_fix_satellites,
                                // is_valid() direkt auf dem FixType nutzen
                                fix_valid: nm.fix_type.map(|f| f.is_valid()).unwrap_or(false),
                            };
                            info!(
                                "GNSS valid={} lat={} lon={} sats={}",
                                data.fix_valid,
                                data.latitude.unwrap_or(0.0),
                                data.longitude.unwrap_or(0.0),
                                data.satellites.unwrap_or(0),
                            );
                            sender.send(data);
                        }
                        Err(_) => warn!("Unparseable NMEA sentence"),
                    }
                }
                pos = 0;
            }

            b if pos < line_buf.len() - 1 => {
                line_buf[pos] = b;
                pos += 1;
            }

            _ => {
                warn!("NMEA line buffer overflow");
                pos = 0;
            }
        }
    }
}
