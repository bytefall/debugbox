use anyhow::{anyhow, Error};
use std::rc::Rc;
use zi::{
	components::text::{Text, TextAlign, TextProperties},
	prelude::*,
};

use crate::{bus::Proxy, x86::Address};

const BYTES_PER_LINE: usize = 16;

pub struct Data {
	props: Properties,
	frame: Rect,
	error: Option<Error>,
	data: Vec<u8>,
}

#[derive(Clone)]
pub struct Properties {
	pub proxy: Rc<Proxy>,
	pub attached: bool,
	pub addr: Address,
}

impl PartialEq for Properties {
	fn eq(&self, other: &Properties) -> bool {
		self.attached == other.attached && self.addr == other.addr
	}
}

impl Component for Data {
	type Message = ();
	type Properties = Properties;

	fn create(props: Self::Properties, frame: Rect, _: ComponentLink<Self>) -> Self {
		let (data, error) = if !props.attached {
			(Vec::new(), Some(anyhow!("Not attached.")))
		} else {
			match props.proxy.mem.get(props.addr.segment, props.addr.offset, 1024) {
				Ok(c) => (c, None),
				Err(e) => (Vec::new(), Some(e.into())),
			}
		};

		Self {
			props,
			frame,
			error,
			data,
		}
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		if self.props != props {
			self.props = props;

			true
		} else {
			false
		}
		.into()
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

		for (y, bytes) in self.data.chunks(BYTES_PER_LINE).enumerate() {
			let style = super::STYLE;

			canvas.draw_str(0, y, style, &format!("{:04X}", self.props.addr.segment));
			canvas.draw_str(
				6,
				y,
				style,
				&format!("{:04X}", self.props.addr.offset as usize + y * BYTES_PER_LINE),
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
				&bytes
					.iter()
					.fold(String::new(), |a, x| format!("{a}{} ", char::from(*x))),
			);
		}

		canvas.into()
	}
}
