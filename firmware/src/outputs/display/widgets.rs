use crate::{
    outputs::display::{SML_STYLE},
    power::BatteryState,
};
use embedded_graphics::{
    mono_font::{self, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{self, PrimitiveStyle},
    text::{Alignment, Text},
};

use heapless::{String, format};

use super::{FrameBuffer, STD_STYLE};

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

pub fn menu(display: &mut FrameBuffer, options: &[&str], selected: usize, highlight_color: Rgb565) {
    let item_height = 32u32;
    let box_width = 160u32;
    let center_x = 120i32;
    let start_y = 70i32;

    for (index, item) in options.iter().enumerate() {
        let y = start_y + (index as i32 * item_height as i32);
        let x = center_x - (box_width as i32 / 2);

        let item_rect =
            primitives::Rectangle::new(Point::new(x, y), Size::new(box_width, item_height));

        // Draw background highlight if selected
        if index == selected {
            item_rect
                .into_styled(PrimitiveStyle::with_fill(highlight_color))
                .draw(display)
                .unwrap();
        } else {
            item_rect
                .into_styled(PrimitiveStyle::with_stroke(Rgb565::WHITE, 1))
                .draw(display)
                .unwrap();
        }

        // Draw text
        let text_color = if index == selected {
            Rgb565::BLACK
        } else {
            Rgb565::WHITE
        };
        let text_style = MonoTextStyle::new(SML_STYLE.font, text_color);

        Text::with_alignment(
            item,
            Point::new(center_x, y + (item_height as i32 / 2) + 4),
            text_style,
            Alignment::Center,
        )
        .draw(display)
        .unwrap();
    }
}

pub fn select_box(
    display: &mut FrameBuffer,
    options: &[&str],
    selected_index: usize,
    highlight_color: Rgb565,
    background_color: Rgb565,
) {
    let item_height = 28u32;
    let box_width = 140u32;
    let center_x = 120i32;
    let start_y = 72i32;

    let header_y = start_y - item_height as i32;
    let header_rect = primitives::Rectangle::new(
        Point::new(center_x - (box_width as i32 / 2), header_y),
        Size::new(box_width, item_height),
    );

    header_rect
        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_GOLDENROD))
        .draw(display)
        .unwrap();
    header_rect
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::CSS_PALE_GOLDENROD, 1))
        .draw(display)
        .unwrap();

    Text::with_alignment(
        "Select",
        Point::new(center_x, header_y + (item_height as i32 / 2) + 3),
        SML_STYLE,
        Alignment::Center,
    )
    .draw(display)
    .unwrap();

    for (index, option) in options.iter().enumerate() {
        let y = start_y + (index as i32 * item_height as i32);
        let x = center_x - (box_width as i32 / 2);
        let is_selected = index + 1 == selected_index;

        let item_rect =
            primitives::Rectangle::new(Point::new(x, y), Size::new(box_width, item_height));

        // Draw background highlight if selected
        if is_selected {
            item_rect
                .into_styled(PrimitiveStyle::with_fill(highlight_color))
                .draw(display)
                .unwrap();
        } else {
            item_rect
                .into_styled(PrimitiveStyle::with_fill(background_color))
                .draw(display)
                .unwrap();
            item_rect
                .into_styled(PrimitiveStyle::with_stroke(Rgb565::WHITE, 1))
                .draw(display)
                .unwrap();
        }

        // Draw text
        let text_color = if is_selected {
            Rgb565::BLACK
        } else {
            Rgb565::WHITE
        };
        let text_style = MonoTextStyle::new(SML_STYLE.font, text_color);

        Text::with_alignment(
            option,
            Point::new(center_x, y + (item_height as i32 / 2) + 3),
            text_style,
            Alignment::Center,
        )
        .draw(display)
        .unwrap();
    }
}

pub fn message_box(
    display: &mut FrameBuffer,
    message: &str,
    bg_color: Rgb565,
    border_color: Rgb565,
) {
    let width = 180u32;
    let height = 70u32;
    let center_x = 120i32;
    let center_y = 120i32;
    let font_y_adjustment = 7i32;

    let top_left = Point::new(
        center_x - (width as i32 / 2),
        center_y - (height as i32 / 2),
    );

    // Draw background
    primitives::Rectangle::new(top_left, Size::new(width, height))
        .into_styled(PrimitiveStyle::with_fill(bg_color))
        .draw(display)
        .unwrap();

    // Draw border
    primitives::Rectangle::new(top_left, Size::new(width, height))
        .into_styled(PrimitiveStyle::with_stroke(border_color, 2))
        .draw(display)
        .unwrap();

    // Draw text (centered both horizontally and vertically)
    Text::with_alignment(
        message,
        Point::new(center_x, center_y + font_y_adjustment),
        STD_STYLE,
        Alignment::Center,
    )
    .draw(display)
    .unwrap();
}
