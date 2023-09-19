use zi::{
	components::text::{Text, TextAlign, TextProperties},
	prelude::*,
};

pub struct Properties {
	pub attached: bool,
	pub cs: u16,
	pub eip: u32,
}

pub struct Code {
	attached: bool,
	cs: u16,
	eip: u32,
}

impl Component for Code {
	type Message = ();
	type Properties = Properties;

	fn create(props: Self::Properties, _: Rect, _: ComponentLink<Self>) -> Self {
		let Self::Properties { attached, cs, eip } = props;

		Self { attached, cs, eip }
	}

	fn view(&self) -> Layout {
		let tc = TextProperties::new().style(super::STYLE).align(TextAlign::Centre);
		let tc = if self.attached {
			tc.content(format!("{:04X}:{:08X}", self.cs, self.eip))
		} else {
			tc.content("This is a code")
		};

		Text::with_key("code", tc).into()
	}
}
