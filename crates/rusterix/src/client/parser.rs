use regex::Regex;
use std::collections::HashMap;

/// Tokens produced by the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    TextKey {
        key: String,
        opts: HashMap<String, String>,
    }, // {the,case=upper}
    Entity {
        id: u32,
        attr: String,
        opts: HashMap<String, String>,
    }, // {E:20.name,article=def}
    Item {
        id: u32,
        attr: String,
        opts: HashMap<String, String>,
    }, // {It:102.name,article=indef}
    Num {
        val: i64,
        opts: HashMap<String, String>,
    }, // {N:50,unit=hp}
    Float {
        val: f64,
        opts: HashMap<String, String>,
    }, // {F:3.14,precision=2}
    Plain(String), // literal outside {…}
}

/// Parser struct (keeps compiled regex for speed).
pub struct MsgParser {
    brace_re: Regex,
}

impl MsgParser {
    /// Create a new parser. Compile the `{...}` matcher once.
    pub fn new() -> Self {
        let brace_re = Regex::new(r"\{([^{}]+)\}").unwrap();
        Self { brace_re }
    }

    /// Parse an input message into tokens.
    pub fn parse(&self, input: &str) -> Vec<Tok> {
        let mut toks = Vec::new();
        let mut last = 0usize;

        for caps in self.brace_re.captures_iter(input) {
            let full = caps.get(0).unwrap();
            let inner = caps.get(1).unwrap().as_str();

            // preceding plain text
            if full.start() > last {
                toks.push(Tok::Plain(input[last..full.start()].to_string()));
            }

            toks.push(self.parse_inner(inner));
            last = full.end();
        }

        // trailing plain text
        if last < input.len() {
            toks.push(Tok::Plain(input[last..].to_string()));
        }

        toks
    }

    fn parse_inner(&self, s: &str) -> Tok {
        let s = s.trim();

        if let Some(rest) = strip_prefix_ci(s, "E:") {
            return self.parse_ref(s, rest, Kind::Entity);
        }
        if let Some(rest) = strip_prefix_ci(s, "I:") {
            return self.parse_ref(s, rest, Kind::Item);
        }
        if let Some(rest) = strip_prefix_ci(s, "It:") {
            return self.parse_ref(s, rest, Kind::Item);
        }
        if let Some(rest) = strip_prefix_ci(s, "Item:") {
            return self.parse_ref(s, rest, Kind::Item);
        }
        if let Some(rest) = strip_prefix_ci(s, "N:") {
            let (head, opts) = split_head_opts(rest);
            if let Ok(val) = head.parse::<i64>() {
                return Tok::Num { val, opts };
            } else {
                let (head, o) = split_head_opts(s);
                return Tok::TextKey { key: head, opts: o };
            }
        }
        if let Some(rest) = strip_prefix_ci(s, "F:") {
            let (head, opts) = split_head_opts(rest);
            if let Ok(val) = head.parse::<f64>() {
                return Tok::Float { val, opts };
            } else {
                let (head, o) = split_head_opts(s);
                return Tok::TextKey { key: head, opts: o };
            }
        }

        // Unknown/malformed → treat as TextKey
        let (head, o) = split_head_opts(s);
        Tok::TextKey { key: head, opts: o }
    }

    fn parse_ref(&self, full: &str, rest: &str, kind: Kind) -> Tok {
        let (head, opts) = split_head_opts(rest);
        let mut it = head.splitn(2, '.');
        let id_str = it.next().unwrap_or_default().trim();
        let attr = it.next().unwrap_or("name").to_string();

        let id_opt = id_str.parse::<u32>().ok().or_else(|| {
            id_str.parse::<i64>().ok().and_then(|v| {
                if v >= 0 && v <= u32::MAX as i64 {
                    Some(v as u32)
                } else {
                    None
                }
            })
        });

        if let Some(id) = id_opt {
            match kind {
                Kind::Entity => Tok::Entity { id, attr, opts },
                Kind::Item => Tok::Item { id, attr, opts },
            }
        } else {
            Tok::TextKey {
                key: full.to_string(),
                opts: HashMap::new(),
            }
        }
    }
}

enum Kind {
    Entity,
    Item,
}
// impl Kind {
//     pub fn as_tag(&self) -> &'static str {
//         match self {
//             Kind::Entity => "E",
//             Kind::Item => "It",
//         }
//     }
// }

/// Split "head[,k=v,k=v]" into ("head", {k:v,...})
fn split_head_opts(s: &str) -> (String, HashMap<String, String>) {
    if let Some(idx) = s.find(',') {
        let head = s[..idx].trim().to_string();
        let opts_str = &s[idx + 1..];
        (head, parse_opts(opts_str))
    } else {
        (s.trim().to_string(), HashMap::new())
    }
}

/// Parse "k=v,k=v" into a map. Values may be unquoted or "quoted".
fn parse_opts(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for seg in s.split(',').map(str::trim).filter(|x| !x.is_empty()) {
        if let Some((k, v)) = seg.split_once('=') {
            let key = k.trim().to_string();
            let mut val = v.trim().to_string();
            // strip optional quotes
            if (val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\''))
            {
                val = val[1..val.len().saturating_sub(1)].to_string();
            }
            map.insert(key, val);
        }
    }
    map
}

/// Case-insensitive prefix strip. Returns remainder if s starts with prefix (case-insensitive).
fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() < prefix.len() {
        return None;
    }
    let (head, tail) = s.split_at(prefix.len());
    if head.eq_ignore_ascii_case(prefix) {
        Some(tail)
    } else {
        None
    }
}
