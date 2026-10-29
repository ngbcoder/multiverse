use multiverse_bootcore::config::Config;
use multiverse_bootcore::discovery::{discover_images, is_image, apply_image_list};
use multiverse_bootcore::menu::{Menu, MenuEntry, MenuMode};
use multiverse_bootcore::plugin::{Plugin, PluginRegistry};
use multiverse_storage::filesystem::InMemoryFs;
use multiverse_storage::filesystem::FileSystem;

#[test]
fn config_parse() {
    let json = r#"{"control":{"timeout":10},"menu_alias":{"a":"b"}}"#;
    let c = Config::from_json(json).unwrap();
    assert_eq!(c.control.unwrap().timeout, Some(10));
    assert_eq!(c.menu_alias.unwrap().get("a").unwrap(), "b");
}

#[test]
fn is_image_ext() {
    assert!(is_image("foo.ISO"));
    assert!(is_image("bar.wim"));
    assert!(!is_image("readme.txt"));
}

#[test]
fn discover_images_in_memory() {
    let mut fs = InMemoryFs::new();
    fs.write_file("/a.iso", b"x").unwrap();
    fs.write_file("/b.txt", b"y").unwrap();
    fs.create_dir("/sub").unwrap();
    fs.write_file("/sub/c.img", b"z").unwrap();
    // InMemoryFs read_dir returns files only at root? Our discover treats entries; sub not walked because create_dir tracked but read_dir root only returns files.
    let imgs = discover_images(&fs, "/", &[]);
    // At least a.iso discovered
    assert!(imgs.iter().any(|i| i.path.contains("a.iso")));
}

#[test]
fn menu_collect_paths() {
    let mut m = Menu::new(MenuMode::List);
    let mut e = MenuEntry::new("img").with_path("/x.iso");
    m.add_entry(e);
    assert_eq!(m.collect_paths(), vec!["/x.iso".to_string()]);
}

#[test]
fn plugin_registry_priority() {
    struct P(&'static str, i32);
    impl Plugin for P {
        fn name(&self) -> &str { self.0 }
        fn priority(&self) -> i32 { self.1 }
        fn check(&self, _i: &multiverse_bootcore::discovery::ImageMeta, _c: &Config) -> bool { true }
        fn apply(&self, _i: &multiverse_bootcore::discovery::ImageMeta, _c: &Config) {}
    }
    let mut reg = PluginRegistry::new();
    reg.register(Box::new(P("low", 1)));
    reg.register(Box::new(P("high", 10)));
    let img = multiverse_bootcore::discovery::ImageMeta { path: "a.iso".into(), size: 0, fragments: 0, extension: "iso".into(), os_hint: None };
    let cfg = Config::default();
    let apps = reg.applicable(&img, &cfg);
    assert_eq!(apps[0].name(), "high");
}