use std::collections::BTreeMap;

use crate::AssembleError;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Number(u16),
    Ident(String),
    Plus,
    Minus,
    LParen,
    RParen,
    Low,
    High,
    Eq,
    Ne,
    Le,
    Ge,
}

pub fn eval(text: &str, symbols: &BTreeMap<String, u16>, line: usize) -> Result<Option<u16>, AssembleError> {
    let tokens = lex(text, line)?;
    let mut parser = Parser { tokens: &tokens, pos: 0, symbols, line };
    let value = parser.expr()?;
    if parser.pos != tokens.len() {
        return Err(AssembleError::InvalidExpression { line, text: text.to_string() });
    }
    Ok(value)
}

fn lex(text: &str, line: usize) -> Result<Vec<Token>, AssembleError> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            c if c.is_whitespace() => i += 1,
            '+' => { out.push(Token::Plus); i += 1; }
            '-' => { out.push(Token::Minus); i += 1; }
            '(' => { out.push(Token::LParen); i += 1; }
            ')' => { out.push(Token::RParen); i += 1; }
            '<' if chars.get(i + 1) == Some(&'=') => { out.push(Token::Le); i += 2; }
            '>' if chars.get(i + 1) == Some(&'=') => { out.push(Token::Ge); i += 2; }
            '=' if chars.get(i + 1) == Some(&'=') => { out.push(Token::Eq); i += 2; }
            '!' if chars.get(i + 1) == Some(&'=') => { out.push(Token::Ne); i += 2; }
            '<' => { out.push(Token::Low); i += 1; }
            '>' => { out.push(Token::High); i += 1; }
            '$' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i].is_ascii_hexdigit() { i += 1; }
                let s: String = chars[start..i].iter().collect();
                let n = u16::from_str_radix(&s, 16).map_err(|_| AssembleError::InvalidExpression { line, text: text.to_string() })?;
                out.push(Token::Number(n));
            }
            c if c.is_ascii_digit() => {
                let start = i;
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
                let s: String = chars[start..i].iter().collect();
                let n = s.parse::<u16>().map_err(|_| AssembleError::InvalidExpression { line, text: text.to_string() })?;
                out.push(Token::Number(n));
            }
            c if c == '_' || c.is_ascii_alphabetic() => {
                let start = i;
                i += 1;
                while i < chars.len() && (chars[i] == '_' || chars[i].is_ascii_alphanumeric()) { i += 1; }
                out.push(Token::Ident(chars[start..i].iter().collect()));
            }
            _ => return Err(AssembleError::InvalidExpression { line, text: text.to_string() }),
        }
    }
    Ok(out)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    symbols: &'a BTreeMap<String, u16>,
    line: usize,
}

impl Parser<'_> {
    fn comparison(&mut self) -> Result<Option<u16>, AssembleError> {
        let lhs = self.expr()?;
        let Some(op) = self.tokens.get(self.pos).cloned() else { return Ok(lhs); };
        if !matches!(op, Token::Eq | Token::Ne | Token::Le | Token::Ge) { return Ok(lhs); }
        self.pos += 1;
        let rhs = self.expr()?;
        Ok(match (lhs, rhs) {
            (Some(a), Some(b)) => Some(match op {
                Token::Eq => (a == b) as u16,
                Token::Ne => (a != b) as u16,
                Token::Le => (a <= b) as u16,
                Token::Ge => (a >= b) as u16,
                _ => unreachable!(),
            }),
            _ => None,
        })
    }

    fn expr(&mut self) -> Result<Option<u16>, AssembleError> {
        let mut lhs = self.unary()?;
        while let Some(token) = self.tokens.get(self.pos) {
            let add = match token { Token::Plus => true, Token::Minus => false, _ => break };
            self.pos += 1;
            let rhs = self.unary()?;
            lhs = match (lhs, rhs) {
                (Some(a), Some(b)) => Some(if add { a.wrapping_add(b) } else { a.wrapping_sub(b) }),
                _ => None,
            };
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Option<u16>, AssembleError> {
        match self.tokens.get(self.pos) {
            Some(Token::Low) => { self.pos += 1; Ok(self.unary()?.map(|v| v & 0xff)) }
            Some(Token::High) => { self.pos += 1; Ok(self.unary()?.map(|v| v >> 8)) }
            Some(Token::Minus) => { self.pos += 1; Ok(self.unary()?.map(|v| 0u16.wrapping_sub(v))) }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<Option<u16>, AssembleError> {
        let token = self.tokens.get(self.pos).cloned().ok_or_else(|| AssembleError::InvalidExpression { line: self.line, text: String::new() })?;
        self.pos += 1;
        match token {
            Token::Number(v) => Ok(Some(v)),
            Token::Ident(name) => Ok(self.symbols.get(&name).copied()),
            Token::LParen => {
                let v = self.expr()?;
                if !matches!(self.tokens.get(self.pos), Some(Token::RParen)) {
                    return Err(AssembleError::InvalidExpression { line: self.line, text: "missing ')'".into() });
                }
                self.pos += 1;
                Ok(v)
            }
            _ => Err(AssembleError::InvalidExpression { line: self.line, text: "expected value".into() }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_scene_friendly_address_expressions() {
        let symbols = BTreeMap::from([("irq".to_string(), 0xc123), ("table".to_string(), 0x2000)]);
        assert_eq!(eval("<irq", &symbols, 1).unwrap(), Some(0x23));
        assert_eq!(eval(">irq", &symbols, 1).unwrap(), Some(0xc1));
        assert_eq!(eval("table + 8 - 2", &symbols, 1).unwrap(), Some(0x2006));
        assert_eq!(eval("<(irq + 1)", &symbols, 1).unwrap(), Some(0x24));
        assert_eq!(eval("table >= $2000", &symbols, 1).unwrap(), Some(1));
        assert_eq!(eval("table == irq", &symbols, 1).unwrap(), Some(0));
    }

    #[test]
    fn unresolved_symbol_is_deferred() {
        assert_eq!(eval("future + 1", &BTreeMap::new(), 1).unwrap(), None);
    }
}
