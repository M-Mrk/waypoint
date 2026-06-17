use defmt::{error};
use embedded_graphics::pixelcolor::{Rgb565, WebColors};
use heapless::{String, Vec};
use crate::data::waypoints::{MAX_NAME_LENGTH, MAX_WAYPOINTS};

use super::FrameBuffer;
use super::widgets::{menu, message_box, select_box};

#[derive(Clone, PartialEq, Eq)]
pub struct State {
    pub main_selection: usize,
    pub result_box: Option<String<15>>,

    pub select_open: bool,
    pub select_options: Vec<String<MAX_NAME_LENGTH>, MAX_WAYPOINTS>,
    pub select_selection: usize,
}
impl State {
    fn validate(&self) -> Result<(), ()> {
        if self.main_selection > 2 {
            error!("main_selection is bigger than options");
            return Err(());
        }

        Ok(())
    }
}

pub async fn draw(display: &mut FrameBuffer, state: &State) {
    if state.validate().is_err() {
        return;
    }
    draw_main_menu(display, state);
    draw_result(display, state);
    draw_select(display, state);
}

fn draw_main_menu(display: &mut FrameBuffer, state: &State) {
    let options = ["New from location", "Delete", "Back"];
    menu(display, &options, state.main_selection, Rgb565::CSS_CRIMSON);
}

fn draw_result(display: &mut FrameBuffer, state: &State) {
    match &state.result_box {
        Some(message) => {
            message_box(display, message, Rgb565::CSS_GRAY, Rgb565::CSS_CRIMSON);
        }
        None => {}
    }
}

fn draw_select(display: &mut FrameBuffer, state: &State) {
    if !state.select_open {
        return;
    }

    let options: Vec<&str, 5> = state.select_options.iter().map(|s| s.as_str()).collect();
    select_box(
        display,
        &options,
        state.select_selection,
        Rgb565::CSS_CRIMSON,
        Rgb565::CSS_GRAY,
    );
}

