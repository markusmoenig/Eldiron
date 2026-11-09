//! Small, deterministic scalar expressions for recipe controls. No host scripting.
use super::{Result, fail, group::Control};
use std::borrow::Cow;

#[derive(Debug)]
enum Expr {
    Number(f64),
    Reference(String),
    Neg(Box<Expr>),
    Binary(u8, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}
struct Parser<'a> {
    text: &'a [u8],
    at: usize,
    nodes: usize,
}
impl Parser<'_> {
    fn space(&mut self) {
        while self.text.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }
    fn take(&mut self, b: u8) -> bool {
        self.space();
        if self.text.get(self.at) == Some(&b) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn expression(&mut self, precedence: u8, depth: usize) -> Result<Expr> {
        self.nodes += 1;
        if self.nodes > 256 || depth > 32 {
            return Err(fail("Control expression is too complex"));
        }
        self.space();
        let mut left = if self.take(b'-') {
            Expr::Neg(Box::new(self.expression(3, depth + 1)?))
        } else if self.take(b'+') {
            self.expression(3, depth + 1)?
        } else if self.take(b'(') {
            let value = self.expression(0, depth + 1)?;
            if !self.take(b')') {
                return Err(fail("Expected ')' in control expression"));
            }
            value
        } else if self.take(b'[') {
            let start = self.at;
            while self.text.get(self.at).is_some_and(|b| *b != b']') {
                self.at += 1;
            }
            let id = std::str::from_utf8(&self.text[start..self.at])
                .map_err(|_| fail("Invalid control reference"))?
                .trim()
                .to_string();
            if id.is_empty() || !self.take(b']') {
                return Err(fail("Expected [control-id]"));
            }
            Expr::Reference(id)
        } else if self
            .text
            .get(self.at)
            .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
        {
            let start = self.at;
            while self
                .text
                .get(self.at)
                .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
            {
                self.at += 1;
            }
            if self
                .text
                .get(self.at)
                .is_some_and(|b| *b == b'e' || *b == b'E')
            {
                self.at += 1;
                if self
                    .text
                    .get(self.at)
                    .is_some_and(|b| *b == b'+' || *b == b'-')
                {
                    self.at += 1;
                }
                while self.text.get(self.at).is_some_and(u8::is_ascii_digit) {
                    self.at += 1;
                }
            }
            let number = std::str::from_utf8(&self.text[start..self.at])
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|v| v.is_finite())
                .ok_or_else(|| fail("Invalid number in control expression"))?;
            Expr::Number(number)
        } else {
            let start = self.at;
            while self
                .text
                .get(self.at)
                .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
            {
                self.at += 1;
            }
            if start == self.at {
                return Err(fail("Expected number, control reference or function"));
            }
            let name = std::str::from_utf8(&self.text[start..self.at])
                .unwrap()
                .to_string();
            if self.take(b'(') {
                let mut args = vec![];
                if !self.take(b')') {
                    loop {
                        args.push(self.expression(0, depth + 1)?);
                        if self.take(b')') {
                            break;
                        }
                        if !self.take(b',') {
                            return Err(fail("Expected ',' or ')' in control expression"));
                        }
                    }
                }
                Expr::Call(name, args)
            } else {
                Expr::Reference(name)
            }
        };
        loop {
            self.space();
            let op = self.text.get(self.at).copied().unwrap_or(0);
            let p = match op {
                b'+' | b'-' => 1,
                b'*' | b'/' => 2,
                _ => 0,
            };
            if p == 0 || p <= precedence {
                break;
            }
            self.at += 1;
            self.nodes += 1;
            if self.nodes > 256 {
                return Err(fail("Control expression is too complex"));
            }
            let right = self.expression(p, depth + 1)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }
}
impl Expr {
    fn parse(text: &str) -> Result<Self> {
        if text.len() > 1024 {
            return Err(fail("Control expression exceeds 1024 bytes"));
        }
        let mut parser = Parser {
            text: text.as_bytes(),
            at: 0,
            nodes: 0,
        };
        let result = parser.expression(0, 0)?;
        parser.space();
        if parser.at != text.len() {
            return Err(fail("Unexpected text in control expression"));
        }
        Ok(result)
    }
    fn evaluate(
        &self,
        depth: usize,
        reference: &mut impl FnMut(&str, usize) -> Result<f64>,
    ) -> Result<f64> {
        if depth > 128 {
            return Err(fail("Control expression dependencies are too deep"));
        }
        let value = match self {
            Self::Number(v) => *v,
            Self::Reference(id) => reference(id, depth + 1)?,
            Self::Neg(e) => -e.evaluate(depth + 1, reference)?,
            Self::Binary(op, a, b) => {
                let (a, b) = (
                    a.evaluate(depth + 1, reference)?,
                    b.evaluate(depth + 1, reference)?,
                );
                match op {
                    b'+' => a + b,
                    b'-' => a - b,
                    b'*' => a * b,
                    b'/' if b != 0. => a / b,
                    _ => return Err(fail("Division by zero in control expression")),
                }
            }
            Self::Call(name, args) => {
                let v = args
                    .iter()
                    .map(|e| e.evaluate(depth + 1, reference))
                    .collect::<Result<Vec<_>>>()?;
                match (name.as_str(), v.as_slice()) {
                    ("sin_deg", [x]) => x.to_radians().sin(),
                    ("cos_deg", [x]) => x.to_radians().cos(),
                    ("tan_deg", [x]) if x.to_radians().cos().abs() > 1e-10 => x.to_radians().tan(),
                    ("abs", [x]) => x.abs(),
                    ("sqrt", [x]) => x.sqrt(),
                    ("floor", [x]) => x.floor(),
                    ("ceil", [x]) => x.ceil(),
                    ("round", [x]) => x.round(),
                    ("min", [a, b]) => a.min(*b),
                    ("max", [a, b]) => a.max(*b),
                    ("clamp", [x, lo, hi]) if lo <= hi => x.clamp(*lo, *hi),
                    _ => {
                        return Err(fail(format!(
                            "Unknown function, invalid arguments or singularity: {name}"
                        )));
                    }
                }
            }
        };
        if !value.is_finite() || value.abs() > 1e12 {
            return Err(fail(
                "Control expression produced an invalid or excessive number",
            ));
        }
        Ok(value)
    }
}
/// Resolve dependencies without changing authored controls. IDs are local to this group.
pub fn resolve(controls: &[Control]) -> Result<Cow<'_, [Control]>> {
    super::group::validate(controls)?;
    if controls.iter().all(|c| c.expression.is_none()) {
        return Ok(Cow::Borrowed(controls));
    }
    let expressions = controls
        .iter()
        .map(|c| c.expression.as_deref().map(Expr::parse).transpose())
        .collect::<Result<Vec<_>>>()?;
    let mut states = vec![0; controls.len()];
    let mut values = vec![0.; controls.len()];
    fn visit(
        i: usize,
        depth: usize,
        controls: &[Control],
        expressions: &[Option<Expr>],
        states: &mut [u8],
        values: &mut [f64],
    ) -> Result<f64> {
        if states[i] == 2 {
            return Ok(values[i]);
        }
        if states[i] == 1 {
            return Err(fail(format!(
                "Cyclic control expression: {}",
                controls[i].label
            )));
        }
        states[i] = 1;
        let c = &controls[i];
        let v = if let Some(expr) = &expressions[i] {
            expr.evaluate(depth, &mut |id, depth| {
                let j = controls
                    .iter()
                    .position(|c| c.id == id)
                    .ok_or_else(|| fail(format!("Unknown control reference: {id}")))?;
                visit(j, depth, controls, expressions, states, values)
            })
            .map_err(|e| fail(format!("{}: {e}", c.label)))?
        } else {
            c.value as f64
        };
        if v < c.min as f64 - 1e-6
            || v > c.max as f64 + 1e-6
            || (c.is_integer() || c.is_palette()) && (v - v.round()).abs() > 1e-6
        {
            return Err(fail(format!(
                "{}: calculated value {v} must be in {}–{}{}",
                c.label,
                c.min,
                c.max,
                if c.is_integer() || c.is_palette() {
                    " and an integer"
                } else {
                    ""
                }
            )));
        }
        let v = v.clamp(c.min as f64, c.max as f64);
        states[i] = 2;
        values[i] = v;
        Ok(v)
    }
    for i in 0..controls.len() {
        visit(i, 0, controls, &expressions, &mut states, &mut values)?;
    }
    let mut resolved = controls.to_vec();
    for (c, v) in resolved.iter_mut().zip(values) {
        c.value = v as f32;
    }
    Ok(Cow::Owned(resolved))
}
