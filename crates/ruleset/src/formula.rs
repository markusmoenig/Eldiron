use std::collections::BTreeSet;

pub fn evaluate_formula<F>(source: &str, resolve: F) -> Option<f32>
where
    F: FnMut(&str) -> f32,
{
    FormulaParser::new(source, resolve, false).parse()
}

pub fn formula_is_valid(source: &str) -> bool {
    FormulaParser::new(source, |_| 1.0, true).parse().is_some()
}

pub fn formula_identifiers(source: &str) -> BTreeSet<String> {
    let bytes = source.as_bytes();
    let mut identifiers = BTreeSet::new();
    let mut index = 0;
    while index < bytes.len() {
        let ch = bytes[index];
        if !ch.is_ascii_alphabetic() && ch != b'_' {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len()
            && (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'_' | b'.'))
        {
            index += 1;
        }
        let Ok(identifier) = std::str::from_utf8(&bytes[start..index]) else {
            continue;
        };
        let mut next = index;
        while next < bytes.len() && bytes[next].is_ascii_whitespace() {
            next += 1;
        }
        if next < bytes.len() && bytes[next] == b'(' {
            continue;
        }
        identifiers.insert(identifier.to_string());
    }
    identifiers
}

struct FormulaParser<'a, F>
where
    F: FnMut(&str) -> f32,
{
    source: &'a [u8],
    index: usize,
    resolve: F,
    validate_only: bool,
}

impl<'a, F> FormulaParser<'a, F>
where
    F: FnMut(&str) -> f32,
{
    fn new(source: &'a str, resolve: F, validate_only: bool) -> Self {
        Self {
            source: source.as_bytes(),
            index: 0,
            resolve,
            validate_only,
        }
    }

    fn parse(mut self) -> Option<f32> {
        let value = self.parse_or()?;
        self.skip_whitespace();
        (self.index == self.source.len() && value.is_finite()).then_some(value)
    }

    fn skip_whitespace(&mut self) {
        while self.index < self.source.len() && self.source[self.index].is_ascii_whitespace() {
            self.index += 1;
        }
    }

    fn consume(&mut self, ch: u8) -> bool {
        self.skip_whitespace();
        if self.index < self.source.len() && self.source[self.index] == ch {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn parse_or(&mut self) -> Option<f32> {
        let mut value = self.parse_and()?;
        loop {
            self.skip_whitespace();
            if self.index + 1 < self.source.len()
                && self.source[self.index] == b'|'
                && self.source[self.index + 1] == b'|'
            {
                self.index += 2;
                let rhs = self.parse_and()?;
                value = (value != 0.0 || rhs != 0.0) as u8 as f32;
            } else {
                return Some(value);
            }
        }
    }

    fn parse_and(&mut self) -> Option<f32> {
        let mut value = self.parse_comparison()?;
        loop {
            self.skip_whitespace();
            if self.index + 1 < self.source.len()
                && self.source[self.index] == b'&'
                && self.source[self.index + 1] == b'&'
            {
                self.index += 2;
                let rhs = self.parse_comparison()?;
                value = (value != 0.0 && rhs != 0.0) as u8 as f32;
            } else {
                return Some(value);
            }
        }
    }

    fn parse_comparison(&mut self) -> Option<f32> {
        let mut value = self.parse_expression()?;
        loop {
            self.skip_whitespace();
            let pair = (self.index + 1 < self.source.len())
                .then(|| (self.source[self.index], self.source[self.index + 1]));
            let result = match pair {
                Some((b'=', b'=')) => {
                    self.index += 2;
                    Some(((value - self.parse_expression()?).abs() <= f32::EPSILON) as u8 as f32)
                }
                Some((b'!', b'=')) => {
                    self.index += 2;
                    Some(((value - self.parse_expression()?).abs() > f32::EPSILON) as u8 as f32)
                }
                Some((b'<', b'=')) => {
                    self.index += 2;
                    Some((value <= self.parse_expression()?) as u8 as f32)
                }
                Some((b'>', b'=')) => {
                    self.index += 2;
                    Some((value >= self.parse_expression()?) as u8 as f32)
                }
                _ if self.consume(b'<') => Some((value < self.parse_expression()?) as u8 as f32),
                _ if self.consume(b'>') => Some((value > self.parse_expression()?) as u8 as f32),
                _ => None,
            };
            let Some(result) = result else {
                return Some(value);
            };
            value = result;
        }
    }

    fn parse_expression(&mut self) -> Option<f32> {
        let mut value = self.parse_term()?;
        loop {
            if self.consume(b'+') {
                value += self.parse_term()?;
            } else if self.consume(b'-') {
                value -= self.parse_term()?;
            } else {
                return Some(value);
            }
        }
    }

    fn parse_term(&mut self) -> Option<f32> {
        let mut value = self.parse_factor()?;
        loop {
            if self.consume(b'*') {
                value *= self.parse_factor()?;
            } else if self.consume(b'/') {
                let divisor = self.parse_factor()?;
                if divisor.abs() <= f32::EPSILON {
                    if self.validate_only {
                        value = 0.0;
                        continue;
                    }
                    return None;
                }
                value /= divisor;
            } else {
                return Some(value);
            }
        }
    }

    fn parse_factor(&mut self) -> Option<f32> {
        if self.consume(b'+') {
            return self.parse_factor();
        }
        if self.consume(b'-') {
            return self.parse_factor().map(|value| -value);
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Option<f32> {
        self.skip_whitespace();
        if self.consume(b'(') {
            let value = self.parse_or()?;
            return self.consume(b')').then_some(value);
        }
        let ch = *self.source.get(self.index)?;
        if ch.is_ascii_digit() || ch == b'.' {
            return self.parse_number();
        }
        if ch.is_ascii_alphabetic() || ch == b'_' {
            let identifier = self.parse_identifier()?;
            if self.consume(b'(') {
                let value = self.parse_call(&identifier)?;
                return self.consume(b')').then_some(value);
            }
            return Some((self.resolve)(&identifier));
        }
        None
    }

    fn parse_identifier(&mut self) -> Option<String> {
        self.skip_whitespace();
        let start = self.index;
        while self.index < self.source.len()
            && (self.source[self.index].is_ascii_alphanumeric()
                || matches!(self.source[self.index], b'_' | b'.'))
        {
            self.index += 1;
        }
        (self.index > start)
            .then(|| std::str::from_utf8(&self.source[start..self.index]).ok())
            .flatten()
            .map(ToString::to_string)
    }

    fn parse_number(&mut self) -> Option<f32> {
        self.skip_whitespace();
        let start = self.index;
        let mut seen_dot = false;
        while self.index < self.source.len() {
            let ch = self.source[self.index];
            if ch.is_ascii_digit() {
                self.index += 1;
            } else if ch == b'.' && !seen_dot {
                seen_dot = true;
                self.index += 1;
            } else {
                break;
            }
        }
        std::str::from_utf8(&self.source[start..self.index])
            .ok()?
            .parse()
            .ok()
    }

    fn parse_arguments(&mut self) -> Option<Vec<f32>> {
        let mut arguments = Vec::new();
        self.skip_whitespace();
        if self.source.get(self.index) == Some(&b')') {
            return Some(arguments);
        }
        loop {
            arguments.push(self.parse_expression()?);
            if !self.consume(b',') {
                return Some(arguments);
            }
        }
    }

    fn parse_call(&mut self, identifier: &str) -> Option<f32> {
        let arguments = self.parse_arguments()?;
        match identifier {
            "min" if arguments.len() == 2 => Some(arguments[0].min(arguments[1])),
            "max" if arguments.len() == 2 => Some(arguments[0].max(arguments[1])),
            "clamp" if arguments.len() == 3 => {
                if arguments[1] <= arguments[2] {
                    Some(arguments[0].clamp(arguments[1], arguments[2]))
                } else if self.validate_only {
                    Some(arguments[0])
                } else {
                    None
                }
            }
            "abs" if arguments.len() == 1 => Some(arguments[0].abs()),
            "floor" if arguments.len() == 1 => Some(arguments[0].floor()),
            "ceil" if arguments.len() == 1 => Some(arguments[0].ceil()),
            "round" if arguments.len() == 1 => Some(arguments[0].round()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_formula_and_extracts_dependencies() {
        let value = evaluate_formula(
            "clamp(base + floor((VIT - 10) / 2), 1, 99)",
            |name| match name {
                "base" => 10.0,
                "VIT" => 14.0,
                _ => 0.0,
            },
        );
        assert_eq!(value, Some(12.0));
        assert_eq!(
            formula_identifiers("base + max(POWER, WIS)"),
            BTreeSet::from(["POWER".into(), "WIS".into(), "base".into()])
        );
        assert!(formula_is_valid("1 / (CURRENT - LIMIT)"));
        assert!(!formula_is_valid("missing(1)"));
        assert_eq!(evaluate_formula("1 / (CURRENT - LIMIT)", |_| 1.0), None);
    }
}
