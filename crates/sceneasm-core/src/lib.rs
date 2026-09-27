use std::collections::BTreeMap;

use thiserror::Error;

pub mod opcodes;
pub mod layout;
pub mod expr;
pub mod timing;
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
    pub cycle_range: timing::CycleRange,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AssembleError {
    #[error("unsupported statement on line {line}: {text}")]
    UnsupportedStatement { line: usize, text: String },
    #[error("invalid number on line {line}: {text}")]
    InvalidNumber { line: usize, text: String },
    #[error("duplicate symbol on line {line}: {name}")]
    DuplicateSymbol { line: usize, name: String },
    #[error("unresolved symbol on line {line}: {name}")]
    UnresolvedSymbol { line: usize, name: String },
    #[error("layout did not converge")]
    LayoutDidNotConverge,
    #[error("invalid expression on line {line}: {text}")]
    InvalidExpression { line: usize, text: String },
    #[error("assertion failed on line {line}: {expression}")]
    AssertionFailed { line: usize, expression: String },
    #[error("cycle budget exceeded on line {line}: worst case {actual} > budget {budget}")]
    CycleBudgetExceeded { line: usize, actual: u64, budget: u64 },
}

pub fn assemble(source: &str, target: Target) -> Result<Assembly, AssembleError> {
    assemble_with_policy(source, target, UndocumentedPolicy::Deny)
}

pub fn assemble_with_policy(source: &str, target: Target, mut undocumented_policy: UndocumentedPolicy) -> Result<Assembly, AssembleError> {
    let resolved = layout::layout(source, target.origin, undocumented_policy)?;
    let mut origin = resolved.origin;
    let mut pc = origin;
    let mut bytes = Vec::new();
    let symbols = resolved.symbols;
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

        if line.ends_with(':') || (!line.starts_with('.') && line.contains('=')) {
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

        if line.starts_with(".assert_cycles") {
            continue;
        }

        if let Some(rest) = line.strip_prefix(".assert") {
            let expression = rest.trim();
            let value = expr::eval(expression, &symbols, line_no)?
                .ok_or_else(|| AssembleError::UnresolvedSymbol { line: line_no, name: expression.to_string() })?;
            if value == 0 {
                return Err(AssembleError::AssertionFailed { line: line_no, expression: expression.to_string() });
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix(".byte") {
            for token in rest.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let value = expr::eval(token, &symbols, line_no)?
                    .ok_or_else(|| AssembleError::UnresolvedSymbol { line: line_no, name: token.to_string() })?;
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

        let (mode, operand_value) = layout::choose_mode(mnemonic, operand, &symbols, undocumented_policy, line_no)?;
        if operand.is_some() && operand_value.is_none() {
            return Err(AssembleError::UnresolvedSymbol { line: line_no, name: operand.unwrap().trim_start_matches('#').trim().to_string() });
        }

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

    let cycle_range = timing::analyze(&instructions);
    for (index, raw) in source.lines().enumerate() {
        let line = raw.split(';').next().unwrap_or("").trim();
        if let Some(rest) = line.strip_prefix(".assert_cycles") {
            let rest = rest.trim();
            let budget_text = rest.strip_prefix("<=").map(str::trim)
                .ok_or_else(|| AssembleError::InvalidExpression { line: index + 1, text: rest.to_string() })?;
            let budget = expr::eval(budget_text, &symbols, index + 1)?
                .ok_or_else(|| AssembleError::UnresolvedSymbol { line: index + 1, name: budget_text.to_string() })? as u64;
            if !cycle_range.fits(budget) {
                return Err(AssembleError::CycleBudgetExceeded { line: index + 1, actual: cycle_range.max, budget });
            }
        }
    }

    Ok(Assembly { bytes, symbols, origin, instructions, cycles, cycle_range })
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
        assert_eq!(out.cycle_range, timing::CycleRange { min: 14, max: 14 });
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

    #[test]
    fn cycle_contract_uses_worst_case_timing() {
        let ok = assemble("lda $1234,x\nbne $10\n.assert_cycles <= 9\n", Target::c64());
        assert!(ok.is_ok());
        let failed = assemble("lda $1234,x\nbne $10\n.assert_cycles <= 8\n", Target::c64());
        assert!(matches!(failed, Err(AssembleError::CycleBudgetExceeded { actual: 9, budget: 8, .. })));
    }

    #[test]
    fn constants_and_assertions_share_expression_semantics() {
        let out = assemble(".org $2000\ntable:\n.byte 1,2,3\ntable_end:\nSIZE = table_end - table\n.assert SIZE <= 3\n.byte SIZE\n", Target::c64()).unwrap();
        assert_eq!(out.symbols["SIZE"], 3);
        assert_eq!(out.bytes, vec![1, 2, 3, 3]);

        let failed = assemble("VALUE = 4\n.assert VALUE <= 3\n", Target::c64());
        assert!(matches!(failed, Err(AssembleError::AssertionFailed { .. })));
    }

    #[test]
    fn low_high_byte_expressions_work_in_immediates() {
        let out = assemble(".org $c000\nirq:\nlda #<irq\nldx #>irq\n", Target::c64()).unwrap();
        assert_eq!(out.bytes, vec![0xa9, 0x00, 0xa2, 0xc0]);
    }

    #[test]
    fn forward_reference_can_shrink_to_zero_page() {
        let out = assemble(".org $0020\nlda table\nnop\ntable:\n.byte 1\n", Target::c64()).unwrap();
        assert_eq!(out.symbols["table"], 0x0023);
        assert_eq!(out.bytes, vec![0xa5, 0x23, 0xea, 0x01]);
    }
}
