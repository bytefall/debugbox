use zi::{Colour, Style};

pub mod code;
pub mod data;
pub mod debugbox;
pub mod registers;
pub mod status_bar;

const BG_GRAY: Colour = Colour::rgb(33, 34, 44);
const BG_DARK: Colour = Colour::rgb(14, 20, 25);
const FG_GRAY: Colour = Colour::rgb(224, 224, 224);

const ST_NORMAL: Style = Style::normal(BG_DARK, FG_GRAY);
const ST_SELECTED: Style = Style::normal(BG_GRAY, FG_GRAY);
const ST_CAPTION: Style = Style::normal(BG_DARK, Colour::rgb(127, 109, 92));
const ST_CHANGED: Style = Style::normal(BG_DARK, Colour::rgb(170, 170, 255));
const ST_ACTIVE: Style = Style::normal(BG_DARK, Colour::rgb(255, 0, 127));

#[derive(Clone, PartialEq, Eq)]
pub struct PaneStatus {
	pub attached: bool,
	pub focused: bool,
	pub reload: bool,
}
