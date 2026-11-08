//! Minimal pt-BR / en localisation. Language from BUNKR_LANG, else the OS locale.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Pt,
}

impl Lang {
    pub fn detect() -> Self {
        // ponytail: locale sniff only; add a language picker in Settings when asked.
        let tag = std::env::var("BUNKR_LANG")
            .ok()
            .or_else(|| {
                std::env::var("LANG")
                    .ok()
                    .or_else(|| std::env::var("LC_ALL").ok())
                    .or_else(windows_locale)
            })
            .unwrap_or_default()
            .to_lowercase();
        if tag.starts_with("pt") {
            Lang::Pt
        } else {
            Lang::En
        }
    }
}

#[cfg(windows)]
fn windows_locale() -> Option<String> {
    // GetUserDefaultUILanguage -> LANGID; primary language is the low 10 bits.
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    let id = unsafe { GetUserDefaultUILanguage() } & 0x3ff;
    // 0x16 = Portuguese (any region).
    Some(if id == 0x16 { "pt".to_string() } else { "en".to_string() })
}

#[cfg(not(windows))]
fn windows_locale() -> Option<String> {
    None
}

/// Picks the pt-BR text on `Lang::Pt` and the English one otherwise.
#[inline]
pub fn t(l: Lang, pt: &'static str, en: &'static str) -> &'static str {
    if l == Lang::Pt {
        pt
    } else {
        en
    }
}
