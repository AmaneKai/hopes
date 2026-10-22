//! A small TOML reader covering what `config.toml` needs, so the full `toml` crate stays out of
//! the binary. Supports tables (`[a.b]`), dotted keys, basic / literal / multi-line strings,
//! integers (decimal, hex, octal, binary, `_` separators), booleans, arrays and inline tables.
//! Floats and dates parse as opaque scalars. Duplicate keys and malformed input are errors.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Bool(bool),
    /// A float or date-time; accepted so the file parses, but never read.
    Other,
    Array(Vec<Value>),
    Table(Table),
}

pub type Table = BTreeMap<String, Value>;

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError;

type Res<T> = Result<T, ParseError>;

pub fn parse(input: &str) -> Res<Table> {
    let mut p = Parser {
        chars: input
            .strip_prefix('\u{feff}')
            .unwrap_or(input)
            .chars()
            .collect(),
        pos: 0,
    };
    p.document()
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn eat(&mut self, c: char) -> bool {
        let hit = self.peek() == Some(c);
        if hit {
            self.pos += 1;
        }
        hit
    }

    fn starts_with(&self, s: &str) -> bool {
        s.chars()
            .enumerate()
            .all(|(i, c)| self.peek_at(i) == Some(c))
    }

    fn skip_blank(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.pos += 1;
        }
    }

    fn skip_comment(&mut self) -> Res<()> {
        if self.peek() == Some('#') {
            while let Some(c) = self.peek() {
                if c == '\n' || (c == '\r' && self.peek_at(1) == Some('\n')) {
                    break;
                }
                if is_control(c) {
                    return Err(ParseError);
                }
                self.pos += 1;
            }
        }
        Ok(())
    }

    /// Whitespace, comments and newlines, for places a value may span lines.
    fn skip_space_and_newlines(&mut self) -> Res<()> {
        loop {
            self.skip_blank();
            self.skip_comment()?;
            match self.peek() {
                Some('\n') => self.pos += 1,
                Some('\r') if self.peek_at(1) == Some('\n') => self.pos += 2,
                _ => return Ok(()),
            }
        }
    }

    fn end_of_line(&mut self) -> Res<()> {
        self.skip_blank();
        self.skip_comment()?;
        match self.peek() {
            None => Ok(()),
            Some('\n') => {
                self.pos += 1;
                Ok(())
            }
            Some('\r') if self.peek_at(1) == Some('\n') => {
                self.pos += 2;
                Ok(())
            }
            _ => Err(ParseError),
        }
    }

    fn document(&mut self) -> Res<Table> {
        let mut root = Table::new();
        let mut current: Vec<String> = Vec::new();
        // Tables created implicitly by a `[a.b]` header may still be defined explicitly later.
        let mut defined: Vec<Vec<String>> = Vec::new();
        // Inline tables and literal arrays are sealed: nothing may be added to them later.
        let mut sealed: Vec<Vec<String>> = Vec::new();
        loop {
            self.skip_space_and_newlines()?;
            match self.peek() {
                None => return Ok(root),
                Some('[') => {
                    self.pos += 1;
                    let array = self.eat('[');
                    self.skip_blank();
                    let path = self.key_path()?;
                    self.skip_blank();
                    if !self.eat(']') || (array && !self.eat(']')) {
                        return Err(ParseError);
                    }
                    self.end_of_line()?;
                    if sealed.iter().any(|p| path.starts_with(p)) {
                        return Err(ParseError);
                    }
                    if array {
                        // A new element starts afresh, so its sub-tables may be defined again.
                        defined.retain(|d| !d.starts_with(&path));
                        push_element(&mut root, &path)?;
                    } else {
                        if defined.contains(&path) {
                            return Err(ParseError);
                        }
                        define_table(&mut root, &path)?;
                        defined.push(path.clone());
                    }
                    current = path;
                }
                Some(_) => {
                    let path = self.key_path()?;
                    self.skip_blank();
                    if !self.eat('=') {
                        return Err(ParseError);
                    }
                    self.skip_blank();
                    let value = self.value()?;
                    self.end_of_line()?;
                    let full: Vec<String> = current.iter().chain(&path).cloned().collect();
                    if sealed
                        .iter()
                        .any(|p| full.starts_with(p) && full.len() > p.len())
                    {
                        return Err(ParseError);
                    }
                    if matches!(value, Value::Table(_) | Value::Array(_)) {
                        sealed.push(full);
                    }
                    insert(table_at(&mut root, &current)?, &path, value)?;
                }
            }
        }
    }

    fn key_path(&mut self) -> Res<Vec<String>> {
        let mut path = vec![self.key()?];
        loop {
            self.skip_blank();
            if !self.eat('.') {
                return Ok(path);
            }
            self.skip_blank();
            path.push(self.key()?);
        }
    }

    fn key(&mut self) -> Res<String> {
        match self.peek() {
            Some('"') => self.basic_string(),
            Some('\'') => self.literal_string(),
            Some(c) if is_bare(c) => {
                let start = self.pos;
                while self.peek().is_some_and(is_bare) {
                    self.pos += 1;
                }
                Ok(self.chars[start..self.pos].iter().collect())
            }
            _ => Err(ParseError),
        }
    }

    fn value(&mut self) -> Res<Value> {
        match self.peek().ok_or(ParseError)? {
            '"' if self.starts_with("\"\"\"") => self.multi_basic().map(Value::Str),
            '"' => self.basic_string().map(Value::Str),
            '\'' if self.starts_with("'''") => self.multi_literal().map(Value::Str),
            '\'' => self.literal_string().map(Value::Str),
            '[' => self.array(),
            '{' => self.inline_table(),
            _ => self.scalar(),
        }
    }

    fn scalar(&mut self) -> Res<Value> {
        let start = self.pos;
        // Date-times may contain one space between the date and the time.
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '_' | '.' | ':') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let word: String = self.chars[start..self.pos].iter().collect();
        match word.as_str() {
            "true" => return Ok(Value::Bool(true)),
            "false" => return Ok(Value::Bool(false)),
            "inf" | "+inf" | "-inf" | "nan" | "+nan" | "-nan" => return Ok(Value::Other),
            _ => {}
        }
        if let Some(n) = parse_int(&word) {
            return Ok(Value::Int(n));
        }
        if is_float(&word) {
            return Ok(Value::Other);
        }
        // A date and time may be separated by a single space instead of `T`.
        let mut word = word;
        if is_date(&word)
            && self.peek() == Some(' ')
            && self.peek_at(1).is_some_and(|c| c.is_ascii_digit())
        {
            let save = self.pos;
            self.pos += 1;
            let time_start = self.pos;
            while self
                .peek()
                .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.' | ':'))
            {
                self.pos += 1;
            }
            let time: String = self.chars[time_start..self.pos].iter().collect();
            if valid_datetime(&format!("{word}T{time}")) {
                return Ok(Value::Other);
            }
            self.pos = save;
            word = word.clone();
        }
        if valid_datetime(&word) {
            return Ok(Value::Other);
        }
        Err(ParseError)
    }

    fn array(&mut self) -> Res<Value> {
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            self.skip_space_and_newlines()?;
            if self.eat(']') {
                return Ok(Value::Array(items));
            }
            items.push(self.value()?);
            self.skip_space_and_newlines()?;
            if self.eat(',') {
                continue;
            }
            if self.eat(']') {
                return Ok(Value::Array(items));
            }
            return Err(ParseError);
        }
    }

    fn inline_table(&mut self) -> Res<Value> {
        self.pos += 1;
        let mut table = Table::new();
        self.skip_blank();
        if self.eat('}') {
            return Ok(Value::Table(table));
        }
        loop {
            self.skip_blank();
            let path = self.key_path()?;
            self.skip_blank();
            if !self.eat('=') {
                return Err(ParseError);
            }
            self.skip_blank();
            let value = self.value()?;
            insert(&mut table, &path, value)?;
            self.skip_blank();
            if self.eat(',') {
                continue;
            }
            if self.eat('}') {
                return Ok(Value::Table(table));
            }
            return Err(ParseError);
        }
    }

    fn basic_string(&mut self) -> Res<String> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            match self.bump().ok_or(ParseError)? {
                '"' => return Ok(out),
                '\\' => out.push(self.escape()?),
                c if c == '\n' || is_control(c) => return Err(ParseError),
                c => out.push(c),
            }
        }
    }

    fn literal_string(&mut self) -> Res<String> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            match self.bump().ok_or(ParseError)? {
                '\'' => return Ok(out),
                c if c == '\n' || is_control(c) => return Err(ParseError),
                c => out.push(c),
            }
        }
    }

    fn multi_basic(&mut self) -> Res<String> {
        self.pos += 3;
        self.skip_first_newline();
        let mut out = String::new();
        loop {
            if self.starts_with("\"\"\"") {
                self.close_multiline('"', &mut out)?;
                return Ok(out);
            }
            match self.bump().ok_or(ParseError)? {
                '\\' => {
                    if matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
                        // Line-ending backslash trims the newline and following whitespace.
                        let save = self.pos;
                        self.skip_blank();
                        if matches!(self.peek(), Some('\n' | '\r')) {
                            while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
                                self.pos += 1;
                            }
                        } else {
                            self.pos = save;
                            return Err(ParseError);
                        }
                    } else {
                        out.push(self.escape()?);
                    }
                }
                '\r' if self.peek() == Some('\n') => {}
                c if c != '\n' && c != '\r' && c != '\t' && is_control(c) => {
                    return Err(ParseError);
                }
                c => out.push(c),
            }
        }
    }

    fn multi_literal(&mut self) -> Res<String> {
        self.pos += 3;
        self.skip_first_newline();
        let mut out = String::new();
        loop {
            if self.starts_with("'''") {
                self.close_multiline('\'', &mut out)?;
                return Ok(out);
            }
            match self.bump().ok_or(ParseError)? {
                '\r' if self.peek() == Some('\n') => {}
                c if c != '\n' && c != '\t' && is_control(c) => return Err(ParseError),
                c => out.push(c),
            }
        }
    }

    /// Consumes a closing `"""` / `'''`; one or two quotes right before it belong to the content.
    fn close_multiline(&mut self, quote: char, out: &mut String) -> Res<()> {
        self.pos += 3;
        let mut extra = 0;
        while self.peek() == Some(quote) {
            extra += 1;
            if extra > 2 {
                return Err(ParseError);
            }
            self.pos += 1;
            out.push(quote);
        }
        Ok(())
    }

    fn skip_first_newline(&mut self) {
        if self.peek() == Some('\n') {
            self.pos += 1;
        } else if self.peek() == Some('\r') && self.peek_at(1) == Some('\n') {
            self.pos += 2;
        }
    }

    fn escape(&mut self) -> Res<char> {
        Ok(match self.bump().ok_or(ParseError)? {
            'b' => '\u{8}',
            't' => '\t',
            'n' => '\n',
            'f' => '\u{c}',
            'r' => '\r',
            '"' => '"',
            '\\' => '\\',
            'u' => self.unicode(4)?,
            'U' => self.unicode(8)?,
            _ => return Err(ParseError),
        })
    }

    fn unicode(&mut self, digits: usize) -> Res<char> {
        let mut code = 0u32;
        for _ in 0..digits {
            let d = self.bump().and_then(|c| c.to_digit(16)).ok_or(ParseError)?;
            code = code.checked_mul(16).ok_or(ParseError)? + d;
        }
        char::from_u32(code).ok_or(ParseError)
    }
}

fn is_control(c: char) -> bool {
    (c.is_control() && c != '\t') || c == '\u{7f}'
}

fn is_bare(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn parse_int(word: &str) -> Option<i64> {
    let (neg, body) = match word.as_bytes().first()? {
        b'+' => (false, &word[1..]),
        b'-' => (true, &word[1..]),
        _ => (false, word),
    };
    let (radix, digits) = match body.get(..2) {
        Some("0x") if !neg && word.len() == body.len() => (16, &body[2..]),
        Some("0o") if !neg && word.len() == body.len() => (8, &body[2..]),
        Some("0b") if !neg && word.len() == body.len() => (2, &body[2..]),
        _ => (10, body),
    };
    if digits.is_empty()
        || digits.starts_with('_')
        || digits.ends_with('_')
        || digits.contains("__")
    {
        return None;
    }
    if radix == 10 && digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    let clean: String = digits.chars().filter(|&c| c != '_').collect();
    if clean.is_empty() || !clean.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    let n = i64::from_str_radix(&clean, radix).ok()?;
    if neg { n.checked_neg() } else { Some(n) }
}

/// Digits with single underscores between them; returns the rest of the input.
fn scan_digits(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    if !bytes.first()?.is_ascii_digit() {
        return None;
    }
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'0'..=b'9' => i += 1,
            b'_' if bytes.get(i + 1).is_some_and(u8::is_ascii_digit) => i += 2,
            _ => break,
        }
    }
    Some(&s[i..])
}

/// `[+-] int [. digits] [e [+-] digits]` with at least a fraction or an exponent, finite.
fn is_float(word: &str) -> bool {
    let body = word.strip_prefix(['+', '-']).unwrap_or(word);
    let int_len = body
        .find(|c: char| !c.is_ascii_digit() && c != '_')
        .unwrap_or(body.len());
    let int_part = &body[..int_len];
    if int_part.len() > 1 && int_part.starts_with('0') {
        return false;
    }
    let Some(mut rest) = scan_digits(body) else {
        return false;
    };
    let mut has_part = false;
    if let Some(after) = rest.strip_prefix('.') {
        let Some(r) = scan_digits(after) else {
            return false;
        };
        rest = r;
        has_part = true;
    }
    if let Some(after) = rest.strip_prefix(['e', 'E']) {
        let after = after.strip_prefix(['+', '-']).unwrap_or(after);
        let Some(r) = scan_digits(after) else {
            return false;
        };
        rest = r;
        has_part = true;
    }
    let clean: String = word.chars().filter(|&c| c != '_').collect();
    has_part && rest.is_empty() && clean.parse::<f64>().is_ok_and(f64::is_finite)
}

fn digits(s: &str, len: usize) -> Option<u32> {
    (s.len() == len && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok())?
}

fn is_date(word: &str) -> bool {
    let mut parts = word.split('-');
    let (Some(y), Some(m), Some(d), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let (Some(y), Some(m), Some(d)) = (digits(y, 4), digits(m, 2), digits(d, 2)) else {
        return false;
    };
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let max = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max).contains(&d)
}

/// `hh:mm:ss` with optional `.fraction`.
fn is_time(word: &str) -> bool {
    let (clock, frac) = match word.split_once('.') {
        Some((c, f)) => (c, Some(f)),
        None => (word, None),
    };
    if frac.is_some_and(|f| f.is_empty() || !f.bytes().all(|b| b.is_ascii_digit())) {
        return false;
    }
    let mut parts = clock.split(':');
    let (Some(h), Some(m), Some(s), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    matches!(
        (digits(h, 2), digits(m, 2), digits(s, 2)),
        (Some(h), Some(m), Some(s)) if h < 24 && m < 60 && s <= 60
    )
}

/// Local or offset date, time, or date-time (`T` / `t` separated).
fn valid_datetime(word: &str) -> bool {
    if is_date(word) || is_time(word) {
        return true;
    }
    let Some(split) = word.find(['T', 't']) else {
        return false;
    };
    let (date, rest) = (&word[..split], &word[split + 1..]);
    if !is_date(date) {
        return false;
    }
    let time = match rest.strip_suffix(['Z', 'z']) {
        Some(time) => time,
        None => match rest.rfind(['+', '-']) {
            Some(i) => {
                let (time, offset) = (&rest[..i], &rest[i + 1..]);
                let ok = matches!(
                    offset.split_once(':'),
                    Some((h, m)) if matches!((digits(h, 2), digits(m, 2)), (Some(h), Some(m)) if h < 24 && m < 60)
                );
                if !ok {
                    return false;
                }
                time
            }
            None => rest,
        },
    };
    is_time(time)
}

/// Walks (creating as needed) to the table at `path`; an array of tables resolves to its
/// last element.
fn table_at<'a>(root: &'a mut Table, path: &[String]) -> Res<&'a mut Table> {
    let mut table = root;
    for key in path {
        match table
            .entry(key.clone())
            .or_insert_with(|| Value::Table(Table::new()))
        {
            Value::Table(t) => table = t,
            Value::Array(items) => match items.last_mut() {
                Some(Value::Table(t)) => table = t,
                _ => return Err(ParseError),
            },
            _ => return Err(ParseError),
        }
    }
    Ok(table)
}

/// Like [`table_at`] for a `[header]`, which may not name an array of tables.
fn define_table(root: &mut Table, path: &[String]) -> Res<()> {
    let (last, parents) = path.split_last().ok_or(ParseError)?;
    match table_at(root, parents)?
        .entry(last.clone())
        .or_insert_with(|| Value::Table(Table::new()))
    {
        Value::Table(_) => Ok(()),
        _ => Err(ParseError),
    }
}

/// Appends a fresh table to the array of tables at `path`, creating the array if needed.
fn push_element(root: &mut Table, path: &[String]) -> Res<()> {
    let (last, parents) = path.split_last().ok_or(ParseError)?;
    let parent = table_at(root, parents)?;
    match parent
        .entry(last.clone())
        .or_insert_with(|| Value::Array(Vec::new()))
    {
        Value::Array(items) if items.iter().all(|v| matches!(v, Value::Table(_))) => {
            items.push(Value::Table(Table::new()));
            Ok(())
        }
        _ => Err(ParseError),
    }
}

/// Inserts `value` at a dotted `path`, rejecting duplicates.
fn insert(table: &mut Table, path: &[String], value: Value) -> Res<()> {
    let (last, parents) = path.split_last().ok_or(ParseError)?;
    let table = table_at(table, parents)?;
    if table.contains_key(last) {
        return Err(ParseError);
    }
    table.insert(last.clone(), value);
    Ok(())
}
