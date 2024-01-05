use anyhow::{anyhow, Error};
use std::rc::Rc;
use zi::{
	components::text::{Text, TextAlign, TextProperties},
	prelude::*,
};

use crate::{bus::Proxy, tui::PaneStatus, x86::Address};

const BYTES_PER_LINE: usize = 16;
const NON_ASCII_CHAR: char = '.';

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

pub struct Data {
	props: Properties,
	frame: Rect,
	error: Option<Error>,
	addr: Address,
	data: Vec<u8>,
	skip: usize,
	pos: Option<usize>,
}

pub enum Message {
	Up,
	Down,
	Enter,
	Escape,
}

impl Component for Data {
	type Message = Message;
	type Properties = Properties;

	fn create(props: Self::Properties, frame: Rect, _: ComponentLink<Self>) -> Self {
		let addr = props.addr;

		let (data, error) = if !props.status.attached {
			(Vec::new(), Some(anyhow!("Not attached.")))
		} else {
			match props.proxy.mem.get(addr.segment, addr.offset, bytes_on_screen(&frame)) {
				Ok(c) => (c, None),
				Err(e) => (Vec::new(), Some(e.into())),
			}
		};

		Self {
			props,
			frame,
			error,
			addr,
			data,
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

		let offset = self.addr.offset + (self.skip * BYTES_PER_LINE) as u32;

		match self
			.props
			.proxy
			.mem
			.get(self.addr.segment, offset, bytes_on_screen(&self.frame))
		{
			Ok(d) => {
				self.addr.offset = offset;
				self.data = d;
				self.skip = 0;
				self.pos = None;
				self.error = None;
			}
			Err(e) => self.error = Some(e.into()),
		}

		self.props = props;
		self.props.status.reload = false;

		true.into()
	}

	fn update(&mut self, message: Self::Message) -> ShouldRender {
		if !self.props.status.attached {
			return false.into();
		}

		let prev = (self.skip, self.pos, self.data.len());

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
				let limit = bytes_on_screen(&self.frame);
				let start = self.addr.offset.saturating_sub(limit);

				if start < self.addr.offset {
					match self.props.proxy.mem.get(self.addr.segment, start, limit) {
						Ok(mut d) if !d.is_empty() => {
							self.skip = (d.len() / BYTES_PER_LINE).saturating_sub(1);
							d.append(&mut self.data);
							self.data = d;
							self.addr.offset = start;
						}
						Ok(_) => (),
						Err(e) => self.error = Some(e.into()),
					}
				}
			}
			Message::Down if self.pos.is_some_and(|x| x < self.frame.height() - 1) => {
				if let Some(pos) = self.pos.as_mut() {
					*pos += 1;
				}
			}
			Message::Down
				if (self.data.len() / BYTES_PER_LINE)
					.saturating_sub(self.skip)
					.saturating_sub(self.pos.unwrap_or(self.frame.height()))
					> 0 =>
			{
				self.skip += 1;
			}
			Message::Down => {
				let start = self.addr.offset.saturating_add(self.data.len() as u32);

				match self
					.props
					.proxy
					.mem
					.get(self.addr.segment, start, bytes_on_screen(&self.frame))
				{
					Ok(d) if !d.is_empty() => {
						self.skip += 1;
						self.data.extend(d)
					}
					Ok(_) => (),
					Err(e) => self.error = Some(e.into()),
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

		((self.skip, self.pos, self.data.len()) != prev).into()
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

		for (y, bytes) in self.data.chunks(BYTES_PER_LINE).skip(self.skip).enumerate() {
			let mut style = super::ST_NORMAL;

			if self.pos == Some(y) {
				style.background = super::ST_SELECTED.background;

				canvas.clear_region(
					Rect::new(Position::new(0, y), Size::new(self.frame.size.width, 1)),
					style,
				);
			}

			canvas.draw_str(0, y, style, &format!("{:04X}", self.addr.segment));
			canvas.draw_str(
				6,
				y,
				style,
				&format!("{:04X}", self.addr.offset as usize + (y + self.skip) * BYTES_PER_LINE),
			);
			canvas.draw_str(
				12,
				y,
				style,
				&bytes.iter().fold(String::new(), |a, x| format!("{a}{x:02X} ")),
			);
			canvas.draw_str(
				61,
				y,
				style,
				&bytes.iter().fold(String::new(), |a, x| {
					let c = char::from(*x);
					format!("{a}{}", if matches!(*x, 32..=0x7E) { c } else { NON_ASCII_CHAR })
				}),
			);
		}

		canvas.into()
	}
}

fn bytes_on_screen(rect: &Rect) -> u32 {
	(rect.size.height * BYTES_PER_LINE) as u32
}
