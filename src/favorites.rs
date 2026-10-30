//! Small local favorites store. Preserve complete Unicode sequences verbatim.
use std::{fs, io, path::Path};

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Favorites {
    pub items: Vec<String>,
}
impl Favorites {
    pub fn contains(&self, text: &str) -> bool {
        self.items.iter().any(|s| s == text)
    }
    pub fn add(&mut self, text: &str) -> Result<bool, &'static str> {
        let text = text.trim();
        if text.is_empty() || text.is_ascii() || text.chars().any(char::is_control) {
            return Err("Paste an emoji or an emoji combination");
        }
        if text.encode_utf16().count() > 128 {
            return Err("Keep each favorite under 128 UTF16 units");
        }
        if self.contains(text) {
            return Ok(false);
        }
        if self.items.len() >= 128 {
            return Err("Favorites is full (128 entries)");
        }
        self.items.push(text.to_owned());
        Ok(true)
    }
    pub fn remove(&mut self, text: &str) {
        self.items.retain(|s| s != text);
    }
    pub fn load(path: &Path) -> io::Result<Self> {
        let mut store = Self::default();
        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(store),
            Err(e) => return Err(e),
        };
        if metadata.len() > 65536 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Favorites file too large",
            ));
        }
        for text in fs::read_to_string(path)?.lines() {
            let _ = store.add(text);
        }
        Ok(store)
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        let contents = self.items.join("\n");
        fs::write(&temporary, contents)?;
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::remove_file(temporary);
            return Err(error);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn favorites_persist_arbitrary_sequences_deduplicate_and_remove() {
        let path = std::env::temp_dir().join(format!(
            "emorbit-favorites-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut store = Favorites::load(&path).unwrap();
        for text in ["🫠", "🧑🏽‍🚀", "😭💔🥀", "🇯🇵"] {
            assert!(store.add(text).unwrap());
        }
        assert!(!store.add("🫠").unwrap());
        assert!(store.add("plain text").is_err());
        assert!(store.add("😭\n💔").is_err());
        store.save(&path).unwrap();
        assert_eq!(Favorites::load(&path).unwrap(), store);
        store.remove("🫠");
        store.save(&path).unwrap();
        assert!(!Favorites::load(&path).unwrap().contains("🫠"));
        fs::remove_file(path).unwrap();
    }
}
