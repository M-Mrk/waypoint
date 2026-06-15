use heapless::{String, format};
use embedded_graphics::{
    mono_font,
    pixelcolor::Rgb565,
    prelude::*,
    primitives,
    text::{Alignment, Text},
};
use crate::power::BatteryState;

use super::FrameBuffer;

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
