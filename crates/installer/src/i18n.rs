//! Language/i18n translation maps for menus and installer UI.

use std::collections::BTreeMap;

pub struct TranslationMap {
    pub lang_code: String,
    messages: BTreeMap<String, String>,
}

impl TranslationMap {
    pub fn english() -> Self {
        let mut messages = BTreeMap::new();
        messages.insert("welcome".to_string(), "Welcome to Multiverse Installer".to_string());
        messages.insert("select_target".to_string(), "Select target drive".to_string());
        messages.insert("confirm_install".to_string(), "Confirm installation?".to_string());
        messages.insert("done".to_string(), "Installation completed successfully!".to_string());
        Self {
            lang_code: "en".to_string(),
            messages,
        }
    }

    pub fn spanish() -> Self {
        let mut messages = BTreeMap::new();
        messages.insert("welcome".to_string(), "Bienvenido al Instalador Multiverse".to_string());
        messages.insert("select_target".to_string(), "Seleccionar disco de destino".to_string());
        messages.insert("confirm_install".to_string(), "¿Confirmar instalación?".to_string());
        messages.insert("done".to_string(), "¡Instalación completada con éxito!".to_string());
        Self {
            lang_code: "es".to_string(),
            messages,
        }
    }

    pub fn translate<'a>(&'a self, key: &'a str) -> &'a str {
        self.messages.get(key).map(|s| s.as_str()).unwrap_or(key)
    }
}
