use std::collections::BTreeMap;

use thiserror::Error;

pub mod c64;
pub mod c64_registers;
mod diagnostic;
pub mod expr;
pub mod layout;
pub mod opcodes;
mod source;
pub mod timing;
pub use opcodes::{
    opcode, opcode_by_byte, opcode_with_policy, AddressingMode, ExtraCycle, Opcode, OpcodeClass,
    UndocumentedPolicy, OPCODES, UNDOCUMENTED_OPCODES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cpu {
    Mos6502,
    Mos6510,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub name: &'static str,
    pub cpu: Cpu,
    pub origin: u16,
}

impl Target {
    pub const fn c64() -> Self {
        Self {
            name: "c64",
            cpu: Cpu::Mos6510,
            origin: 0x0801,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub file_id: u32,
    pub line: usize,
    pub column_start: usize,
    pub column_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionInfo {
    pub address: u16,
    pub source: SourceSpan,
    pub expansion: source::ExpansionTrace,
    pub opcode: Opcode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareWrite {
    pub source: SourceSpan,
    pub register: c64_registers::HardwareRegister,
    pub value: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assembly {
    pub bytes: Vec<u8>,
    pub symbols: BTreeMap<String, u16>,
    pub symbol_definitions: BTreeMap<String, SourceSpan>,
    pub origin: u16,
    pub instructions: Vec<InstructionInfo>,
    pub cycles: u64,
    pub cycle_range: timing::CycleRange,
    pub raster_contracts: Vec<c64::RasterContract>,
    pub hardware_writes: Vec<HardwareWrite>,
    pub source_map: source::SourceMap,
    pub diagnostics: Vec<diagnostic::Diagnostic>,
}

impl Assembly {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == diagnostic::Severity::Error)
    }

    pub fn errors(&self) -> impl Iterator<Item = &diagnostic::Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == diagnostic::Severity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &diagnostic::Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == diagnostic::Severity::Warning)
    }
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
    CycleBudgetExceeded {
        line: usize,
        actual: u64,
        budget: u64,
    },
    #[error("invalid raster line on source line {source_line}: {raster_line}")]
    InvalidRasterLine {
        source_line: usize,
        raster_line: u16,
    },
    #[error("raster budget exceeded on source line {source_line}: raster {raster_line}, worst case {actual} > {budget}")]
    RasterBudgetExceeded {
        source_line: usize,
        raster_line: u16,
        actual: u64,
        budget: u16,
    },
    #[error("invalid VIC-II state on line {line}: {text}")]
    InvalidVicState { line: usize, text: String },
}

pub fn assemble(source: &str, target: Target) -> Result<Assembly, AssembleError> {
    assemble_with_policy(source, target, UndocumentedPolicy::Deny)
}

pub fn assemble_with_policy(
    source: &str,
    target: Target,
    mut undocumented_policy: UndocumentedPolicy,
) -> Result<Assembly, AssembleError> {
    let resolved = layout::layout(source, target.origin, undocumented_policy)?;
    let mut origin = resolved.origin;
    let mut pc = origin;
    let mut bytes = Vec::new();
    let symbols = resolved.symbols;
    let mut symbol_definitions = BTreeMap::new();
    for (index, raw) in source.lines().enumerate() {
        let code = raw.split(';').next().unwrap_or("");
        let trimmed = code.trim();
        let name = if let Some(label) = trimmed.strip_suffix(':') {
            Some(label.trim())
        } else if !trimmed.starts_with('.') {
            trimmed.split_once('=').map(|(name, _)| name.trim())
        } else {
            None
        };
        if let Some(name) = name.filter(|name| symbols.contains_key(*name)) {
            let column_start = raw.find(name).unwrap_or(0) + 1;
            symbol_definitions.insert(
                name.to_string(),
                SourceSpan {
                    file_id: 0,
                    line: index + 1,
                    column_start,
                    column_end: column_start + name.len(),
                },
            );
        }
    }
    let mut instructions = Vec::new();
    let mut cycles = 0u64;
    let source_map = source::SourceMap::default();
    let mut diagnostics = Vec::new();
    let mut raster_blocks: Vec<(usize, u16, usize, usize, c64::VicState)> = Vec::new();
    let mut open_raster: Option<(usize, u16, usize, c64::VicState)> = None;
    let mut vic_state = c64::VicState::default();

    for (index, raw) in source.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.split(';').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line
            .strip_prefix("* =")
            .or_else(|| line.strip_prefix(".org"))
        {
            let value = parse_u16(rest.trim(), line_no)?;
            if bytes.is_empty() {
                origin = value;
                pc = value;
                continue;
            }
            if value < pc {
                return Err(AssembleError::InvalidNumber {
                    line: line_no,
                    text: line.to_string(),
                });
            }
            bytes.resize(bytes.len() + (value - pc) as usize, 0);
            pc = value;
            continue;
        }

        if line == "}" {
            if let Some((source_line, raster_line, start, state)) = open_raster.take() {
                raster_blocks.push((source_line, raster_line, start, instructions.len(), state));
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix(".raster") {
            if line.ends_with('{') {
                let expression = rest.trim_end_matches('{').trim();
                let raster_line = expr::eval(expression, &symbols, line_no)?.ok_or_else(|| {
                    AssembleError::UnresolvedSymbol {
                        line: line_no,
                        name: expression.to_string(),
                    }
                })?;
                open_raster = Some((line_no, raster_line, instructions.len(), vic_state));
                continue;
            }
        }

        if let Some(rest) = line.strip_prefix(".vic_display") {
            vic_state.display_enabled = match rest.trim().to_ascii_lowercase().as_str() {
                "on" => true,
                "off" => false,
                _ => {
                    return Err(AssembleError::InvalidVicState {
                        line: line_no,
                        text: line.to_string(),
                    })
                }
            };
            continue;
        }

        if let Some(rest) = line.strip_prefix(".vic_yscroll") {
            let value = expr::eval(rest.trim(), &symbols, line_no)?.ok_or_else(|| {
                AssembleError::UnresolvedSymbol {
                    line: line_no,
                    name: rest.trim().to_string(),
                }
            })?;
            if value > 7 {
                return Err(AssembleError::InvalidVicState {
                    line: line_no,
                    text: line.to_string(),
                });
            }
            vic_state.y_scroll = value as u8;
            continue;
        }

        if let Some(rest) = line.strip_prefix(".vic_sprites") {
            let value = expr::eval(rest.trim(), &symbols, line_no)?.ok_or_else(|| {
                AssembleError::UnresolvedSymbol {
                    line: line_no,
                    name: rest.trim().to_string(),
                }
            })?;
            if value > 0xff {
                return Err(AssembleError::InvalidVicState {
                    line: line_no,
                    text: line.to_string(),
                });
            }
            vic_state.sprite_enable_mask = value as u8;
            continue;
        }

        if let Some(rest) = line.strip_prefix(".vic_sprite_y") {
            let mut parts = rest.split_whitespace();
            let sprite_text = parts.next().ok_or_else(|| AssembleError::InvalidVicState {
                line: line_no,
                text: line.to_string(),
            })?;
            let y_text = parts.next().ok_or_else(|| AssembleError::InvalidVicState {
                line: line_no,
                text: line.to_string(),
            })?;
            if parts.next().is_some() {
                return Err(AssembleError::InvalidVicState {
                    line: line_no,
                    text: line.to_string(),
                });
            }
            let sprite = expr::eval(sprite_text, &symbols, line_no)?.ok_or_else(|| {
                AssembleError::UnresolvedSymbol {
                    line: line_no,
                    name: sprite_text.to_string(),
                }
            })?;
            let y = expr::eval(y_text, &symbols, line_no)?.ok_or_else(|| {
                AssembleError::UnresolvedSymbol {
                    line: line_no,
                    name: y_text.to_string(),
                }
            })?;
            if sprite > 7 || y > 0xff {
                return Err(AssembleError::InvalidVicState {
                    line: line_no,
                    text: line.to_string(),
                });
            }
            vic_state.sprite_y[sprite as usize] = y as u8;
            continue;
        }

        if line.ends_with(':') || (!line.starts_with('.') && line.contains('=')) {
            continue;
        }

        if let Some(rest) = line.strip_prefix(".undocumented") {
            undocumented_policy = match rest.trim().to_ascii_lowercase().as_str() {
                "deny" => UndocumentedPolicy::Deny,
                "stable" => UndocumentedPolicy::Stable,
                "all" => UndocumentedPolicy::All,
                _ => {
                    return Err(AssembleError::UnsupportedStatement {
                        line: line_no,
                        text: line.to_string(),
                    })
                }
            };
            continue;
        }

        if line.starts_with(".assert_cycles") || line.starts_with(".raster") {
            continue;
        }

        if let Some(rest) = line.strip_prefix(".assert") {
            let expression = rest.trim();
            let value = expr::eval(expression, &symbols, line_no)?.ok_or_else(|| {
                AssembleError::UnresolvedSymbol {
                    line: line_no,
                    name: expression.to_string(),
                }
            })?;
            if value == 0 {
                return Err(AssembleError::AssertionFailed {
                    line: line_no,
                    expression: expression.to_string(),
                });
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix(".text") {
            let text = rest.trim();
            if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
                let payload = &text.as_bytes()[1..text.len() - 1];
                bytes.extend_from_slice(payload);
                pc = pc.wrapping_add(payload.len() as u16);
                continue;
            }
            return Err(AssembleError::UnsupportedStatement {
                line: line_no,
                text: line.to_string(),
            });
        }

        if let Some(rest) = line.strip_prefix(".byte") {
            for token in rest.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let value = expr::eval(token, &symbols, line_no)?.ok_or_else(|| {
                    AssembleError::UnresolvedSymbol {
                        line: line_no,
                        name: token.to_string(),
                    }
                })?;
                if value > 0xff {
                    return Err(AssembleError::InvalidNumber {
                        line: line_no,
                        text: token.to_string(),
                    });
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

        let (mode, operand_value) = layout::choose_mode(
            mnemonic,
            operand,
            &symbols,
            undocumented_policy,
            line_no,
            pc,
        )?;
        if let (Some(operand), None) = (operand, operand_value) {
            return Err(AssembleError::UnresolvedSymbol {
                line: line_no,
                name: operand.trim_start_matches('#').trim().to_string(),
            });
        }

        let opcode = opcode_with_policy(mnemonic, mode, undocumented_policy).ok_or_else(|| {
            AssembleError::UnsupportedStatement {
                line: line_no,
                text: line.to_string(),
            }
        })?;

        let address = pc;
        bytes.push(opcode.code);
        match mode {
            AddressingMode::Implied | AddressingMode::Accumulator => {}
            AddressingMode::Immediate
            | AddressingMode::ZeroPage
            | AddressingMode::ZeroPageX
            | AddressingMode::ZeroPageY
            | AddressingMode::IndexedIndirect
            | AddressingMode::IndirectIndexed => {
                let value = operand_value.unwrap();
                if value > 0xff {
                    return Err(AssembleError::InvalidNumber {
                        line: line_no,
                        text: operand.unwrap().into(),
                    });
                }
                bytes.push(value as u8);
            }
            AddressingMode::Relative => {
                let target = operand_value.unwrap();
                let next = pc.wrapping_add(2);
                let delta = target as i32 - next as i32;
                if !(-128..=127).contains(&delta) {
                    return Err(AssembleError::InvalidNumber {
                        line: line_no,
                        text: operand.unwrap().into(),
                    });
                }
                bytes.push((delta as i8) as u8);
            }
            AddressingMode::Absolute
            | AddressingMode::AbsoluteX
            | AddressingMode::AbsoluteY
            | AddressingMode::Indirect => {
                let value = operand_value.unwrap();
                bytes.extend_from_slice(&[value as u8, (value >> 8) as u8]);
            }
        }
        pc = pc.wrapping_add(opcode.bytes as u16);
        cycles += opcode.cycles as u64;
        let column_start = raw.len().saturating_sub(raw.trim_start().len()) + 1;
        let column_end = raw.len() + 1;
        instructions.push(InstructionInfo {
            address,
            source: SourceSpan {
                file_id: 0,
                line: line_no,
                column_start,
                column_end,
            },
            expansion: source::ExpansionTrace::default(),
            opcode,
        });
    }

    if let Some((source_line, raster_line, start, state)) = open_raster.take() {
        raster_blocks.push((source_line, raster_line, start, instructions.len(), state));
    }
    let cycle_range = timing::analyze(&instructions);
    let mut raster_contracts = Vec::new();
    for (source_line, raster_line, start, end, state) in raster_blocks {
        let range = timing::analyze(&instructions[start..end]);
        let profile = c64::C64Timing::pal();
        let vic = profile
            .vic_line(raster_line, state)
            .ok_or(AssembleError::InvalidRasterLine {
                source_line,
                raster_line,
            })?;
        let schedule = vic.bus.schedule(&instructions[start..end]);
        for scheduled in &schedule.instructions {
            if scheduled.stalled_cycles > 0 {
                diagnostics.push(
                    diagnostic::Diagnostic::info(
                        "C64_VIC_STALL",
                        "VIC-II bus activity stretches instruction timing",
                    )
                    .with_primary(
                        scheduled.source.clone(),
                        "instruction stalls on VIC-II bus ownership",
                    )
                    .with_timing(diagnostic::TimingDiagnostic {
                        raster_line,
                        nominal_cycles: scheduled.nominal_cycles,
                        stalled_cycles: scheduled.stalled_cycles,
                        actual_cycles: scheduled.end_cycle - scheduled.start_cycle,
                    }),
                );
            }
        }
        let contract = c64::RasterContract {
            line: raster_line,
            scheduled_end_cycle: schedule.end_cycle,
            scheduled_stall_cycles: schedule.stalled_cycles,
            schedule: schedule.instructions,
            line_cycles: profile.cycles_per_line,
            available_cycles: vic.cpu_available_cycles,
            vic_stolen_cycles: vic.vic_stolen_cycles,
            badline_stolen_cycles: vic.badline_stolen_cycles,
            sprite_stolen_cycles: vic.sprite_stolen_cycles,
            active_sprites: vic.active_sprites,
            sprite_dma_mask: vic.sprite_dma_mask,
            badline: vic.badline,
            min_cycles: range.min,
            max_cycles: range.max,
        };
        if !contract.fits() {
            let raw_line = source.lines().nth(source_line - 1).unwrap_or("");
            let column_start = raw_line.len().saturating_sub(raw_line.trim_start().len()) + 1;
            diagnostics.push(
                diagnostic::Diagnostic::error(
                    "C64_RASTER_BUDGET",
                    format!(
                        "raster {} requires up to {} CPU cycles but only {} are available",
                        raster_line, range.max, vic.cpu_available_cycles
                    ),
                )
                .with_primary(
                    SourceSpan {
                        file_id: 0,
                        line: source_line,
                        column_start,
                        column_end: raw_line.len() + 1,
                    },
                    "raster timing contract is exceeded",
                )
                .with_note(format!(
                    "{} VIC-II cycles are unavailable on this raster line",
                    vic.vic_stolen_cycles
                )),
            );
        }
        raster_contracts.push(contract);
    }
    for (index, raw) in source.lines().enumerate() {
        let line = raw.split(';').next().unwrap_or("").trim();
        if line.starts_with(".raster") {
            continue;
        }
        if let Some(rest) = line.strip_prefix(".assert_cycles") {
            let rest = rest.trim();
            let budget_text = rest.strip_prefix("<=").map(str::trim).ok_or_else(|| {
                AssembleError::InvalidExpression {
                    line: index + 1,
                    text: rest.to_string(),
                }
            })?;
            let budget = expr::eval(budget_text, &symbols, index + 1)?.ok_or_else(|| {
                AssembleError::UnresolvedSymbol {
                    line: index + 1,
                    name: budget_text.to_string(),
                }
            })? as u64;
            if !cycle_range.fits(budget) {
                let column_start = raw.len().saturating_sub(raw.trim_start().len()) + 1;
                diagnostics.push(
                    diagnostic::Diagnostic::error(
                        "CYCLE_BUDGET",
                        format!(
                            "worst-case execution requires {} cycles but budget is {}",
                            cycle_range.max, budget
                        ),
                    )
                    .with_primary(
                        SourceSpan {
                            file_id: 0,
                            line: index + 1,
                            column_start,
                            column_end: raw.len() + 1,
                        },
                        "cycle guarantee is exceeded",
                    )
                    .with_note(format!("best-case execution is {} cycles", cycle_range.min)),
                );
            }
        }
    }

    let mut hardware_writes = Vec::new();
    let mut known_a: Option<u8> = None;
    for instruction in &instructions {
        let offset = instruction.address.wrapping_sub(origin) as usize;
        match (instruction.opcode.mnemonic, instruction.opcode.mode) {
            ("LDA", AddressingMode::Immediate) => {
                known_a = bytes.get(offset + 1).copied();
            }
            ("STA", AddressingMode::Absolute) => {
                if let (Some(value), Some(lo), Some(hi)) =
                    (known_a, bytes.get(offset + 1), bytes.get(offset + 2))
                {
                    let address = u16::from_le_bytes([*lo, *hi]);
                    if let Some(register) = c64_registers::register_by_address(address) {
                        hardware_writes.push(HardwareWrite {
                            source: instruction.source.clone(),
                            register,
                            value,
                        });
                    }
                }
            }
            ("STA", _) => {}
            (
                "ADC" | "AND" | "ASL" | "EOR" | "LAX" | "LDA" | "LSR" | "ORA" | "PLA" | "ROL"
                | "ROR" | "SBC" | "TXA" | "TYA",
                _,
            ) => known_a = None,
            _ => {}
        }
    }

    Ok(Assembly {
        bytes,
        symbols,
        symbol_definitions,
        origin,
        instructions,
        cycles,
        cycle_range,
        raster_contracts,
        hardware_writes,
        source_map,
        diagnostics,
    })
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
    parsed.map_err(|_| AssembleError::InvalidNumber {
        line,
        text: text.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_known_immediate_accumulator_write_to_hardware() {
        let assembly = assemble("lda #$1b\nsta $d011\n", Target::c64()).unwrap();
        assert_eq!(assembly.hardware_writes.len(), 1);
        let write = &assembly.hardware_writes[0];
        assert_eq!(write.register.name, "VIC_CTRL1");
        assert_eq!(write.value, 0x1b);
        assert_eq!(write.source.line, 2);
    }

    #[test]
    fn unknown_accumulator_value_is_not_reported_as_hardware_write() {
        let assembly = assemble("lda #$1b\neor #$ff\nsta $d011\n", Target::c64()).unwrap();
        assert!(assembly.hardware_writes.is_empty());
    }

    fn assembly_exposes_diagnostic_severity_views() {
        let failed =
            assemble("lda $1234,x\nbne $10\n.assert_cycles <= 8\n", Target::c64()).unwrap();
        assert!(failed.has_errors());
        assert_eq!(failed.errors().count(), 1);
        assert_eq!(failed.warnings().count(), 0);

        let clean = assemble("nop\n", Target::c64()).unwrap();
        assert!(!clean.has_errors());
        assert_eq!(clean.errors().count(), 0);
    }

    #[test]
    fn vic_stalls_emit_structured_source_diagnostics() {
        let mut source = ".vic_display on\n.vic_yscroll 0\n.raster 48 {\n".to_string();
        for _ in 0..8 {
            source.push_str("nop\n");
        }
        source.push_str("}\n");
        let out = assemble(&source, Target::c64()).unwrap();
        let diagnostic = out
            .diagnostics
            .iter()
            .find(|d| d.code == "C64_VIC_STALL")
            .unwrap();
        assert_eq!(diagnostic.severity, diagnostic::Severity::Info);
        assert_eq!(diagnostic.timing.as_ref().unwrap().raster_line, 48);
        assert!(diagnostic.primary.as_ref().unwrap().span.line >= 4);
    }

    #[test]
    fn assembly_registers_main_source() {
        let out = assemble("nop\n", Target::c64()).unwrap();
        assert_eq!(out.source_map.file(0).unwrap().name, "<input>");
        assert_eq!(out.instructions[0].source.file_id, 0);
        assert!(out.instructions[0].expansion.frames.is_empty());
    }

    #[test]
    fn supports_text_and_forward_origin_gap() {
        let out = assemble(
            "* = $0801\n.byte 1,2\n.text \"AB\"\n* = $0808\n.byte 3\n",
            Target::c64(),
        )
        .unwrap();
        assert_eq!(out.origin, 0x0801);
        assert_eq!(out.bytes, vec![1, 2, b'A', b'B', 0, 0, 0, 3]);
    }

    #[test]
    fn c64scene_hello_raster_syntax_and_relative_branch() {
        let src = "* = $0801\n.byte $0c,$08,$0a,$00,$9e,$20,$32,$30,$36,$31,$00,$00,$00\nstart:\nsei\nlda #$00\nsta $d020\nsta $d021\nmain:\nlda $d012\nwait:\ncmp $d012\nbeq wait\nlda $d012\nlsr\nlsr\nand #$0f\nsta $d020\njmp main\n";
        let out = assemble(src, Target::c64()).unwrap();
        assert_eq!(out.origin, 0x0801);
        assert_eq!(out.symbols["start"], 0x080e);
        assert_eq!(out.bytes[24], 0xf0);
        assert_eq!(out.bytes[25], 0xfb);
    }

    #[test]
    fn assembles_minimal_c64_code_and_tracks_cycles() {
        let out = assemble(
            ".org $080d\nstart:\n  sei\n  lda #$06\n  sta $d020\n  rts\n",
            Target::c64(),
        )
        .unwrap();
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
    fn pal_raster_contract_uses_machine_line_budget() {
        let out = assemble("sei\n.raster 100 {\nnop\nrts\n}\n", Target::c64()).unwrap();
        assert_eq!(out.raster_contracts.len(), 1);
        assert_eq!(out.raster_contracts[0].available_cycles, 63);
        assert_eq!(out.raster_contracts[0].vic_stolen_cycles, 0);
        assert!(!out.raster_contracts[0].badline);
        assert_eq!(out.raster_contracts[0].min_cycles, 8);
        assert_eq!(out.raster_contracts[0].max_cycles, 8);
        assert!(out.raster_contracts[0].fits());
    }

    #[test]
    fn raster_budget_violation_is_structured_error() {
        let mut source = ".vic_display on\n.vic_yscroll 0\n.raster 48 {\n".to_string();
        for _ in 0..12 {
            source.push_str("rts\n");
        }
        source.push_str("}\n");
        let out = assemble(&source, Target::c64()).unwrap();
        let diagnostic = out
            .diagnostics
            .iter()
            .find(|d| d.code == "C64_RASTER_BUDGET")
            .unwrap();
        assert_eq!(diagnostic.severity, diagnostic::Severity::Error);
        assert_eq!(diagnostic.primary.as_ref().unwrap().span.line, 3);
        assert!(!out.raster_contracts[0].fits());
    }

    #[test]
    fn raster_contract_accounts_for_pal_badline() {
        let out = assemble(".raster 51 {\nnop\n}\n", Target::c64()).unwrap();
        let contract = &out.raster_contracts[0];
        assert!(contract.badline);
        assert_eq!(contract.line_cycles, 63);
        assert_eq!(contract.vic_stolen_cycles, 40);
        assert_eq!(contract.available_cycles, 23);
        assert_eq!(contract.margin(), 21);
        assert_eq!(contract.scheduled_end_cycle, 2);
        assert_eq!(contract.scheduled_stall_cycles, 0);
        assert_eq!(contract.schedule.len(), 1);
        assert_eq!(contract.schedule[0].address, out.instructions[1].address);
        assert_eq!(
            contract.schedule[0].source.line,
            out.instructions[1].source.line
        );
    }

    #[test]
    fn raster_schedule_reports_vic_stalls() {
        let source = ".vic_display on\n.vic_yscroll 0\n.raster 48 {\n";
        let mut owned = source.to_string();
        for _ in 0..8 {
            owned.push_str("nop\n");
        }
        owned.push_str("}\n");
        let out = assemble(&owned, Target::c64()).unwrap();
        let contract = &out.raster_contracts[0];
        assert!(contract.badline);
        assert!(contract.scheduled_stall_cycles > 0);
        assert!(contract.scheduled_end_cycle > contract.min_cycles as u16);
    }

    #[test]
    fn raster_contract_keeps_per_instruction_schedule() {
        let out = assemble(
            ".vic_display off\n.raster 100 {\nlda #1\nnop\nrts\n}\n",
            Target::c64(),
        )
        .unwrap();
        let contract = &out.raster_contracts[0];
        assert_eq!(contract.schedule.len(), 3);
        assert_eq!(contract.schedule[0].nominal_cycles, 2);
        assert_eq!(contract.schedule[0].source.line, 3);
        assert_eq!(contract.schedule[0].start_cycle, 0);
        assert_eq!(contract.schedule[0].end_cycle, 2);
        assert_eq!(contract.schedule[1].start_cycle, 2);
        assert_eq!(contract.schedule[2].end_cycle, 10);
    }

    #[test]
    fn vic_state_changes_badline_analysis() {
        let normal = assemble(".vic_display off\n.raster 51 {\nnop\n}\n", Target::c64()).unwrap();
        assert!(!normal.raster_contracts[0].badline);
        assert_eq!(normal.raster_contracts[0].available_cycles, 63);

        let shifted = assemble(".vic_yscroll 4\n.raster 51 {\nnop\n}\n", Target::c64()).unwrap();
        assert!(!shifted.raster_contracts[0].badline);

        let bad = assemble(".vic_yscroll 4\n.raster 52 {\nnop\n}\n", Target::c64()).unwrap();
        assert!(bad.raster_contracts[0].badline);
        assert_eq!(bad.raster_contracts[0].available_cycles, 23);
    }

    #[test]
    fn sprite_dma_reduces_raster_budget() {
        let out = assemble(".vic_display off\n.vic_sprites 7\n.vic_sprite_y 0 90\n.vic_sprite_y 1 90\n.vic_sprite_y 2 90\n.raster 100 {\nnop\n}\n", Target::c64()).unwrap();
        let contract = &out.raster_contracts[0];
        assert_eq!(contract.active_sprites, 3);
        assert_eq!(contract.sprite_stolen_cycles, 6);
        assert_eq!(contract.vic_stolen_cycles, 6);
        assert_eq!(contract.available_cycles, 57);
    }

    #[test]
    fn sprite_y_state_selects_dma_per_raster() {
        let out = assemble(".vic_display off\n.vic_sprites 1\n.vic_sprite_y 0 100\n.raster 99 {\nnop\n}\n.raster 100 {\nnop\n}\n.raster 121 {\nnop\n}\n", Target::c64()).unwrap();
        assert_eq!(out.raster_contracts[0].active_sprites, 0);
        assert_eq!(out.raster_contracts[1].sprite_dma_mask, 1);
        assert_eq!(out.raster_contracts[1].active_sprites, 1);
        assert_eq!(out.raster_contracts[2].active_sprites, 0);
    }

    #[test]
    fn raster_blocks_are_independently_scoped() {
        let out = assemble(
            ".raster 100 {\nnop\n}\n.raster 101 {\nrts\n}\n",
            Target::c64(),
        )
        .unwrap();
        assert_eq!(out.raster_contracts[0].min_cycles, 2);
        assert_eq!(out.raster_contracts[1].min_cycles, 6);
    }

    #[test]
    fn cycle_contract_uses_worst_case_timing() {
        let ok = assemble("lda $1234,x\nbne $10\n.assert_cycles <= 9\n", Target::c64());
        assert!(ok.is_ok());
        let failed =
            assemble("lda $1234,x\nbne $10\n.assert_cycles <= 8\n", Target::c64()).unwrap();
        let diagnostic = failed
            .diagnostics
            .iter()
            .find(|d| d.code == "CYCLE_BUDGET")
            .unwrap();
        assert_eq!(diagnostic.severity, diagnostic::Severity::Error);
        assert_eq!(diagnostic.primary.as_ref().unwrap().span.line, 3);
        assert_eq!(failed.cycle_range.max, 9);
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
    fn relative_branch_accepts_current_pc_expression() {
        let out = assemble(".org $c000\nbeq *+2\nnop\n", Target::c64()).unwrap();
        assert_eq!(out.bytes, vec![0xf0, 0x00, 0xea]);
    }

    #[test]
    fn forward_reference_can_shrink_to_zero_page() {
        let out = assemble(
            ".org $0020\nlda table\nnop\ntable:\n.byte 1\n",
            Target::c64(),
        )
        .unwrap();
        assert_eq!(out.symbols["table"], 0x0023);
        assert_eq!(out.bytes, vec![0xa5, 0x23, 0xea, 0x01]);
    }
}
