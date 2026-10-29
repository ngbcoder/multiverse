//! Multiverse Installer library: core installer options, ISO downloader, helper CLI, configurator, and i18n.

pub mod configurator;
pub mod downloader;
pub mod helper_cli;
pub mod i18n;

pub use configurator::ConfiguratorServer;
pub use downloader::{IsoDescriptor, IsoDownloaderCatalog};
pub use helper_cli::HelperCli;
pub use i18n::TranslationMap;
