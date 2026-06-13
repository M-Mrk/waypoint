use defmt::error;

use crate::power::BatteryState;

use super::FrameBuffer;
use embedded_graphics::{
    image::Image,
    mono_font::MonoTextStyle,
    mono_font::ascii::FONT_9X15_BOLD,
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Line, PrimitiveStyle},
    text::{Alignment, Text},
};
use tinybmp::Bmp;

use super::battery;
#[derive(Clone, Copy, PartialEq)]
pub enum Item {
    Navigation,
    Waypoints,
    Settings,
}

#[derive(Clone, Copy, PartialEq)]
pub struct State {
    pub current_item: Item,
    pub battery: BatteryState,
}

pub async fn draw(display: &mut FrameBuffer, state: &State) {
    if battery(display, &state.battery).is_err() {
        error!("Failed to draw battery")
    }
    if arrows(display).is_err() {
        error!("Failed to draw arrows")
    }
    if item(display, state).is_err() {
        error!("Failed to draw item")
    }
}

fn arrows(display: &mut FrameBuffer) -> Result<(), ()> {
    const COLOR: Rgb565 = Rgb565::CSS_CRIMSON;
    const ARROW_Y: i32 = 120;
    const ARROW_LEFT_X: i32 = 5;
    const ARROW_RIGHT_X: i32 = 240 - ARROW_LEFT_X;
    const ARROW_LEFT_MIDDLE: Point = Point {
        x: ARROW_LEFT_X,
        y: ARROW_Y,
    };
    const ARROW_RIGHT_MIDDLE: Point = Point {
        x: ARROW_RIGHT_X,
        y: ARROW_Y,
    };
    const ARROW_X_CHANGE: i32 = 15;
    const ARROW_Y_CHANGE: i32 = 45;

    // Left arrow
    Line::new(
        ARROW_LEFT_MIDDLE,
        Point {
            x: ARROW_LEFT_X + ARROW_X_CHANGE,
            y: ARROW_Y + ARROW_Y_CHANGE,
        },
    )
    .into_styled(PrimitiveStyle::with_stroke(COLOR, 3))
    .draw(display)
    .map_err(|_| ())?;
    Line::new(
        ARROW_LEFT_MIDDLE,
        Point {
            x: ARROW_LEFT_X + ARROW_X_CHANGE,
            y: ARROW_Y - ARROW_Y_CHANGE,
        },
    )
    .into_styled(PrimitiveStyle::with_stroke(COLOR, 3))
    .draw(display)
    .map_err(|_| ())?;

    // Right arrow
    Line::new(
        ARROW_RIGHT_MIDDLE,
        Point {
            x: ARROW_RIGHT_X - ARROW_X_CHANGE,
            y: ARROW_Y + ARROW_Y_CHANGE,
        },
    )
    .into_styled(PrimitiveStyle::with_stroke(COLOR, 3))
    .draw(display)
    .map_err(|_| ())?;
    Line::new(
        ARROW_RIGHT_MIDDLE,
        Point {
            x: ARROW_RIGHT_X - ARROW_X_CHANGE,
            y: ARROW_Y - ARROW_Y_CHANGE,
        },
    )
    .into_styled(PrimitiveStyle::with_stroke(COLOR, 3))
    .draw(display)
    .map_err(|_| ())?;

    Ok(())
}

fn item(display: &mut FrameBuffer, state: &State) -> Result<(), ()> {
    let text_style = MonoTextStyle::new(&FONT_9X15_BOLD, Rgb565::WHITE);

    let (bmp_data, name) = match state.current_item {
        Item::Navigation => (include_bytes!("../../../img/navigate.bmp"), "Navigate"),
        Item::Settings => (include_bytes!("../../../img/cog.bmp"), "Settings"),
        Item::Waypoints => (include_bytes!("../../../img/flag.bmp"), "Waypoints"),
    };

    let bmp = Bmp::from_slice(bmp_data).unwrap();
    let image = Image::new(&bmp, Point { x: 60, y: 40 });
    image.draw(display);

    Text::with_alignment(
        &name,
        Point { x: 120, y: 180 },
        text_style,
        Alignment::Center,
    )
    .draw(display)
    .map_err(|_| ())?;

    Ok(())
}
