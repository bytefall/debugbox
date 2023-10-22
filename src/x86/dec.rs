use anyhow::Result;
use iced_x86::{Decoder, DecoderOptions, Instruction};
use std::rc::Rc;
use zbus::blocking::Connection;

use super::Address;

const DECODER_OPTIONS: u32 = DecoderOptions::NONE;
const AVG_INSTR_LEN: usize = 3; // average instruction length
const MAX_INSTR_LEN: usize = 15; // maximal instruction length (don't load less than that)
const SKIP_INSTR_LEN: usize = MAX_INSTR_LEN * 3; // number of bytes to skip as a few instructions might be corrupted

pub struct Fetcher {
	conn: Rc<Connection>,
	bitness: u32,
}

impl Fetcher {
	pub fn new(conn: Rc<Connection>) -> Self {
		Self { conn, bitness: 16 }
	}

	pub fn before(&self, addr: Address, limit: usize) -> Result<Vec<(Instruction, Vec<u8>)>> {
		let mut code: Vec<(Instruction, Vec<u8>)> = Vec::new();

		for attempt in 1..=5 {
			let first = code.first().map(|(i, _)| i.ip32()).unwrap_or(addr.offset);

			let start = first
				.saturating_sub((attempt * SKIP_INSTR_LEN + limit.saturating_sub(code.len()) * AVG_INSTR_LEN) as u32);

			let data: Vec<u8> = self
				.conn
				.call_method(
					Some("com.dosbox"),
					"/mem",
					Some("com.dosbox"),
					"get",
					&(addr.segment, start, first.saturating_sub(start)),
				)?
				.body_unchecked()?;

			let mut dec = Decoder::with_ip(self.bitness, &data, start.into(), DECODER_OPTIONS);

			let Some(mut ins) = (0..data.len()).find_map(|skip| {
				let tmp: Vec<_> = dec
					.iter()
					.skip(skip)
					.skip_while(|i| i.is_invalid())
					.take_while(|i| !i.is_invalid())
					.map(|i| {
						(
							i,
							data.iter()
								.skip(i.ip32().saturating_sub(start) as usize)
								.take(i.len())
								.copied()
								.collect::<Vec<_>>(),
						)
					})
					.collect();

				if tmp.last().map(|(x, _)| x.next_ip32()) == Some(first) {
					Some(tmp)
				} else {
					None
				}
			}) else {
				continue;
			};

			if start > 0 {
				let mut i = 0;

				while i < SKIP_INSTR_LEN && !ins.is_empty() {
					i += ins.remove(0).1.len();
				}
			}

			if !ins.is_empty() {
				ins.append(&mut code);
				code = ins;
			}

			if start == 0 || code.len() >= limit {
				break;
			}
		}

		Ok(code)
	}

	pub fn after(&self, addr: Address, limit: usize) -> Result<Vec<(Instruction, Vec<u8>)>> {
		let max = if self.bitness == 16 { u16::MAX as u32 } else { u32::MAX };
		let mut code: Vec<(Instruction, Vec<u8>)> = Vec::new();
		let mut ins = Instruction::default();

		for attempt in 1..=5 {
			let start = code.last().map(|(i, _)| i.next_ip32()).unwrap_or(addr.offset);

			let end =
				max.min(start.saturating_add(
					(attempt * SKIP_INSTR_LEN + limit.saturating_sub(code.len()) * AVG_INSTR_LEN) as u32,
				));

			let data: Vec<u8> = self
				.conn
				.call_method(
					Some("com.dosbox"),
					"/mem",
					Some("com.dosbox"),
					"get",
					&(addr.segment, start, end.saturating_sub(start)),
				)?
				.body_unchecked()?;

			let mut dec = Decoder::with_ip(self.bitness, &data, start.into(), DECODER_OPTIONS);

			while dec.can_decode() {
				let pos = dec.position();
				dec.decode_out(&mut ins);

				code.push((ins, data.iter().skip(pos).take(ins.len()).copied().collect()));
			}

			if end < max {
				let mut i = 0;

				while let Some((ins, _)) = code.pop() {
					i += ins.len();

					if i > SKIP_INSTR_LEN {
						break;
					}
				}
			}

			if end >= max || code.len() >= limit {
				break;
			}
		}

		Ok(code)
	}
}
