use zi::prelude::*;

use crate::cpu::Registers as Regs;

pub struct Registers {
	regs: Regs,
	frame: Rect,
}

impl Component for Registers {
	type Message = ();
	type Properties = Regs;

	fn create(regs: Self::Properties, frame: Rect, _: ComponentLink<Self>) -> Self {
		Self { regs, frame }
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

		let mut canvas = Canvas::new(self.frame.size);
		canvas.clear(super::STYLE);

		let col_width = self.frame.size.width / regs.len();

		for (x, col) in regs.iter().enumerate() {
			for (y, (name, value)) in col.iter().enumerate() {
				canvas.draw_str(
					x * col_width,
					y,
					super::STYLE,
					&if name.len() == 2 {
						format!("{name}={value:04X}")
					} else {
						format!("{name}={value:08X}")
					},
				);
			}
		}

		canvas.into()
	}
}
