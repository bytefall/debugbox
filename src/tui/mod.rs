use zi::{Colour, Style};

pub mod code;
pub mod data;
pub mod debugbox;
pub mod registers;
pub mod status_bar;

const BG_GRAY: Colour = Colour::rgb(33, 34, 44);
const BG_DARK: Colour = Colour::rgb(14, 20, 25);
const FG_GRAY: Colour = Colour::rgb(248, 248, 242);
const STYLE: Style = Style::normal(BG_DARK, FG_GRAY);
const STYLE_SEL: Style = Style::normal(BG_GRAY, FG_GRAY);

#[derive(Clone, PartialEq, Eq)]
pub struct PaneStatus {
	pub attached: bool,
	pub focused: bool,
	pub reload: bool,
}
