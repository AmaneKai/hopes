use hopes::{
    config::{Backend, Config},
    utils::mini_toml::{self, Value},
};

fn convert(v: &toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::Str(s.clone()),
        toml::Value::Integer(n) => Value::Int(*n),
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Float(_) | toml::Value::Datetime(_) => Value::Other,
        toml::Value::Array(a) => Value::Array(a.iter().map(convert).collect()),
        toml::Value::Table(t) => {
            Value::Table(t.iter().map(|(k, v)| (k.clone(), convert(v))).collect())
        }
    }
}

fn oracle(text: &str) -> Option<Value> {
    text.parse::<toml::Table>()
        .ok()
        .map(|t| convert(&toml::Value::Table(t)))
}

fn mine(text: &str) -> Option<Value> {
    mini_toml::parse(text).ok().map(Value::Table)
}

const VALID: &[&str] = &[
    "",
    "# only a comment\n",
    "a = 1",
    "a = 1\n",
    "a = 1\r\nb = 2\r\n",
    "[sync]\nenabled = true\ndebounce_seconds = 5\n",
    "[sync]\ndata_dir = \"/Users/x/hopes-data\"\nremote = 'origin'\nbranch = \"main\"\n",
    "[sync] # trailing\nenabled=false # c\n",
    "[ sync ]\n  enabled   =   true\n",
    "[a.b.c]\nx = 1\n[a]\ny = 2\n",
    "[a . b]\nx = 1\n",
    "a.b.c = 1\na.b.d = 2\n",
    "\"quoted key\" = 1\n'lit key' = 2\n",
    "n = 1_000\nh = 0xdead_beef\no = 0o755\nb = 0b1010\nz = 0\np = +5\nm = -5\n",
    "f = 3.14\ne = 1e10\nf2 = -2.5E-3\ni = inf\nn = nan\npi = +inf\n",
    "d = 1979-05-27\nt = 07:32:00\ndt = 1979-05-27T07:32:00Z\nsp = 1979-05-27 07:32:00\n",
    "s = \"esc \\n \\t \\\" \\\\ \\u00e9 \\U0001F600\"\n",
    "s = 'C:\\no\\escape'\n",
    "s = \"\"\"\nline1\nline2\"\"\"\n",
    "s = \"\"\"a \\\n   b\"\"\"\n",
    "s = '''\nraw \\n text'''\n",
    "s = \"\"\"quote \"\" inside\"\"\"\n",
    "s = \"\"\"ends with quote\\\"\"\"\"\n",
    "s = \"\"\"x\"\"\"\"\"\n",
    "arr = []\narr2 = [1, 2, 3]\narr3 = [\"a\", \"b\",]\n",
    "arr = [\n  \"a\", # first\n  \"b\",\n  # c\n]\n",
    "arr = [[1, 2], [\"a\"]]\n",
    "t = {}\nt2 = { a = 1, b = \"x\" }\nt3 = { a.b = 1 }\n",
    "[storage]\nbackend = \"exec\"\ncommand = [\"sh\", \"-c\", \"cat\"]\n",
    "[storage]\nbackend = \"sqlite\"\ndir = \"/tmp/x\"\n",
    "x = true\ny = false\n",
    "[[arr]]\nx = 1\n[[arr]]\nx = 2\n[arr.sub]\ny = 3\n",
    "[[a.b]]\nx = 1\n[a.b.c]\nz = 1\n[[a.b]]\n[a.b.c]\nz = 2\n",
    "\u{feff}a = 1\n",
    "a = \"é日本語\"\n",
    "# c\n\n\n[a]\n\n# c\nb = 1\n\n",
];

const INVALID: &[&str] = &[
    "a",
    "a =",
    "= 1",
    "a = 1 b = 2",
    "a = 1\na = 2",
    "[a]\n[a]",
    "[a\nb = 1",
    "a = \"unterminated",
    "a = 'unterminated",
    "a = \"bad \\q escape\"",
    "a = \"\\uD800\"",
    "a = 01",
    "a = 1__0",
    "a = _1",
    "a = 1_",
    "a = 0x",
    "a = -0x1",
    "a = 99999999999999999999",
    "a = [1, 2",
    "a = [1 2]",
    "a = {a = 1",
    "a = {a = 1,}",
    "a = tru",
    "a = True",
    "a = 1.",
    "a = .5",
    "a = \"\"\"never closed",
    "a = \"\"\"x\"\"\"\"\"\"",
    "a.b = 1\na = 2",
    "a = 1\n[a]",
    "a = \"tab\tin\"\nb = \"nl\nx\"",
    "a = 1 # \u{7} bell",
    "[sync]]\n",
    "[[arr]\n",
    "a = [1]\n[[a]]\n",
    "[[a]]\n[a]\n",
    "d = 2023-02-30\n",
    "t = 25:00:00\n",
    "[]\n",
    "a b = 1",
    "a = 1,",
];

#[test]
fn agrees_with_toml_on_valid_documents() {
    for doc in VALID {
        let expected = oracle(doc);
        assert!(expected.is_some(), "corpus bug, toml rejects: {doc:?}");
        assert_eq!(mine(doc), expected, "doc: {doc:?}");
    }
}

#[test]
fn rejects_what_toml_rejects() {
    for doc in INVALID {
        assert!(oracle(doc).is_none(), "corpus bug, toml accepts: {doc:?}");
        assert!(mine(doc).is_none(), "should reject: {doc:?}");
    }
}

#[test]
fn agrees_with_toml_on_mutated_documents() {
    let alphabet: Vec<char> = "=[]{}\",'#.\n \\_-+:0aZ9\t".chars().collect();
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut disagreements = Vec::new();
    for doc in VALID {
        let chars: Vec<char> = doc.chars().collect();
        for _ in 0..4000 {
            let mut c = chars.clone();
            let at = if c.is_empty() {
                0
            } else {
                next() as usize % c.len()
            };
            let ch = alphabet[next() as usize % alphabet.len()];
            match next() % 3 {
                0 if !c.is_empty() => {
                    c.remove(at);
                }
                1 if !c.is_empty() => c[at] = ch,
                _ => c.insert(at, ch),
            }
            let text: String = c.into_iter().collect();
            if mine(&text) != oracle(&text) {
                disagreements.push(text);
            }
        }
    }
    disagreements.sort();
    disagreements.dedup();
    assert!(
        disagreements.is_empty(),
        "{} disagreements, first few: {:#?}",
        disagreements.len(),
        &disagreements[..disagreements.len().min(8)]
    );
}

#[test]
fn config_defaults_overrides_and_fallbacks() {
    let d = Config::default();
    let c = Config::from_toml("").unwrap();
    assert_eq!(c.sync.debounce_seconds, d.sync.debounce_seconds);
    assert!(c.sync.enabled);
    assert_eq!(c.storage.backend, Backend::Json);

    let c = Config::from_toml(
        "[sync]\nenabled = false\ndebounce_seconds = 9\ndata_dir = \"/x\"\nremote = \"up\"\nbranch = \"dev\"\npull_on_startup = false\nunknown = 1\n\n[storage]\nbackend = \"exec\"\ncommand = [\"a\", \"b\"]\ndir = \"/d\"\n\n[other]\nx = 1\n",
    )
    .unwrap();
    assert!(!c.sync.enabled);
    assert_eq!(c.sync.debounce_seconds, 9);
    assert_eq!(c.sync.data_dir.as_deref(), Some(std::path::Path::new("/x")));
    assert_eq!(
        (c.sync.remote.as_str(), c.sync.branch.as_str()),
        ("up", "dev")
    );
    assert!(!c.sync.pull_on_startup);
    assert_eq!(c.storage.backend, Backend::Exec);
    assert_eq!(c.storage.command, ["a", "b"]);

    assert!(Config::from_toml("[sync]\ndebounce_seconds = -1\n").is_none());
    assert!(Config::from_toml("[sync]\nenabled = \"yes\"\n").is_none());
    assert!(Config::from_toml("[storage]\nbackend = \"mongo\"\n").is_none());
    assert!(Config::from_toml("[storage]\ncommand = [1]\n").is_none());
    assert!(Config::from_toml("sync = 3\n").is_none());
    assert!(Config::from_toml("not toml at all [").is_none());
}
