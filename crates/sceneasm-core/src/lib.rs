use std::collections::BTreeMap;

use thiserror::Error;

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
pub struct Assembly {
    pub bytes: Vec<u8>,
    pub symbols: BTreeMap<String, u16>,
    pub origin: u16,
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
    let mut origin = target.origin;
    let mut pc = origin;
    let mut bytes = Vec::new();
    let mut symbols = BTreeMap::new();

    for (index, raw) in source.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.split(';').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix(".org") {
            let value = parse_u16(rest.trim(), line_no)?;
            if bytes.is_empty() {
                origin = value;
                pc = value;
                continue;
            }
        }

        if let Some(label) = line.strip_suffix(':') {
            let name = label.trim().to_string();
            if symbols.insert(name.clone(), pc).is_some() {
                return Err(AssembleError::DuplicateSymbol { line: line_no, name });
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix(".byte") {
            for token in rest.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let value = parse_u16(token, line_no)?;
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
        match upper.as_str() {
            "SEI" => emit(&mut bytes, &mut pc, &[0x78]),
            "CLI" => emit(&mut bytes, &mut pc, &[0x58]),
            "NOP" => emit(&mut bytes, &mut pc, &[0xea]),
            "RTS" => emit(&mut bytes, &mut pc, &[0x60]),
            "BRK" => emit(&mut bytes, &mut pc, &[0x00]),
            _ if upper.starts_with("LDA #") => {
                let arg = line[5..].trim();
                let value = parse_u16(arg, line_no)?;
                if value > 0xff {
                    return Err(AssembleError::InvalidNumber { line: line_no, text: arg.into() });
                }
                emit(&mut bytes, &mut pc, &[0xa9, value as u8]);
            }
            _ if upper.starts_with("STA ") => {
                let arg = line[4..].trim();
                let addr = parse_u16(arg, line_no)?;
                emit(&mut bytes, &mut pc, &[0x8d, addr as u8, (addr >> 8) as u8]);
            }
            _ if upper.starts_with("JMP ") => {
                let arg = line[4..].trim();
                let addr = parse_u16(arg, line_no)?;
                emit(&mut bytes, &mut pc, &[0x4c, addr as u8, (addr >> 8) as u8]);
            }
            _ => {
                return Err(AssembleError::UnsupportedStatement {
                    line: line_no,
                    text: line.to_string(),
                })
            }
        }
    }

    Ok(Assembly { bytes, symbols, origin })
}

fn emit(bytes: &mut Vec<u8>, pc: &mut u16, data: &[u8]) {
    bytes.extend_from_slice(data);
    *pc = pc.wrapping_add(data.len() as u16);
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
    fn assembles_minimal_c64_code() {
        let out = assemble(
            ".org $080d\nstart:\n  sei\n  lda #$06\n  sta $d020\n  rts\n",
            Target::c64(),
        )
        .unwrap();

        assert_eq!(out.origin, 0x080d);
        assert_eq!(out.symbols["start"], 0x080d);
        assert_eq!(out.bytes, vec![0x78, 0xa9, 0x06, 0x8d, 0x20, 0xd0, 0x60]);
    }
}
