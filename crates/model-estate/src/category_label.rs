//! Opt-in decode of short category codes → human labels on complete receipts.
//!
//! Letter-trained classify seats return one letter. Mixed UX wants the word
//! label beside the raw letter. Default is off (safe for letter contracts).
//! Enable with binding `params.category_codec` (codec name) and/or
//! `CELL_COMPLETE_LABEL=1` when the binding id matches a registered codec.
//!
//! AG News A–D is the first codec. Other fixed-class classify presets can
//! register beside it. Maps mirror `estate-control` classify_import class
//! tables (A=World … D=Sci/Tech for ag_news).

use estate_schema::ModelBinding;

/// One letter → human label row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategoryLetter {
    pub letter: char,
    pub label: &'static str,
}

/// Named codec a binding or env can select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategoryCodec {
    pub name: &'static str,
    pub letters: &'static [CategoryLetter],
}

const AG_NEWS: CategoryCodec = CategoryCodec {
    name: "ag_news",
    letters: &[
        CategoryLetter {
            letter: 'A',
            label: "World",
        },
        CategoryLetter {
            letter: 'B',
            label: "Sports",
        },
        CategoryLetter {
            letter: 'C',
            label: "Business",
        },
        CategoryLetter {
            letter: 'D',
            label: "Sci/Tech",
        },
    ],
};

const DEVIGN: CategoryCodec = CategoryCodec {
    name: "devign",
    letters: &[
        CategoryLetter {
            letter: 'A',
            label: "Secure",
        },
        CategoryLetter {
            letter: 'B',
            label: "Insecure",
        },
    ],
};

const RUST_IDIOM: CategoryCodec = CategoryCodec {
    name: "rust_idiom",
    letters: &[
        CategoryLetter {
            letter: 'A',
            label: "NeedsFix",
        },
        CategoryLetter {
            letter: 'B',
            label: "Idiomatic",
        },
    ],
};

/// Built-in codecs. New classify specialties register here.
pub const CODECS: &[CategoryCodec] = &[AG_NEWS, DEVIGN, RUST_IDIOM];

/// Look up a codec by name (binding id or `params.category_codec`).
pub fn codec_by_name(name: &str) -> Option<&'static CategoryCodec> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    CODECS
        .iter()
        .find(|codec| codec.name.eq_ignore_ascii_case(name))
}

/// Map a raw completion to a human label when it is exactly one codec letter
/// (optional trivial wrappers: whitespace, trailing period, quotes, parens).
pub fn decode_letter(codec: &CategoryCodec, raw: &str) -> Option<&'static str> {
    let letter = parse_choice_letter(raw, codec)?;
    codec
        .letters
        .iter()
        .find(|row| row.letter == letter)
        .map(|row| row.label)
}

fn parse_choice_letter(raw: &str, codec: &CategoryCodec) -> Option<char> {
    let trimmed = raw.trim();
    let stripped = trimmed
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '(' | ')' | '[' | ']' | '`'))
        .trim()
        .trim_end_matches(|c: char| matches!(c, '.' | ')' | ',' | ';'))
        .trim();
    let mut chars = stripped.chars();
    let ch = chars.next()?;
    if chars.next().is_some() || !ch.is_ascii_alphabetic() {
        return None;
    }
    let up = ch.to_ascii_uppercase();
    if codec.letters.iter().any(|row| row.letter == up) {
        Some(up)
    } else {
        None
    }
}

/// Env opt-in. `1` / `true` / `yes` / `on` enable. Unset or other values stay off.
pub fn complete_label_env_enabled() -> bool {
    match std::env::var("CELL_COMPLETE_LABEL") {
        Ok(raw) => {
            let v = raw.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "on")
        }
        Err(_) => false,
    }
}

/// Resolve the codec for a binding when label decode is enabled.
///
/// Order:
/// 1. `params.category_codec` (string codec name) — enables for that binding
/// 2. else `CELL_COMPLETE_LABEL=1` and binding id matches a registered codec
///
/// Default off when neither is set (letter-trained seats stay raw).
pub fn resolve_codec(binding: &ModelBinding) -> Option<&'static CategoryCodec> {
    if let Some(name) = binding
        .params
        .get("category_codec")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return codec_by_name(name);
    }
    if complete_label_env_enabled() {
        return codec_by_name(&binding.id);
    }
    None
}

/// When a codec applies and `completion` is a mapped letter, return the label.
pub fn completion_label_for(binding: &ModelBinding, completion: &str) -> Option<&'static str> {
    let codec = resolve_codec(binding)?;
    decode_letter(codec, completion)
}

#[cfg(test)]
mod tests {
    use super::*;
    use estate_schema::ModelClass;
    use serde_json::json;

    fn binding(id: &str, params: serde_json::Value) -> ModelBinding {
        ModelBinding {
            id: id.into(),
            class: ModelClass::Local,
            driver: "ollama".into(),
            params,
            wired: true,
        }
    }

    #[test]
    fn ag_news_letters_decode() {
        let codec = codec_by_name("ag_news").unwrap();
        assert_eq!(decode_letter(codec, "D"), Some("Sci/Tech"));
        assert_eq!(decode_letter(codec, " d."), Some("Sci/Tech"));
        assert_eq!(decode_letter(codec, "A"), Some("World"));
        assert_eq!(decode_letter(codec, "Sci/Tech"), None);
        assert_eq!(decode_letter(codec, "I pick D"), None);
    }

    #[test]
    fn param_codec_enables_without_env() {
        let _guard = EnvLock::set("CELL_COMPLETE_LABEL", None);
        let b = binding("ag_news", json!({"category_codec": "ag_news"}));
        assert_eq!(completion_label_for(&b, "D"), Some("Sci/Tech"));
        let off = binding("ag_news", json!({}));
        assert_eq!(completion_label_for(&off, "D"), None);
    }

    #[test]
    fn env_enables_when_binding_id_matches() {
        let _guard = EnvLock::set("CELL_COMPLETE_LABEL", Some("1"));
        let b = binding("ag_news", json!({}));
        assert_eq!(completion_label_for(&b, "B"), Some("Sports"));
        let unknown = binding("local_slm", json!({}));
        assert_eq!(completion_label_for(&unknown, "B"), None);
    }

    #[test]
    fn env_off_keeps_letter_safe() {
        let _guard = EnvLock::set("CELL_COMPLETE_LABEL", Some("0"));
        let b = binding("ag_news", json!({}));
        assert_eq!(completion_label_for(&b, "D"), None);
    }

    struct EnvLock {
        key: &'static str,
        prev: Option<String>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl EnvLock {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            use std::sync::{Mutex, OnceLock};
            static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
            let lock = LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let prev = std::env::var(key).ok();
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
            Self {
                key,
                prev,
                _lock: lock,
            }
        }
    }

    impl Drop for EnvLock {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
