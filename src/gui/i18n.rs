use serde::{Deserialize, Serialize};
use std::borrow::Cow;

include!(concat!(env!("OUT_DIR"), "/catalogs.rs"));

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Locale {
    #[default]
    English,
    Indonesian,
}

impl Locale {
    pub const ALL: [Self; 2] = [Self::English, Self::Indonesian];

    pub fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Indonesian => "Bahasa Indonesia",
        }
    }

    pub fn detect() -> Self {
        fastframe_i18n::detect(|tag| match tag.language.as_str() {
            "id" | "in" => Some(Self::Indonesian),
            "en" => Some(Self::English),
            _ => None,
        })
        .unwrap_or_default()
    }

    pub fn tr(self, source: &'static str) -> Cow<'static, str> {
        fastframe_i18n::gettext(self, source)
    }
}

impl fastframe_i18n::Locale for Locale {
    fn catalog(self) -> Option<&'static dyn fastframe_i18n::Translator> {
        match self {
            Self::English => None,
            Self::Indonesian => Some(&id::Translator),
        }
    }
}
