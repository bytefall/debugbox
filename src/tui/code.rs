use anyhow::{anyhow, Error};
use iced_x86::{Formatter, Instruction, IntelFormatter};
use std::rc::Rc;
use zi::{
	components::text::{Text, TextAlign, TextProperties},
	prelude::*,
};

use crate::{
	bus::Proxy,
	tui::PaneStatus,
	x86::{
		dec::{fetch_after, fetch_before},
		Address,
	},
};

#[derive(Clone)]
pub struct Properties {
	pub status: PaneStatus,
	pub proxy: Rc<Proxy>,
	pub addr: Address,
}

impl PartialEq for Properties {
	fn eq(&self, other: &Properties) -> bool {
		self.status == other.status && self.addr == other.addr
	}
}

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

impl Component for Code {
	type Message = Message;
	type Properties = Properties;

	fn create(props: Self::Properties, frame: Rect, _: ComponentLink<Self>) -> Self {
		let (code, error) = if !props.status.attached {
			(Vec::new(), Some(anyhow!("Not attached.")))
		} else {
			match fetch_after(&props.proxy, props.addr, frame.size.height) {
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
		if self.props == props {
			return false.into();
		}

		if !props.status.attached {
			self.props = props;
			return true.into();
		}

		const BOTTOM_PADDING: usize = 4; // number of extra rows on the botoom
		let limit = self.frame.height();
		let pad = limit.saturating_sub(BOTTOM_PADDING);

		let start = self
			.code
			.iter()
			.skip(self.skip)
			.take(limit)
			.enumerate()
			.map(|(i, (ins, _))| (i, ins.ip32()))
			.find_map(|(i, ip)| {
				if ip == props.addr.offset {
					self.code.get(self.skip + i.saturating_sub(pad)).map(|(x, _)| x.ip32())
				} else {
					None
				}
			})
			.unwrap_or(props.addr.offset);

		match fetch_after(&props.proxy, (props.addr.segment, start).into(), limit) {
			Ok(c) => {
				self.code = c;
				self.skip = 0;
				self.pos = None;
				self.error = None;
			}
			Err(e) => self.error = Some(e),
		}

		self.props = props;

		true.into()
	}

	fn update(&mut self, message: Self::Message) -> ShouldRender {
		if !self.props.status.attached {
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
						(self.props.addr.segment, offset).into(),
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
			Message::Down
				if self
					.code
					.len()
					.saturating_sub(self.skip)
					.saturating_sub(self.pos.unwrap_or(self.frame.height()))
					> 0 =>
			{
				self.skip += 1;
			}
			Message::Down => {
				if let Some(offset) = self.code.last().map(|(i, _)| i.next_ip32()) {
					match fetch_after(
						&self.props.proxy,
						(self.props.addr.segment, offset).into(),
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
		bindings.set_focus(self.props.status.focused);

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
					.style(super::ST_NORMAL)
					.align(TextAlign::Centre)
					.content(e.to_string()),
			);
		}

		let mut canvas = Canvas::new(self.frame.size);
		canvas.clear(super::ST_NORMAL);

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
			let mut style = if ins.ip32() == self.props.addr.offset {
				super::ST_ACTIVE
			} else {
				super::ST_NORMAL
			};

			if self.props.status.attached && self.pos == Some(y) {
				style.background = super::ST_SELECTED.background;

				canvas.clear_region(
					Rect::new(Position::new(0, y), Size::new(self.frame.size.width, 1)),
					style,
				);
			}

			canvas.draw_str(0, y, style, &format!("{:04X}", self.props.addr.segment));
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
