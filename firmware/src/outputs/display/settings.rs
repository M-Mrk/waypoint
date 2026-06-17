use embedded_graphics::pixelcolor::{Rgb565, WebColors};

use super::FrameBuffer;
use super::widgets::message_box;

#[derive(Clone, PartialEq, Eq)]
pub struct State {
}

#[allow(unused)]
pub async fn draw(display: &mut FrameBuffer, state: &State) {
    message_box(display, "Nothing here", Rgb565::CSS_GRAY, Rgb565::CSS_CRIMSON);
}