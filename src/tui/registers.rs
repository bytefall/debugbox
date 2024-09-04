use zi::prelude::*;

use crate::bus::Regs;

pub struct Registers {
    regs: Regs,
    prev: Regs,
    frame: Rect,
}

impl Component for Registers {
    type Message = ();
    type Properties = Regs;

    fn create(regs: Self::Properties, frame: Rect, _: ComponentLink<Self>) -> Self {
        Self {
            regs,
            prev: regs,
            frame,
        }
    }

    fn change(&mut self, regs: Self::Properties) -> ShouldRender {
        if self.regs == regs {
            return false.into();
        }

        self.prev = self.regs;
        self.regs = regs;

        true.into()
    }

    fn view(&self) -> Layout {
        let (r, p) = (self.regs, self.prev);

        let regs = [
            [
                ("EAX", r.eax, p.eax),
                ("EBX", r.ebx, p.ebx),
                ("ECX", r.ecx, p.ecx),
                ("EDX", r.edx, p.edx),
            ],
            [
                ("ESI", r.esi, p.esi),
                ("EDI", r.edi, p.edi),
                ("EBP", r.ebp, p.ebp),
                ("ESP", r.esp, p.esp),
            ],
            [
                ("CS", r.cs.into(), p.cs.into()),
                ("DS", r.ds.into(), p.ds.into()),
                ("ES", r.es.into(), p.es.into()),
                ("SS", r.ss.into(), p.ss.into()),
            ],
        ];

        let mut canvas = Canvas::new(self.frame.size);
        canvas.clear(super::ST_NORMAL);

        let col_width = self.frame.size.width / regs.len();

        for (x, col) in regs.iter().enumerate() {
            for (y, (name, value, prev)) in col.iter().enumerate() {
                print_reg(&mut canvas, x * col_width, y, name, *value, *prev);
            }
        }

        let y = regs[0].len() + 1;
        print_reg(&mut canvas, 0, y, "EIP", r.eip, p.eip);
        print_reg(&mut canvas, col_width, y, "FS", r.fs.into(), p.fs.into());
        print_reg(
            &mut canvas,
            col_width * 2,
            y,
            "GS",
            r.gs.into(),
            p.gs.into(),
        );

        let regs = [
            ("C", r.cf, p.cf),
            ("Z", r.zf, p.zf),
            ("S", r.sf, p.sf),
            ("O", r.of, p.of),
            ("A", r.af, p.af),
            ("P", r.pf, p.pf),
            ("D", r.df, p.df),
            ("I", r.r#if, p.r#if),
            ("T", r.tf, p.tf),
        ];

        let y = y + 2;

        for (x, (name, value, prev)) in regs.into_iter().enumerate() {
            let x = x * 4;

            canvas.draw_str(x, y, super::ST_CAPTION, name);
            canvas.draw_str(
                x + 1,
                y,
                if value != prev {
                    super::ST_ACTIVE
                } else if value {
                    super::ST_CHANGED
                } else {
                    super::ST_NORMAL
                },
                if value { "1" } else { "0" },
            );
        }

        canvas.into()
    }
}

fn print_reg(canvas: &mut Canvas, x: usize, y: usize, name: &str, value: u32, prev: u32) {
    let mut x = x;

    if value != prev {
        let (val, pre) = &if name.len() == 2 {
            (format!("{value:04X}"), format!("{prev:04X}"))
        } else {
            (format!("{value:08X}"), format!("{prev:08X}"))
        };

        let i = val
            .chars()
            .zip(pre.chars())
            .enumerate()
            .find_map(|(i, (v, p))| if v != p { Some(i) } else { None });

        canvas.draw_str(x, y, super::ST_CAPTION, name);
        x += name.len() + 1;

        if let Some(i) = i {
            canvas.draw_str(x, y, super::ST_CHANGED, &val[..i]);
            x += i;
            canvas.draw_str(x, y, super::ST_ACTIVE, &val[i..]);
        } else {
            canvas.draw_str(x, y, super::ST_NORMAL, val);
        }
    } else {
        let val = &if name.len() == 2 {
            format!("{value:04X}")
        } else {
            format!("{value:08X}")
        };

        canvas.draw_str(x, y, super::ST_CAPTION, name);
        x += name.len() + 1;
        canvas.draw_str(x, y, super::ST_NORMAL, val);
    }
}
