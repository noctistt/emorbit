//! Small, manually curated prototype catalog. This is not a full Unicode dataset.
#[derive(Debug)]
pub struct Emoji {
    pub text: &'static str,
    pub name: &'static str,
    pub keywords: &'static str,
    pub variants: &'static [&'static str],
}

pub const CATALOG: &[Emoji] = &[
    Emoji {
        text: "😂",
        name: "tears of joy",
        keywords: "laugh funny happy lol",
        variants: &["😂", "😂😂", "😂😂😂", "🤣", "😆", "😹"],
    },
    Emoji {
        text: "❤️",
        name: "red heart",
        keywords: "love like affection",
        variants: &["❤️", "❤️❤️", "❤️❤️❤️", "🧡", "💛", "💜"],
    },
    Emoji {
        text: "👍",
        name: "thumbs up",
        keywords: "thumb hand yes good approve",
        variants: &["👍", "👍🏻", "👍🏼", "👍🏽", "👍🏾", "👍🏿"],
    },
    Emoji {
        text: "🥲",
        name: "smiling face with tear",
        keywords: "sad happy bittersweet cry",
        variants: &["🥲", "🥲🥲", "🥲🥲🥲", "🥹", "😢", "😭"],
    },
    Emoji {
        text: "🔥",
        name: "fire",
        keywords: "hot lit flame",
        variants: &["🔥", "🔥🔥", "🔥🔥🔥", "✨", "💥", "🌟"],
    },
    Emoji {
        text: "👋",
        name: "waving hand",
        keywords: "hello hi bye hand wave",
        variants: &["👋", "👋🏻", "👋🏼", "👋🏽", "👋🏾", "👋🏿"],
    },
    Emoji {
        text: "👀",
        name: "eyes",
        keywords: "look see watching curious",
        variants: &["👀", "👀👀", "👀👀👀", "🫣", "🤔", "😏"],
    },
    Emoji {
        text: "🎉",
        name: "party popper",
        keywords: "celebrate birthday party congratulations",
        variants: &["🎉", "🎉🎉", "🎉🎉🎉", "🥳", "🎊", "🎂"],
    },
    Emoji {
        text: "😀",
        name: "grinning face",
        keywords: "smile happy grin",
        variants: &[],
    },
    Emoji {
        text: "😊",
        name: "smiling face",
        keywords: "happy blush smile",
        variants: &[],
    },
    Emoji {
        text: "😍",
        name: "heart eyes",
        keywords: "love happy smile",
        variants: &[],
    },
    Emoji {
        text: "😭",
        name: "loudly crying face",
        keywords: "sad cry tears",
        variants: &[],
    },
    Emoji {
        text: "🤣",
        name: "rolling on the floor laughing",
        keywords: "laugh funny lol rofl",
        variants: &[],
    },
    Emoji {
        text: "🤔",
        name: "thinking face",
        keywords: "think question confused",
        variants: &[],
    },
    Emoji {
        text: "🙏",
        name: "folded hands",
        keywords: "please thank prayer hand",
        variants: &["🙏", "🙏🏻", "🙏🏼", "🙏🏽", "🙏🏾", "🙏🏿"],
    },
    Emoji {
        text: "👎",
        name: "thumbs down",
        keywords: "thumb hand no bad disapprove",
        variants: &["👎", "👎🏻", "👎🏼", "👎🏽", "👎🏾", "👎🏿"],
    },
    Emoji {
        text: "👌",
        name: "OK hand",
        keywords: "hand good okay",
        variants: &["👌", "👌🏻", "👌🏼", "👌🏽", "👌🏾", "👌🏿"],
    },
    Emoji {
        text: "💪",
        name: "flexed biceps",
        keywords: "strong muscle gym arm",
        variants: &["💪", "💪🏻", "💪🏼", "💪🏽", "💪🏾", "💪🏿"],
    },
    Emoji {
        text: "✨",
        name: "sparkles",
        keywords: "star magic shine",
        variants: &[],
    },
    Emoji {
        text: "✅",
        name: "check mark",
        keywords: "yes done correct tick",
        variants: &[],
    },
    Emoji {
        text: "❌",
        name: "cross mark",
        keywords: "no wrong cancel",
        variants: &[],
    },
    Emoji {
        text: "🤝",
        name: "handshake",
        keywords: "hand agreement deal",
        variants: &[],
    },
    Emoji {
        text: "👩‍💻",
        name: "woman technologist",
        keywords: "computer developer work",
        variants: &[],
    },
    Emoji {
        text: "🇮🇳",
        name: "India flag",
        keywords: "country indian flag",
        variants: &[],
    },
    Emoji {
        text: "😞",
        name: "disappointed face",
        keywords: "sad down unhappy",
        variants: &["😞", "😢", "😭"],
    },
    Emoji {
        text: "😢",
        name: "crying face",
        keywords: "sad tears cry",
        variants: &["😢", "😭", "🥲"],
    },
    Emoji {
        text: "😠",
        name: "angry face",
        keywords: "angry mad annoyed rage",
        variants: &["😠", "😡", "🤬"],
    },
    Emoji {
        text: "😡",
        name: "enraged face",
        keywords: "angry mad furious rage",
        variants: &["😠", "😡", "🤬"],
    },
    Emoji {
        text: "🤬",
        name: "face with symbols on mouth",
        keywords: "angry furious swear rage",
        variants: &["😠", "😡", "🤬"],
    },
];

#[derive(Clone, Copy)]
pub struct Intensity {
    pub choices: &'static [&'static str],
    pub labels: &'static [&'static str],
}

/// Curated expressive sequences, not a claim that emotions have a universal scale.
pub fn intensity(text: &str) -> Option<Intensity> {
    let (choices, labels): (&[&str], &[&str]) = match text {
        "🥲" | "😞" | "😢" | "😭" => {
            (&["😞", "😢💔", "😭💔🥀"], &["Sad", "Sadder", "Saddest"])
        }
        "😠" | "😡" | "🤬" => (&["😠", "😡💢", "🤬💢🔥"], &["Angry", "Angrier", "Angriest"]),
        "😂" | "🤣" => (
            &["😄", "😂🤣", "🤣💀😭"],
            &["Amused", "Laughing", "Losing it"],
        ),
        "❤️" | "😍" => (&["❤️", "🥰❤️", "😍❤️‍🔥🫶"], &["Love", "Adore", "Obsessed"]),
        "😀" | "😊" => (&["😊", "😄✨", "🤩🎉✨"], &["Happy", "Happier", "Happiest"]),
        "🎉" => (
            &["🎉", "🥳🎊", "🥳🎉🎊"],
            &["Celebrate", "Party", "Party time"],
        ),
        "🔥" => (&["🔥", "🔥💥", "🔥💥🚀"], &["Hot", "Hotter", "On fire"]),
        _ => return None,
    };
    Some(Intensity { choices, labels })
}

/// All query words must match; an empty query preserves the home ordering.
pub fn search(query: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    CATALOG
        .iter()
        .enumerate()
        .filter_map(|(index, emoji)| {
            let words = format!("{} {} {}", emoji.name, emoji.keywords, emoji.text).to_lowercase();
            query
                .split_whitespace()
                .all(|word| words.contains(word))
                .then_some(index)
        })
        .collect()
}

pub fn page(results: &[usize], number: usize) -> &[usize] {
    let start = number.saturating_mul(8).min(results.len());
    &results[start..start.saturating_add(8).min(results.len())]
}

pub fn page_count(results: &[usize]) -> usize {
    results.len().div_ceil(8).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_requires_all_words_and_ignores_case() {
        let found = search("  THUMB hand ");
        assert_eq!(
            found.iter().map(|i| CATALOG[*i].text).collect::<Vec<_>>(),
            ["👍", "👎"]
        );
        assert!(search("thumb party").is_empty());
        assert_eq!(search("👩‍💻"), [22]);
    }

    #[test]
    fn intensity_sequences_are_named_searchable_and_preserve_combinations() {
        for text in ["🥲", "😭", "😠", "😡", "🤬", "😂", "❤️", "😊", "🎉", "🔥"]
        {
            let levels = intensity(text).unwrap();
            assert_eq!(levels.choices.len(), 3);
            assert_eq!(levels.labels.len(), 3);
            assert_ne!(levels.choices[0], levels.choices[2]);
            for output in levels.choices {
                assert_eq!(
                    String::from_utf16(&crate::unicode_units(output)).unwrap(),
                    *output
                );
            }
        }
        assert_eq!(intensity("😠").unwrap().choices[2], "🤬💢🔥");
        assert_eq!(intensity("😭").unwrap().choices[2], "😭💔🥀");
        assert!(search("angry").len() >= 3);
        assert!(intensity("👍").is_none());
    }

    #[test]
    fn pages_cover_all_results_without_duplicates() {
        let found = search("");
        let flattened: Vec<_> = (0..page_count(&found))
            .flat_map(|p| page(&found, p).iter().copied())
            .collect();
        assert_eq!(flattened, found);
        assert!(page(&found, usize::MAX).is_empty());
        assert_eq!(page_count(&[]), 1);
    }
}
