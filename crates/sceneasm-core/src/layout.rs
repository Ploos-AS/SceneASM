use std::collections::BTreeMap;

use crate::{opcode_with_policy, AddressingMode, AssembleError, UndocumentedPolicy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub origin: u16,
    pub symbols: BTreeMap<String, u16>,
}

pub fn resolve_value(text: &str, symbols: &BTreeMap<String, u16>, line: usize) -> Result<Option<u16>, AssembleError> {
    let text = text.trim();
    if let Some(value) = parse_literal(text) {
        return Ok(Some(value));
    }
    if is_identifier(text) {
        return Ok(symbols.get(text).copied());
    }
    Err(AssembleError::InvalidNumber { line, text: text.to_string() })
}

pub fn choose_mode(
    mnemonic: &str,
    operand: Option<&str>,
    symbols: &BTreeMap<String, u16>,
    policy: UndocumentedPolicy,
    line: usize,
) -> Result<(AddressingMode, Option<u16>), AssembleError> {
    match operand {
        None => Ok((AddressingMode::Implied, None)),
        Some(arg) if arg.starts_with('#') => {
            let value = resolve_value(arg.trim_start_matches('#').trim(), symbols, line)?;
            Ok((AddressingMode::Immediate, value))
        }
        Some(arg) => {
            let value = resolve_value(arg, symbols, line)?;
            let can_zp = opcode_with_policy(mnemonic, AddressingMode::ZeroPage, policy).is_some();
            let mode = if can_zp && value.is_some_and(|v| v <= 0xff) {
                AddressingMode::ZeroPage
            } else {
                AddressingMode::Absolute
            };
            Ok((mode, value))
        }
    }
}

pub fn layout(source: &str, default_origin: u16, policy: UndocumentedPolicy) -> Result<Layout, AssembleError> {
    let mut previous = BTreeMap::new();
    let mut origin = default_origin;

    // Start pessimistically: unresolved operands use absolute addressing.
    // Re-run until labels stop moving as zero-page opportunities become known.
    for _ in 0..16 {
        let mut symbols = BTreeMap::new();
        let mut pc = origin;
        let mut current_origin = origin;
        let mut current_policy = policy;

        for (index, raw) in source.lines().enumerate() {
            let line_no = index + 1;
            let line = raw.split(';').next().unwrap_or("").trim();
            if line.is_empty() { continue; }

            if let Some(rest) = line.strip_prefix(".org") {
                let value = resolve_value(rest.trim(), &previous, line_no)?
                    .ok_or_else(|| AssembleError::UnresolvedSymbol { line: line_no, name: rest.trim().to_string() })?;
                current_origin = value;
                pc = value;
                continue;
            }
            if let Some(label) = line.strip_suffix(':') {
                let name = label.trim().to_string();
                if symbols.insert(name.clone(), pc).is_some() {
                    return Err(AssembleError::DuplicateSymbol { line: line_no, name });
                }
                continue;
            }
            if let Some(rest) = line.strip_prefix(".undocumented") {
                current_policy = match rest.trim().to_ascii_lowercase().as_str() {
                    "deny" => UndocumentedPolicy::Deny,
                    "stable" => UndocumentedPolicy::Stable,
                    "all" => UndocumentedPolicy::All,
                    _ => return Err(AssembleError::UnsupportedStatement { line: line_no, text: line.to_string() }),
                };
                continue;
            }
            if let Some(rest) = line.strip_prefix(".byte") {
                pc = pc.wrapping_add(rest.split(',').filter(|s| !s.trim().is_empty()).count() as u16);
                continue;
            }

            let upper = line.to_ascii_uppercase();
            let (mnemonic, operand) = if let Some((m, _)) = upper.split_once(' ') {
                (m, line.split_once(' ').map(|(_, a)| a.trim()))
            } else { (upper.as_str(), None) };
            let (mode, _) = choose_mode(mnemonic, operand, &previous, current_policy, line_no)?;
            let op = opcode_with_policy(mnemonic, mode, current_policy)
                .ok_or_else(|| AssembleError::UnsupportedStatement { line: line_no, text: line.to_string() })?;
            pc = pc.wrapping_add(op.bytes as u16);
        }

        origin = current_origin;
        if symbols == previous {
            return Ok(Layout { origin, symbols });
        }
        previous = symbols;
    }

    Err(AssembleError::LayoutDidNotConverge)
}

fn parse_literal(text: &str) -> Option<u16> {
    if let Some(hex) = text.strip_prefix('$') { u16::from_str_radix(hex, 16).ok() }
    else if let Some(hex) = text.strip_prefix("0x") { u16::from_str_radix(hex, 16).ok() }
    else { text.parse::<u16>().ok() }
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}
