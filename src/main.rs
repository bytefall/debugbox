use iced::{Sandbox, Settings};

use crate::gui::DebugBox;

mod gui;

fn main() -> iced::Result {
	DebugBox::run(Settings::default())
}
