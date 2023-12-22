use anyhow::{anyhow, Error};
use iced_x86::{Formatter, Instruction, IntelFormatter};
use std::rc::Rc;
use zi::{
	components::text::{Text, TextAlign, TextProperties},
	prelude::*,
};

use crate::{
	bus::Proxy,
	x86::dec::{fetch_after, fetch_before},
};

const FG_EIP: Colour = Colour::rgb(139, 233, 253);
const STYLE_EIP: Style = Style::normal(super::BG_DARK, FG_EIP);
const STYLE_SEL: Style = Style::normal(super::BG_GRAY, super::FG_GRAY);

pub struct Code {
	props: Properties,
	frame: Rect,
	error: Option<Error>,
	code: Vec<(Instruction, Vec<u8>)>,
	skip: usize,
	pos: Option<usize>,
}

pub enum Message {
	Up,
	Down,
	Enter,
	Escape,
}

#[derive(Clone)]
pub struct Properties {
	pub proxy: Rc<Proxy>,
	pub attached: bool,
	pub focused: bool,
	pub cs: u16,
	pub eip: u32,
}

impl PartialEq for Properties {
	fn eq(&self, other: &Properties) -> bool {
		self.attached == other.attached && self.focused == other.focused && self.cs == other.cs && self.eip == other.eip
	}
}

impl Component for Code {
	type Message = Message;
	type Properties = Properties;

	fn create(props: Self::Properties, frame: Rect, _: ComponentLink<Self>) -> Self {
		let (code, error) = if !props.attached {
			(Vec::new(), Some(anyhow!("Not attached.")))
		} else {
			match fetch_after(&props.proxy, (props.cs, props.eip).into(), frame.size.height) {
				Ok(c) => (c, None),
				Err(e) => (Vec::new(), Some(e)),
			}
		};

		Self {
			props,
			frame,
			error,
			code,
			skip: 0,
			pos: None,
		}
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		if self.props != props {
			self.props = props;

			if self.props.attached && self.code.is_empty() {
				match fetch_after(
					&self.props.proxy,
					(self.props.cs, self.props.eip).into(),
					self.frame.size.height,
				) {
					Ok(c) => self.code = c,
					Err(e) => self.error = Some(e),
				}
			}

			true
		} else {
			false
		}
		.into()
	}

	fn update(&mut self, message: Self::Message) -> ShouldRender {
		if !self.props.attached {
			return false.into();
		}

		let prev = (self.skip, self.pos, self.code.len());

		match message {
			Message::Up if self.pos.is_some_and(|x| x > 0) => {
				if let Some(pos) = self.pos.as_mut() {
					*pos -= 1;
				}
			}
			Message::Up if self.skip > 0 => {
				self.skip -= 1;
			}
			Message::Up => {
				if let Some(offset) = self.code.first().map(|(i, _)| i.ip32()) {
					match fetch_before(
						&self.props.proxy,
						(self.props.cs, offset).into(),
						self.frame.size.height,
					) {
						Ok(mut c) if !c.is_empty() => {
							self.skip = c.len().saturating_sub(1);
							c.append(&mut self.code);
							self.code = c;
						}
						Ok(_) => (),
						Err(e) => self.error = Some(e),
					}
				}
			}
			Message::Down if self.pos.is_some_and(|x| x < self.frame.height() - 1) => {
				if let Some(pos) = self.pos.as_mut() {
					*pos += 1;
				}
			}
			Message::Down if self.skip + self.pos.unwrap_or(self.frame.height()) < self.code.len() - 1 => {
				self.skip += 1;
			}
			Message::Down => {
				if let Some(offset) = self.code.last().map(|(i, _)| i.next_ip32()) {
					match fetch_after(
						&self.props.proxy,
						(self.props.cs, offset).into(),
						self.frame.size.height,
					) {
						Ok(c) if !c.is_empty() => {
							self.skip += 1;
							self.code.extend(c)
						}
						Ok(_) => (),
						Err(e) => self.error = Some(e),
					}
				}
			}
			Message::Enter => {
				if self.pos.is_none() {
					self.pos = Some(0);
				}
			}
			Message::Escape => {
				if self.pos.is_some() {
					self.pos = None;
				}
			}
		}

		((self.skip, self.pos, self.code.len()) != prev).into()
	}

	fn bindings(&self, bindings: &mut Bindings<Self>) {
		bindings.set_focus(self.props.focused);

		if !bindings.is_empty() {
			return;
		}

		bindings.command("up", || Message::Up).with([Key::Up]);
		bindings.command("down", || Message::Down).with([Key::Down]);
		bindings.command("enter", || Message::Enter).with([Key::Char('\n')]);
		bindings.command("escape", || Message::Escape).with([Key::Esc]);
	}

	fn view(&self) -> Layout {
		if let Some(e) = &self.error {
			return Text::with(
				TextProperties::new()
					.style(super::STYLE)
					.align(TextAlign::Centre)
					.content(e.to_string()),
			);
		}

		let mut canvas = Canvas::new(self.frame.size);
		canvas.clear(super::STYLE);

		let mut fmt = IntelFormatter::new();
		fmt.options_mut().set_space_after_operand_separator(true);
		let mut out = String::new();

		for (y, (ins, data)) in self
			.code
			.iter()
			.skip(self.skip)
			.take(self.frame.size.height)
			.enumerate()
		{
			let mut style = if ins.ip32() == self.props.eip {
				STYLE_EIP
			} else {
				super::STYLE
			};

			if self.props.attached && self.pos == Some(y) {
				style.background = STYLE_SEL.background;

				canvas.clear_region(
					Rect::new(Position::new(0, y), Size::new(self.frame.size.width, 1)),
					style,
				);
			}

			canvas.draw_str(0, y, style, &format!("{:04X}", self.props.cs));
			canvas.draw_str(6, y, style, &format!("{:04X}", ins.ip16()));
			canvas.draw_str(
				12,
				y,
				style,
				&data.iter().fold(String::new(), |a, x| format!("{a}{x:02X}")),
			);

			out.clear();
			fmt.format_mnemonic(ins, &mut out);
			canvas.draw_str(30, y, style, &out);

			if ins.op_count() > 0 {
				out.clear();
				fmt.format_all_operands(ins, &mut out);
				canvas.draw_str(42, y, style, &out);
			}
		}

		canvas.into()
	}
}
