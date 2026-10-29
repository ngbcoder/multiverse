#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuMode {
    List,
    Tree,
}

#[derive(Debug, Clone)]
pub struct MenuEntry {
    pub name: String,
    pub path: Option<String>,
    pub class: Option<String>,
    pub tip: Option<String>,
    pub password: Option<String>,
    pub children: Vec<MenuEntry>,
}

impl MenuEntry {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: None,
            class: None,
            tip: None,
            password: None,
            children: Vec::new(),
        }
    }

    pub fn with_path(mut self, p: impl Into<String>) -> Self {
        self.path = Some(p.into());
        self
    }

    pub fn add_child(&mut self, child: MenuEntry) {
        self.children.push(child);
    }
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub mode: MenuMode,
    pub entries: Vec<MenuEntry>,
    pub auto_boot_delay: u32,
}

impl Menu {
    pub fn new(mode: MenuMode) -> Self {
        Self {
            mode,
            entries: Vec::new(),
            auto_boot_delay: 5,
        }
    }

    pub fn add_entry(&mut self, entry: MenuEntry) {
        self.entries.push(entry);
    }

    /// Apply aliases from config.
    pub fn apply_aliases(&mut self, aliases: &std::collections::BTreeMap<String, String>) {
        for e in &mut self.entries {
            if let Some(a) = aliases.get(&e.name) {
                e.name = a.clone();
            }
            // recurse not strictly needed for top-level
        }
    }

    /// Flatten tree to list of paths (for auto-boot).
    pub fn collect_paths(&self) -> Vec<String> {
        let mut out = Vec::new();
        for e in &self.entries {
            if let Some(p) = &e.path {
                out.push(p.clone());
            }
            for c in &e.children {
                if let Some(p) = &c.path {
                    out.push(p.clone());
                }
            }
        }
        out
    }

    /// Inject custom entries from config (menu_extra).
    pub fn inject_custom(&mut self, customs: &[String]) {
        for c in customs {
            let mut e = MenuEntry::new(c.clone());
            e.class = Some("custom".to_string());
            self.entries.push(e);
        }
    }

    /// Check if an entry requires a password and whether provided matches.
    pub fn check_password(entry: &MenuEntry, provided: Option<&str>, global: Option<&str>) -> bool {
        let required = entry.password.as_deref().or(global);
        match required {
            None => true,
            Some(req) => provided == Some(req),
        }
    }
}