use anyhow::{anyhow, bail, Result};
use capstone::prelude::*;
use std::rc::Rc;
use zbus::blocking::Connection;
use zi::{
	components::text::{Text, TextAlign, TextProperties},
	prelude::*,
};

const FG_EIP: Colour = Colour::rgb(139, 233, 253);
const STYLE_EIP: Style = Style::normal(super::BG_DARK, FG_EIP);

const BYTES_TO_ROWS_RATIO: u32 = 3;

pub struct Properties {
	pub conn: Rc<Connection>,
	pub attached: bool,
	pub cs: u16,
	pub eip: u32,
}

impl PartialEq for Properties {
	fn eq(&self, other: &Properties) -> bool {
		self.attached == other.attached && self.cs == other.cs && self.eip == other.eip
	}
}

pub struct Row {
	offset: u16,
	data: Vec<u8>,
	mnemonic: Option<String>,
	operands: Option<String>,
}

pub struct Code {
	props: Properties,
	frame: Rect,
	capstone: Capstone,
	code: Result<Vec<Row>>,
}

impl Code {
	fn load(&self) -> Result<Vec<Row>> {
		if !self.props.attached {
			bail!("Not attached.");
		}

		let mut code = Vec::new();
		let mut start = self.props.eip;

		while code.len() < self.frame.size.height {
			let limit = self.frame.size.height.saturating_sub(code.len()) as u32 * BYTES_TO_ROWS_RATIO;

			let data: Vec<u8> = self
				.props
				.conn
				.call_method(
					Some("com.dosbox"),
					"/mem",
					Some("com.dosbox"),
					"get",
					&(self.props.cs, start, limit),
				)?
				.body_unchecked()?;

			code.extend(
				self.capstone
					.disasm_all(&data, start.into())
					.map_err(|e| anyhow!("{e}"))?
					.iter()
					.map(|i| Row {
						offset: i.address() as u16,
						data: i.bytes().to_vec(),
						mnemonic: i.mnemonic().map(String::from),
						operands: i.op_str().map(String::from),
					}),
			);

			start += limit;
		}

		Ok(code)
	}
}

impl Component for Code {
	type Message = ();
	type Properties = Properties;

	fn create(props: Self::Properties, frame: Rect, _: ComponentLink<Self>) -> Self {
		let capstone = Capstone::new()
			.x86()
			.mode(arch::x86::ArchMode::Mode16)
			.syntax(arch::x86::ArchSyntax::Intel)
			.detail(true)
			.build()
			.unwrap();

		let mut this = Self {
			props,
			frame,
			capstone,
			code: Err(anyhow!("Not loaded")),
		};

		this.code = this.load();
		this
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		if self.props != props {
			self.props = props;

			if self.props.attached {
				self.code = self.load();
			}

			true
		} else {
			false
		}
		.into()
	}

	fn view(&self) -> Layout {
		let code = match &self.code {
			Ok(c) => c,
			Err(e) => {
				return Text::with(
					TextProperties::new()
						.style(super::STYLE)
						.align(TextAlign::Centre)
						.content(e.to_string()),
				);
			}
		};

		let mut canvas = Canvas::new(self.frame.size);
		canvas.clear(super::STYLE);

		for (y, row) in code.iter().take(self.frame.size.height).enumerate() {
			let style = if row.offset as u32 == self.props.eip {
				STYLE_EIP
			} else {
				super::STYLE
			};

			canvas.draw_str(0, y, style, &format!("{:04X}", self.props.cs));
			canvas.draw_str(6, y, style, &format!("{:04X}", row.offset));
			canvas.draw_str(
				12,
				y,
				style,
				&row.data.iter().fold(String::new(), |a, x| format!("{a}{x:02X}")),
			);

			if let Some(mn) = &row.mnemonic {
				canvas.draw_str(30, y, style, mn);

				if let Some(op) = &row.operands {
					canvas.draw_str(42, y, style, op);
				}
			}
		}

		canvas.into()
	}
}
