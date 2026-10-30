pub const PAGE_SIZE: usize = 12;

pub mod catalog;
pub mod favorites;
pub mod motion;
pub mod wheel;

/// Windows Unicode injection uses UTF-16 code units, including surrogate pairs
/// and every code unit of a joined emoji sequence.
pub fn unicode_units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injection_preserves_joined_and_modified_sequences() {
        for text in ["😂", "👍🏽", "👩‍💻", "🇮🇳", "❤️", "😂😂😂"] {
            assert_eq!(String::from_utf16(&unicode_units(text)).unwrap(), text);
        }
    }
}
