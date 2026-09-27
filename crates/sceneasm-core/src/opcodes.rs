#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressingMode {
    Implied,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Relative,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndexedIndirect,
    IndirectIndexed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtraCycle {
    None,
    PageCross,
    BranchTaken,
    BranchTakenAndPageCross,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opcode {
    pub mnemonic: &'static str,
    pub mode: AddressingMode,
    pub code: u8,
    pub bytes: u8,
    pub cycles: u8,
    pub extra_cycle: ExtraCycle,
    pub undocumented: bool,
}

macro_rules! op {
    ($m:literal,$mode:ident,$code:expr,$bytes:expr,$cycles:expr) => {
        Opcode { mnemonic:$m, mode:AddressingMode::$mode, code:$code, bytes:$bytes, cycles:$cycles, extra_cycle:ExtraCycle::None, undocumented:false }
    };
    ($m:literal,$mode:ident,$code:expr,$bytes:expr,$cycles:expr,$extra:ident) => {
        Opcode { mnemonic:$m, mode:AddressingMode::$mode, code:$code, bytes:$bytes, cycles:$cycles, extra_cycle:ExtraCycle::$extra, undocumented:false }
    };
}

/// Initial documented NMOS 6502/6510 table. This is deliberately data-driven:
/// assembler, disassembler, LSP and timing analysis must consume the same model.
pub const OPCODES: &[Opcode] = &[
    op!("BRK",Implied,0x00,1,7), op!("ORA",IndexedIndirect,0x01,2,6), op!("ORA",ZeroPage,0x05,2,3),
    op!("ASL",ZeroPage,0x06,2,5), op!("PHP",Implied,0x08,1,3), op!("ORA",Immediate,0x09,2,2),
    op!("ASL",Accumulator,0x0a,1,2), op!("ORA",Absolute,0x0d,3,4), op!("ASL",Absolute,0x0e,3,6),
    op!("BPL",Relative,0x10,2,2,BranchTakenAndPageCross), op!("ORA",IndirectIndexed,0x11,2,5,PageCross),
    op!("ORA",ZeroPageX,0x15,2,4), op!("ASL",ZeroPageX,0x16,2,6), op!("CLC",Implied,0x18,1,2),
    op!("ORA",AbsoluteY,0x19,3,4,PageCross), op!("ORA",AbsoluteX,0x1d,3,4,PageCross), op!("ASL",AbsoluteX,0x1e,3,7),
    op!("JSR",Absolute,0x20,3,6), op!("AND",IndexedIndirect,0x21,2,6), op!("BIT",ZeroPage,0x24,2,3),
    op!("AND",ZeroPage,0x25,2,3), op!("ROL",ZeroPage,0x26,2,5), op!("PLP",Implied,0x28,1,4),
    op!("AND",Immediate,0x29,2,2), op!("ROL",Accumulator,0x2a,1,2), op!("BIT",Absolute,0x2c,3,4),
    op!("AND",Absolute,0x2d,3,4), op!("ROL",Absolute,0x2e,3,6), op!("BMI",Relative,0x30,2,2,BranchTakenAndPageCross),
    op!("SEC",Implied,0x38,1,2), op!("RTI",Implied,0x40,1,6), op!("EOR",Immediate,0x49,2,2),
    op!("LSR",Accumulator,0x4a,1,2), op!("JMP",Absolute,0x4c,3,3), op!("JMP",Indirect,0x6c,3,5),
    op!("RTS",Implied,0x60,1,6), op!("ADC",Immediate,0x69,2,2), op!("ROR",Accumulator,0x6a,1,2),
    op!("SEI",Implied,0x78,1,2), op!("STA",IndexedIndirect,0x81,2,6), op!("STY",ZeroPage,0x84,2,3),
    op!("STA",ZeroPage,0x85,2,3), op!("STX",ZeroPage,0x86,2,3), op!("DEY",Implied,0x88,1,2),
    op!("TXA",Implied,0x8a,1,2), op!("STY",Absolute,0x8c,3,4), op!("STA",Absolute,0x8d,3,4),
    op!("STX",Absolute,0x8e,3,4), op!("BCC",Relative,0x90,2,2,BranchTakenAndPageCross),
    op!("STA",IndirectIndexed,0x91,2,6), op!("STY",ZeroPageX,0x94,2,4), op!("STA",ZeroPageX,0x95,2,4),
    op!("STX",ZeroPageY,0x96,2,4), op!("TYA",Implied,0x98,1,2), op!("STA",AbsoluteY,0x99,3,5),
    op!("TXS",Implied,0x9a,1,2), op!("STA",AbsoluteX,0x9d,3,5), op!("LDY",Immediate,0xa0,2,2),
    op!("LDA",IndexedIndirect,0xa1,2,6), op!("LDX",Immediate,0xa2,2,2), op!("LDY",ZeroPage,0xa4,2,3),
    op!("LDA",ZeroPage,0xa5,2,3), op!("LDX",ZeroPage,0xa6,2,3), op!("TAY",Implied,0xa8,1,2),
    op!("LDA",Immediate,0xa9,2,2), op!("TAX",Implied,0xaa,1,2), op!("LDY",Absolute,0xac,3,4),
    op!("LDA",Absolute,0xad,3,4), op!("LDX",Absolute,0xae,3,4), op!("BCS",Relative,0xb0,2,2,BranchTakenAndPageCross),
    op!("LDA",IndirectIndexed,0xb1,2,5,PageCross), op!("LDY",ZeroPageX,0xb4,2,4), op!("LDA",ZeroPageX,0xb5,2,4),
    op!("LDX",ZeroPageY,0xb6,2,4), op!("CLV",Implied,0xb8,1,2), op!("LDA",AbsoluteY,0xb9,3,4,PageCross),
    op!("TSX",Implied,0xba,1,2), op!("LDY",AbsoluteX,0xbc,3,4,PageCross), op!("LDA",AbsoluteX,0xbd,3,4,PageCross),
    op!("LDX",AbsoluteY,0xbe,3,4,PageCross), op!("CPY",Immediate,0xc0,2,2), op!("CMP",IndexedIndirect,0xc1,2,6),
    op!("CPY",ZeroPage,0xc4,2,3), op!("CMP",ZeroPage,0xc5,2,3), op!("DEC",ZeroPage,0xc6,2,5),
    op!("INY",Implied,0xc8,1,2), op!("CMP",Immediate,0xc9,2,2), op!("DEX",Implied,0xca,1,2),
    op!("CPY",Absolute,0xcc,3,4), op!("CMP",Absolute,0xcd,3,4), op!("DEC",Absolute,0xce,3,6),
    op!("BNE",Relative,0xd0,2,2,BranchTakenAndPageCross), op!("CMP",IndirectIndexed,0xd1,2,5,PageCross),
    op!("CMP",ZeroPageX,0xd5,2,4), op!("DEC",ZeroPageX,0xd6,2,6), op!("CLD",Implied,0xd8,1,2),
    op!("CMP",AbsoluteY,0xd9,3,4,PageCross), op!("CMP",AbsoluteX,0xdd,3,4,PageCross), op!("DEC",AbsoluteX,0xde,3,7),
    op!("CPX",Immediate,0xe0,2,2), op!("SBC",IndexedIndirect,0xe1,2,6), op!("CPX",ZeroPage,0xe4,2,3),
    op!("SBC",ZeroPage,0xe5,2,3), op!("INC",ZeroPage,0xe6,2,5), op!("INX",Implied,0xe8,1,2),
    op!("SBC",Immediate,0xe9,2,2), op!("NOP",Implied,0xea,1,2), op!("CPX",Absolute,0xec,3,4),
    op!("SBC",Absolute,0xed,3,4), op!("INC",Absolute,0xee,3,6), op!("BEQ",Relative,0xf0,2,2,BranchTakenAndPageCross),
    op!("SBC",IndirectIndexed,0xf1,2,5,PageCross), op!("SBC",ZeroPageX,0xf5,2,4), op!("INC",ZeroPageX,0xf6,2,6),
    op!("SED",Implied,0xf8,1,2), op!("SBC",AbsoluteY,0xf9,3,4,PageCross), op!("SBC",AbsoluteX,0xfd,3,4,PageCross),
    op!("INC",AbsoluteX,0xfe,3,7),
];

pub fn opcode(mnemonic: &str, mode: AddressingMode) -> Option<Opcode> {
    OPCODES.iter().copied().find(|op| op.mnemonic.eq_ignore_ascii_case(mnemonic) && op.mode == mode)
}

pub fn opcode_by_byte(code: u8) -> Option<Opcode> {
    OPCODES.iter().copied().find(|op| op.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_metadata_models_page_crossing() {
        let lda = opcode("LDA", AddressingMode::AbsoluteX).unwrap();
        assert_eq!(lda.cycles, 4);
        assert_eq!(lda.extra_cycle, ExtraCycle::PageCross);
    }

    #[test]
    fn branches_expose_variable_timing() {
        let bne = opcode("BNE", AddressingMode::Relative).unwrap();
        assert_eq!(bne.cycles, 2);
        assert_eq!(bne.extra_cycle, ExtraCycle::BranchTakenAndPageCross);
    }

    #[test]
    fn documented_table_has_no_duplicate_bytes() {
        let mut seen = [false; 256];
        for op in OPCODES {
            assert!(!seen[op.code as usize], "duplicate opcode byte {:02x}", op.code);
            seen[op.code as usize] = true;
        }
    }

    #[test]
    fn opcode_lengths_match_addressing_modes() {
        for op in OPCODES {
            let expected = match op.mode {
                AddressingMode::Implied | AddressingMode::Accumulator => 1,
                AddressingMode::Immediate | AddressingMode::ZeroPage | AddressingMode::ZeroPageX |
                AddressingMode::ZeroPageY | AddressingMode::Relative | AddressingMode::IndexedIndirect |
                AddressingMode::IndirectIndexed => 2,
                AddressingMode::Absolute | AddressingMode::AbsoluteX | AddressingMode::AbsoluteY |
                AddressingMode::Indirect => 3,
            };
            assert_eq!(op.bytes, expected, "bad length for {} {:?}", op.mnemonic, op.mode);
        }
    }

    #[test]
    fn reverse_lookup_supports_disassembler_and_debugger() {
        let sei = opcode_by_byte(0x78).unwrap();
        assert_eq!(sei.mnemonic, "SEI");
        assert_eq!(sei.mode, AddressingMode::Implied);
    }
}
