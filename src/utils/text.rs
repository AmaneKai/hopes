use std::borrow::Cow;

pub fn normalize(bytes: &[u8]) -> Cow<'_, str> {
    match bytes {
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest),
        [0xFF, 0xFE, rest @ ..] => Cow::Owned(decode_utf16(rest, u16::from_le_bytes)),
        [0xFE, 0xFF, rest @ ..] => Cow::Owned(decode_utf16(rest, u16::from_be_bytes)),
        _ => String::from_utf8_lossy(bytes),
    }
}

fn decode_utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
    let units = bytes.chunks_exact(2).map(|pair| unit([pair[0], pair[1]]));
    char::decode_utf16(units)
        .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}
