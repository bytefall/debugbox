use anyhow::Result;
use iced_x86::{Code, Decoder, DecoderOptions, Instruction};

use super::Address;
use crate::bus::Proxy;

const DECODER_OPTIONS: u32 = DecoderOptions::NONE;
const MAX_INSTR_LEN: usize = 15; // maximal instruction length (don't load less than that)
const SKIP_INSTR_LEN: usize = MAX_INSTR_LEN * 3; // number of bytes to skip as a few instructions might be corrupted
const FETCH_ATTEMPT_NUM: usize = 5; // number of attempts to decode instructions
const BITNESS: u32 = 16;

pub fn fetch_before(
    proxy: &Proxy,
    addr: Address,
    limit: usize,
) -> Result<Vec<(Instruction, Vec<u8>)>> {
    let mut code: Vec<(Instruction, Vec<u8>)> = Vec::new();

    for attempt in 1..=FETCH_ATTEMPT_NUM {
        let first = code.first().map(|(i, _)| i.ip32()).unwrap_or(addr.offset);
        let start = first.saturating_sub(
            (attempt * SKIP_INSTR_LEN + limit.saturating_sub(code.len()) * MAX_INSTR_LEN) as u32,
        );

        let data = proxy
            .mem
            .get(addr.segment, start, first.saturating_sub(start))?;
        let mut dec = Decoder::with_ip(BITNESS, &data, start.into(), DECODER_OPTIONS);

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

pub fn fetch_after(
    proxy: &Proxy,
    addr: Address,
    limit: usize,
) -> Result<Vec<(Instruction, Vec<u8>)>> {
    let max = if BITNESS == 16 {
        u16::MAX as u32
    } else {
        u32::MAX
    };
    let mut code: Vec<(Instruction, Vec<u8>)> = Vec::new();
    let mut ins = Instruction::default();

    for attempt in 1..=FETCH_ATTEMPT_NUM {
        let start = code
            .last()
            .map(|(i, _)| i.next_ip32())
            .unwrap_or(addr.offset);
        let end = max.min(start.saturating_add(
            (attempt * SKIP_INSTR_LEN + limit.saturating_sub(code.len()) * MAX_INSTR_LEN) as u32,
        ));

        let data = proxy
            .mem
            .get(addr.segment, start, end.saturating_sub(start))?;
        let mut dec = Decoder::with_ip(BITNESS, &data, start.into(), DECODER_OPTIONS);

        while dec.can_decode() {
            let pos = dec.position();
            dec.decode_out(&mut ins);

            code.push((
                ins,
                data.iter().skip(pos).take(ins.len()).copied().collect(),
            ));
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

pub fn step_over(proxy: &Proxy, addr: Address) -> Result<()> {
    let data = proxy
        .mem
        .get(addr.segment, addr.offset, (MAX_INSTR_LEN * 2) as u32)?;
    let mut dec = Decoder::with_ip(BITNESS, &data, addr.offset as u64, DECODER_OPTIONS);

    let mut ins = Instruction::default();
    dec.decode_out(&mut ins);

    if !(ins.is_call_near()
        || ins.is_call_far()
        || ins.is_call_near_indirect()
        || ins.is_call_far_indirect()
        || ins.is_loop()
        || ins.has_rep_prefix()
        || ins.has_repne_prefix()
        || ins.code() == Code::Int_imm8)
    {
        proxy.cpu.step_in()?;

        return Ok(());
    }

    dec.decode_out(&mut ins);

    if ins.is_invalid() {
        return Ok(());
    }

    let mut data = [0u8; MAX_INSTR_LEN];

    for (i, d) in data.iter_mut().enumerate().take(ins.len()) {
        *d = proxy.mem.set(addr.segment, ins.ip32() + i as u32, 0xCC)?;
    }

    proxy.cpu.run()?;

    for (i, d) in data.iter().enumerate().take(ins.len()) {
        proxy.mem.set(addr.segment, ins.ip32() + i as u32, *d)?;
    }

    Ok(())
}
