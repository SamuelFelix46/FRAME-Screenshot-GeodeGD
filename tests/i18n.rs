use frame_studio::{
    i18n::{self, Language},
    model::Settings,
    store::Store,
};

#[test]
fn unified_package_keeps_language_from_both_previous_packages() {
    let root = std::env::temp_dir().join(format!(
        "frame-language-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let store = Store::new(root).unwrap();
    assert_eq!(store.load_settings().unwrap().language, Language::English);
    let original_path = store.root.join("captures/existing.png");
    std::fs::write(&original_path, b"existing screenshot bytes").unwrap();

    for previous_build in [Language::French, Language::English] {
        for chosen_language in [Language::French, Language::English] {
            let legacy = serde_json::json!({
                "language": chosen_language,
                "last_build_language": previous_build,
                "capture_key": "Ctrl+P",
                "auto_copy": true
            });
            std::fs::write(
                store.root.join("settings.json"),
                serde_json::to_vec(&legacy).unwrap(),
            )
            .unwrap();
            let settings = store.load_settings().unwrap();
            assert_eq!(settings.language, chosen_language);
            assert_eq!(settings.capture_key, "Ctrl+P");
            assert!(settings.auto_copy);
            store.save_settings(&settings).unwrap();
            assert_eq!(store.load_settings().unwrap().language, chosen_language);
            let saved: serde_json::Value =
                serde_json::from_slice(&std::fs::read(store.root.join("settings.json")).unwrap())
                    .unwrap();
            assert!(saved.get("last_build_language").is_none());
            assert_eq!(
                std::fs::read(&original_path).unwrap(),
                b"existing screenshot bytes"
            );
        }
    }
    std::fs::remove_dir_all(store.root).unwrap();
}

#[test]
fn one_binary_switches_text_formats_and_shortcut_labels_at_runtime() {
    let settings = Settings::default();
    for language in [Language::French, Language::English, Language::French] {
        i18n::set_language(language);
        let english = language == Language::English;
        assert_eq!(i18n::english(), english);
        assert_eq!(
            i18n::text("Copier", "Copy"),
            if english { "Copy" } else { "Copier" }
        );
        assert_eq!(
            frame_studio::translated_format!("{0} captures", "{0} screenshots", 3),
            if english {
                "3 screenshots"
            } else {
                "3 captures"
            }
        );
        assert_eq!(
            i18n::shortcut_label("Ctrl+Maj+Suppr"),
            if english {
                "Ctrl+Shift+Delete"
            } else {
                "Ctrl+Maj+Suppr"
            }
        );
        assert_eq!(settings.capture_key, "F8");
    }
}
