//! Recursive-descent calculator (try_calc), extracted from search.rs.
// ── Calculator: recursive-descent expression parser ────────────────────────
// Supports: +, -, *, /, ^, %, parentheses, unary minus
// Named functions: sqrt, abs, round, floor, ceil, sin, cos, tan, log, ln
// Special form: "N% of M" → N/100 * M
pub fn try_calc(input: &str) -> Option<f64> {
    let s = input.trim();
    // Must contain at least one digit to be a math expression
    if !s.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }

    // Handle "X% of Y" shorthand
    let s = if let Some(pct_of) = try_pct_of(s) {
        return Some(pct_of);
    } else {
        s.to_string()
    };

    let tokens = tokenize(&s)?;
    // Guard the recursive-descent parser against a stack overflow on pathological input:
    // deeply nested parentheses recurse one frame per level, and search runs on a worker
    // thread. No real calculation nests anywhere near this deep.
    let mut depth = 0i32;
    let mut max_depth = 0i32;
    for t in &tokens {
        match t {
            Token::LParen => {
                depth += 1;
                max_depth = max_depth.max(depth);
            }
            Token::RParen => depth -= 1,
            _ => {}
        }
    }
    if max_depth > 128 {
        return None;
    }
    let mut pos = 0usize;
    let result = parse_expr(&tokens, &mut pos)?;
    // Consume any trailing whitespace tokens
    while pos < tokens.len() {
        if tokens[pos] != Token::EOF {
            return None;
        }
        pos += 1;
    }
    if result.is_nan() || result.is_infinite() {
        return None;
    }
    Some(result)
}

fn try_pct_of(s: &str) -> Option<f64> {
    // Match "N% of M" case-insensitively.
    // SAFETY: Use ASCII-only case-insensitive search on the original bytes so the
    // returned index is always valid for slicing `s`. Lowercasing can change UTF-8
    // byte length (e.g. Kelvin sign \u{212A} lowercases to 'k'), so we must never
    // reuse an offset from a lowercased copy to slice the original string.
    let needle = b"% of ";
    let bytes = s.as_bytes();
    let idx = (0..bytes.len().saturating_sub(needle.len() - 1)).find(|&i| {
        bytes[i..]
            .get(..needle.len())
            .map_or(false, |w| w.eq_ignore_ascii_case(needle))
    })?;
    let pct_str = s[..idx].trim();
    let rest_str = s[idx + needle.len()..].trim();
    let pct: f64 = pct_str.parse().ok()?;
    let base: f64 = rest_str.parse().ok()?;
    Some(pct / 100.0 * base)
}

#[derive(Debug, PartialEq, Clone)]
enum Token {
    Num(f64),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
    LParen,
    RParen,
    Ident(String),
    EOF,
}

fn tokenize(s: &str) -> Option<Vec<Token>> {
    let chars: Vec<char> = s.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' => {
                i += 1;
            }
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' | '×' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' | '÷' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '^' => {
                tokens.push(Token::Caret);
                i += 1;
            }
            '%' => {
                tokens.push(Token::Percent);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            ',' => {
                i += 1;
            } // ignore comma separators
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let num_str: String = chars[start..i].iter().collect();
                let n: f64 = num_str.parse().ok()?;
                tokens.push(Token::Num(n));
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                tokens.push(Token::Ident(word.to_lowercase()));
            }
            _ => return None, // unknown character → not a math expression
        }
    }
    tokens.push(Token::EOF);
    Some(tokens)
}

// expr = term (('+' | '-') term)*
fn parse_expr(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    let mut left = parse_term(tokens, pos)?;
    loop {
        match tokens.get(*pos) {
            Some(Token::Plus) => {
                *pos += 1;
                left += parse_term(tokens, pos)?;
            }
            Some(Token::Minus) => {
                *pos += 1;
                left -= parse_term(tokens, pos)?;
            }
            _ => break,
        }
    }
    Some(left)
}

// term = power (('*' | '/' | '%') power)*
fn parse_term(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    let mut left = parse_power(tokens, pos)?;
    loop {
        match tokens.get(*pos) {
            Some(Token::Star) => {
                *pos += 1;
                left *= parse_power(tokens, pos)?;
            }
            Some(Token::Slash) => {
                *pos += 1;
                let r = parse_power(tokens, pos)?;
                if r == 0.0 {
                    return None;
                }
                left /= r;
            }
            Some(Token::Percent) => {
                // Check if next token is 'of' (handled earlier) or treat as modulo
                *pos += 1;
                match tokens.get(*pos) {
                    Some(Token::Ident(w)) if w == "of" => {
                        *pos += 1;
                        let base = parse_power(tokens, pos)?;
                        left = left / 100.0 * base;
                    }
                    _ => {
                        // treat as percentage of the next value if present, else /100
                        left = left / 100.0;
                    }
                }
            }
            _ => break,
        }
    }
    Some(left)
}

// power = unary ('^' power)?  (right-associative)
fn parse_power(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    let base = parse_unary(tokens, pos)?;
    if matches!(tokens.get(*pos), Some(Token::Caret)) {
        *pos += 1;
        let exp = parse_power(tokens, pos)?;
        Some(base.powf(exp))
    } else {
        Some(base)
    }
}

// unary = '-' unary | primary
fn parse_unary(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    if matches!(tokens.get(*pos), Some(Token::Minus)) {
        *pos += 1;
        Some(-parse_unary(tokens, pos)?)
    } else {
        parse_primary(tokens, pos)
    }
}

// primary = number | ident '(' expr ')' | '(' expr ')'
fn parse_primary(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    match tokens.get(*pos)?.clone() {
        Token::Num(n) => {
            *pos += 1;
            Some(n)
        }
        Token::LParen => {
            *pos += 1;
            let val = parse_expr(tokens, pos)?;
            if tokens.get(*pos) == Some(&Token::RParen) {
                *pos += 1;
            }
            Some(val)
        }
        Token::Ident(name) => {
            *pos += 1;
            // Named functions expect a parenthesised argument
            if tokens.get(*pos) == Some(&Token::LParen) {
                *pos += 1;
                let arg = parse_expr(tokens, pos)?;
                if tokens.get(*pos) == Some(&Token::RParen) {
                    *pos += 1;
                }
                match name.as_str() {
                    "sqrt" => Some(arg.sqrt()),
                    "abs" => Some(arg.abs()),
                    "round" => Some(arg.round()),
                    "floor" => Some(arg.floor()),
                    "ceil" => Some(arg.ceil()),
                    "sin" => Some(arg.to_radians().sin()),
                    "cos" => Some(arg.to_radians().cos()),
                    "tan" => Some(arg.to_radians().tan()),
                    "log" => Some(arg.log10()),
                    "ln" => Some(arg.ln()),
                    _ => None,
                }
            } else {
                // Named constants
                match name.as_str() {
                    "pi" | "π" => Some(std::f64::consts::PI),
                    "e" => Some(std::f64::consts::E),
                    _ => None,
                }
            }
        }
        _ => None,
    }
}
