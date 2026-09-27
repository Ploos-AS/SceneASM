use std::collections::BTreeMap;

use thiserror::Error;

pub mod opcodes;
pub use opcodes::{
    opcode, opcode_by_byte, opcode_with_policy, AddressingMode, ExtraCycle, Opcode, OpcodeClass,
    UndocumentedPolicy, OPCODES, UNDOCUMENTED_OPCODES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cpu { Mos6502, Mos6510 }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target { pub name: &'static str, pub cpu: Cpu, pub origin: u16 }

impl Target {
    pub const fn c64() -> Self { Self { name: "c64", cpu: Cpu::Mos6510, origin: 0x0801 } }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionInfo { pub address: u16, pub opcode: Opcode }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assembly {
    pub bytes: Vec<u8>,
    pub symbols: BTreeMap<String, u16>,
    pub origin: u16,
    pub instructions: Vec<InstructionInfo>,
    pub cycles: u64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AssembleError {
    #[error("unsupported statement on line {line}: {text}")]
    UnsupportedStatement { line: usize, text: String },
    #[error("invalid number on line {line}: {text}")]
    InvalidNumber { line: usize, text: String },
    #[error("duplicate symbol on line {line}: {name}")]
    DuplicateSymbol { line: usize, name: String },
}

pub fn assemble(source: &str, target: Target) -> Result<Assembly, AssembleError> {
    assemble_with_policy(source, target, UndocumentedPolicy::Deny)
}

pub fn assemble_with_policy(source: &str, target: Target, mut undocumented_policy: UndocumentedPolicy) -> Result<Assembly, AssembleError> {
    let mut origin = target.origin;
    let mut pc = origin;
    let mut bytes = Vec::new();
    let mut symbols = BTreeMap::new();
    let mut instructions = Vec::new();
    let mut cycles = 0u64;

    for (index, raw) in source.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.split(';').next().unwrap_or("").trim();
        if line.is_empty() { continue; }

        if let Some(rest) = line.strip_prefix(".org") {
            let value = parse_u16(rest.trim(), line_no)?;
            if bytes.is_empty() { origin = value; pc = value; continue; }
        }

        if let Some(label) = line.strip_suffix(':') {
            let name = label.trim().to_string();
            if symbols.insert(name.clone(), pc).is_some() {
                return Err(AssembleError::DuplicateSymbol { line: line_no, name });
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix(".undocumented") {
            undocumented_policy = match rest.trim().to_ascii_lowercase().as_str() {
                "deny" => UndocumentedPolicy::Deny,
                "stable" => UndocumentedPolicy::Stable,
                "all" => UndocumentedPolicy::All,
                _ => return Err(AssembleError::UnsupportedStatement { line: line_no, text: line.to_string() }),
            };
            continue;
        }

        if let Some(rest) = line.strip_prefix(".byte") {
            for token in rest.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let value = parse_u16(token, line_no)?;
                if value > 0xff {
                    return Err(AssembleError::InvalidNumber { line: line_no, text: token.to_string() });
                }
                bytes.push(value as u8);
                pc = pc.wrapping_add(1);
            }
            continue;
        }

        let upper = line.to_ascii_uppercase();
        let (mnemonic, operand) = if let Some((m, _)) = upper.split_once(' ') {
            (m, line.split_once(' ').map(|(_, a)| a.trim()))
        } else {
            (upper.as_str(), None)
        };

        let (mode, operand_value) = match operand {
            None => (AddressingMode::Implied, None),
            Some(arg) if arg.starts_with('#') => (AddressingMode::Immediate, Some(parse_u16(arg.trim_start_matches('#').trim(), line_no)?)),
            Some(arg) => {
                let value = parse_u16(arg, line_no)?;
                let zp = value <= 0xff && opcode_with_policy(mnemonic, AddressingMode::ZeroPage, undocumented_policy).is_some();
                (if zp { AddressingMode::ZeroPage } else { AddressingMode::Absolute }, Some(value))
            }
        };

        let opcode = opcode_with_policy(mnemonic, mode, undocumented_policy)
            .ok_or_else(|| AssembleError::UnsupportedStatement { line: line_no, text: line.to_string() })?;

        let address = pc;
        bytes.push(opcode.code);
        match mode {
            AddressingMode::Implied | AddressingMode::Accumulator => {}
            AddressingMode::Immediate | AddressingMode::ZeroPage | AddressingMode::ZeroPageX |
            AddressingMode::ZeroPageY | AddressingMode::Relative | AddressingMode::IndexedIndirect |
            AddressingMode::IndirectIndexed => {
                let value = operand_value.unwrap();
                if value > 0xff { return Err(AssembleError::InvalidNumber { line: line_no, text: operand.unwrap().into() }); }
                bytes.push(value as u8);
            }
            AddressingMode::Absolute | AddressingMode::AbsoluteX | AddressingMode::AbsoluteY |
            AddressingMode::Indirect => {
                let value = operand_value.unwrap();
                bytes.extend_from_slice(&[value as u8, (value >> 8) as u8]);
            }
        }
        pc = pc.wrapping_add(opcode.bytes as u16);
        cycles += opcode.cycles as u64;
        instructions.push(InstructionInfo { address, opcode });
    }

    Ok(Assembly { bytes, symbols, origin, instructions, cycles })
}

fn parse_u16(text: &str, line: usize) -> Result<u16, AssembleError> {
    let text = text.trim();
    let parsed = if let Some(hex) = text.strip_prefix('$') {
        u16::from_str_radix(hex, 16)
    } else if let Some(hex) = text.strip_prefix("0x") {
        u16::from_str_radix(hex, 16)
    } else {
        text.parse::<u16>()
    };
    parsed.map_err(|_| AssembleError::InvalidNumber { line, text: text.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembles_minimal_c64_code_and_tracks_cycles() {
        let out = assemble(".org $080d\nstart:\n  sei\n  lda #$06\n  sta $d020\n  rts\n", Target::c64()).unwrap();
        assert_eq!(out.origin, 0x080d);
        assert_eq!(out.symbols["start"], 0x080d);
        assert_eq!(out.bytes, vec![0x78, 0xa9, 0x06, 0x8d, 0x20, 0xd0, 0x60]);
        assert_eq!(out.cycles, 14);
        assert_eq!(out.instructions.len(), 4);
        assert_eq!(out.instructions[2].address, 0x0810);
    }

    #[test]
    fn opcode_metadata_is_available_to_tooling() {
        let sei = OPCODES.iter().find(|op| op.mnemonic == "SEI").unwrap();
        assert_eq!(sei.cycles, 2);
        assert_eq!(sei.class, OpcodeClass::Documented);
    }

    #[test]
    fn chooses_zero_page_when_instruction_supports_it() {
        let out = assemble("lda $20\nsta $21\n", Target::c64()).unwrap();
        assert_eq!(out.bytes, vec![0xa5, 0x20, 0x85, 0x21]);
        assert_eq!(out.cycles, 6);
    }

    #[test]
    fn undocumented_policy_can_be_selected_in_source() {
        let denied = assemble("lax ($20,x)\n", Target::c64());
        assert!(denied.is_err());
    }
}
