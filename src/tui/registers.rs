use zi::{
	components::text::{Text, TextProperties},
	prelude::*,
};

use crate::cpu::Registers as Regs;

pub struct Registers {
	regs: Regs,
}

impl Component for Registers {
	type Message = ();
	type Properties = Regs;

	fn create(regs: Self::Properties, _: Rect, _: ComponentLink<Self>) -> Self {
		Self { regs }
	}

	fn change(&mut self, regs: Self::Properties) -> ShouldRender {
		if self.regs != regs {
			self.regs = regs;

			true
		} else {
			false
		}
		.into()
	}

	fn view(&self) -> Layout {
		let r = self.regs;

		let regs = [
			[("EAX", r.eax), ("EBX", r.ebx), ("ECX", r.ecx), ("EDX", r.edx)],
			[("ESI", r.esi), ("EDI", r.edi), ("EBP", r.ebp), ("ESP", r.esp)],
			[
				("CS", r.cs.into()),
				("DS", r.ds.into()),
				("ES", r.es.into()),
				("SS", r.ss.into()),
			],
		];

		Layout::row(regs.map(|col| {
			Item::auto(Layout::column(col.map(|(name, value)| {
				Item::fixed(1)(Text::with_key(
					name,
					TextProperties::new().style(super::STYLE).content(if name.len() == 2 {
						format!("{name}={value:04X}")
					} else {
						format!("{name}={value:08X}")
					}),
				))
			})))
		}))
	}
}
