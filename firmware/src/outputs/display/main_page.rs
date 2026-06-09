use defmt::error;
use heapless::{String, format};

use super::FrameBuffer;
use embedded_graphics::{
    mono_font::MonoTextStyle,
    mono_font::ascii::FONT_9X15_BOLD,
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
    image::Image,
};
use tinybmp::Bmp;

#[derive(Clone, Copy)]
pub enum Item {
    Navigation,
    Waypoints,
    Settings,
}

#[derive(Clone, Copy)]
pub struct State {
    pub current_item: Item,
    pub bat_percent: u8,
    pub charging: bool,
}

fn map(x: u32, in_min: u32, in_max: u32, out_min: u32, out_max: u32) -> u32 {
    return (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min;
}

pub async fn draw(display: &mut FrameBuffer, state: &State) {
    if battery(display, state).is_err() {
        error!("Failed to draw battery")
    }
    if arrows(display).is_err() {
        error!("Failed to draw arrows")
    }
    if item(display, state).is_err() {
        error!("Failed to draw item")
    }
}

fn battery(display: &mut FrameBuffer, state: &State) -> Result<(), ()> {
    // Battery
    const BAT_LEFT_X: i32 = 41;
    const BAT_HEIGHT: i32 = 30;
    let text_style = MonoTextStyle::new(&FONT_9X15_BOLD, Rgb565::WHITE);
    const HALF_TEXT_HEIGHT: i32 = 5;

    // background
    Rectangle::new(
        Point {
            x: BAT_LEFT_X,
            y: 0,
        },
        Size {
            width: (240 - (BAT_LEFT_X * 2)) as u32,
            height: BAT_HEIGHT as u32,
        },
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_GRAY))
    .draw(display);

    // bar
    let percent: String<5> = format!("{}%", state.bat_percent).unwrap();
    let charge_width = map(
        state.bat_percent as u32,
        0,
        100,
        0,
        (240 - BAT_LEFT_X * 2) as u32,
    );
    let mut color = match state.bat_percent {
        low if low < 15 => Rgb565::CSS_RED,
        _ => Rgb565::CSS_DARK_SEA_GREEN,
    };
    if state.charging {
        color = Rgb565::CSS_GOLDENROD;
    }

    Rectangle::new(
        Point {
            x: BAT_LEFT_X,
            y: 0,
        },
        Size {
            width: charge_width,
            height: BAT_HEIGHT as u32,
        },
    )
    .into_styled(PrimitiveStyle::with_fill(color))
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
        Item::Navigation => {
            (include_bytes!("../../../img/navigate.bmp"), "Navigate")
        },
        Item::Settings => {
            (include_bytes!("../../../img/cog.bmp"), "Settings")
        },
        Item::Waypoints => {
            (include_bytes!("../../../img/flag.bmp"), "Waypoints")
        },
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
    .draw(display).map_err(|_|())?;
    
    Ok(())
}