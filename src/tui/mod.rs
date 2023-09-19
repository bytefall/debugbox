use zi::{Colour, Style};

pub mod code;
pub mod debugbox;
pub mod registers;
pub mod status_bar;

const BACKGROUND: Colour = Colour::rgb(33, 34, 44);
const BACKGROUND_DARK: Colour = Colour::rgb(14, 20, 25);
const FOREGROUND: Colour = Colour::rgb(80, 250, 123);
const FOREGROUND_GRAY: Colour = Colour::rgb(248, 248, 242);
const STYLE: Style = Style::bold(BACKGROUND_DARK, FOREGROUND);
