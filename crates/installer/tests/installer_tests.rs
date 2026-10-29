use multiverse_installer::{
    IsoDownloaderCatalog, HelperCli, ConfiguratorServer, TranslationMap
};
use multiverse_bootcore::config::Config;
use multiverse_storage::partition::PartitionTable;
use multiverse_storage::constants::{DATA_START_SECTOR, BOOT_PARTITION_SECTORS};

#[test]
fn test_iso_downloader_catalog() {
    let catalog = IsoDownloaderCatalog::new();
    let list = catalog.list();
    assert!(!list.is_empty());

    let script = catalog.generate_download_script("ubuntu-22.04", "/var/images").unwrap();
    assert!(script.contains("curl -L -o /var/images/ubuntu-22.04.iso"));
}

#[test]
fn test_helper_cli_validation() {
    let disk_size = (DATA_START_SECTOR + BOOT_PARTITION_SECTORS + 1000) * 512;
    let table = PartitionTable::canonical(disk_size).unwrap();
    assert!(HelperCli::validate_partition_table(&table));
}

#[test]
fn test_configurator_server_update() {
    let cfg = Config::default();
    let mut server = ConfiguratorServer::new(8080, cfg);
    let updated_json = r#"{"control":{"timeout":45}}"#;
    let res = server.handle_update_json(updated_json).unwrap();
    assert!(res.contains("\"timeout\": 45"));
    assert_eq!(server.current_config.control.unwrap().timeout, Some(45));
}

#[test]
fn test_translation_map() {
    let en = TranslationMap::english();
    assert_eq!(en.translate("welcome"), "Welcome to Multiverse Installer");

    let es = TranslationMap::spanish();
    assert_eq!(es.translate("welcome"), "Bienvenido al Instalador Multiverse");
}
