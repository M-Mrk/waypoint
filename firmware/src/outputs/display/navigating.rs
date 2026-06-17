use crate::power::BatteryState;
use defmt::error;
use heapless::{String, format};

use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Alignment::Center, Text},
};
use profont::{PROFONT_12_POINT, PROFONT_18_POINT, PROFONT_24_POINT};

use super::FrameBuffer;
use super::widgets::battery;

#[derive(Clone, PartialEq)]
pub struct State {
    pub waypoint_name: String<15>,
    pub latitude: f64,
    pub longitude: f64,
    pub distance: f64,
    pub height_delta: i32,
    pub battery: BatteryState,
}

pub async fn draw(display: &mut FrameBuffer, state: &State) {
    if battery(display, &state.battery).is_err() {
        error!("Failed to draw battery")
    }
    if waypoint(display, &state).is_err() {
        error!("Failed to draw waypoint")
    }
    if waypoint_details(display, &state).is_err() {
        error!("Failed to draw waypoint details")
    }
}

fn waypoint(display: &mut FrameBuffer, state: &State) -> Result<(), ()> {
    let major_text_style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::CSS_BLANCHED_ALMOND);
    let minor_text_style = MonoTextStyle::new(&PROFONT_12_POINT, Rgb565::CSS_GRAY);

    Text::with_alignment(
        &state.waypoint_name,
        Point { x: 120, y: 70 },
        major_text_style,
        Center,
    )
    .draw(display)
    .map_err(|_| ())?;

    let text_lat: String<8> = format!("{}º", &state.latitude).map_err(|_| ())?;
    Text::with_alignment(&text_lat, Point { x: 120, y: 85 }, minor_text_style, Center)
        .draw(display)
        .map_err(|_| ())?;

    let text_long: String<8> = format!("{}º", &state.longitude).map_err(|_| ())?;
    Text::with_alignment(
        &text_long,
        Point { x: 120, y: 100 },
        minor_text_style,
        Center,
    )
    .draw(display)
    .map_err(|_| ())?;

    Ok(())
}

fn waypoint_details(display: &mut FrameBuffer, state: &State) -> Result<(), ()> {
    let text_style = MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::WHITE);

    let distance: String<7> = match state.distance {
        s if s < 10000_f64 => format!("{}m", state.distance).map_err(|_| ())?,
        _ => format!("{}km", state.distance / 1000_f64).map_err(|_| ())?,
    };
    Text::with_alignment(&distance, Point { x: 120, y: 150 }, text_style, Center)
        .draw(display)
        .map_err(|_| ())?;

    let (color, pre_fix) = match state.distance {
        b if b > 0_f64 => (Rgb565::CSS_LIGHT_GREEN, "+"),
        _ => (Rgb565::CSS_CRIMSON, ""),
    };
    let text: String<10> = format!("{}{}m", pre_fix, state.height_delta).map_err(|_| ())?;
    Text::with_alignment(
        &text,
        Point { x: 120, y: 170 },
        MonoTextStyleBuilder::new()
            .font(&PROFONT_18_POINT)
            .text_color(color)
            .build(),
        Center,
    )
    .draw(display)
    .map_err(|_| ())?;

    Ok(())
}
