use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    French,
    English,
}
impl Default for Language {
    fn default() -> Self {
        Self::English
    }
}
static ENGLISH: AtomicBool = AtomicBool::new(true);
pub fn set_language(language: Language) {
    ENGLISH.store(language == Language::English, Ordering::Relaxed);
}
pub fn english() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}
pub fn text(french: &'static str, english: &'static str) -> &'static str {
    if self::english() { english } else { french }
}
pub fn shortcut_label(key: &str) -> String {
    if !english() {
        return key.to_owned();
    }
    key.split('+')
        .map(|part| match part {
            "Maj" => "Shift",
            "Espace" => "Space",
            "Suppr" => "Delete",
            "Début" => "Home",
            "Fin" => "End",
            "Gauche" => "Left",
            "Haut" => "Up",
            "Droite" => "Right",
            "Bas" => "Down",
            other => other,
        })
        .collect::<Vec<_>>()
        .join("+")
}
#[macro_export]
macro_rules! translated_format {($fr:literal,$en:literal $(,$argument:expr)* $(,)?)=>{if $crate::i18n::english(){format!($en $(,$argument)*)}else{format!($fr $(,$argument)*)}};}
