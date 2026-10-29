//! Bootloader core: discovery, menu, config model, plugin engine, adapters, chain builder.

pub mod adapters;
pub mod chain;
pub mod config;
pub mod discovery;
pub mod menu;
pub mod plugin;

pub use adapters::{get_adapter, BootParams, OsAdapter, OsType, LinuxAdapter, WindowsAdapter, UnixAdapter, GenericAdapter};
pub use chain::ChainDataBuilder;
pub use config::Config;
pub use discovery::{discover_images, ImageMeta};
pub use menu::{Menu, MenuEntry, MenuMode};
pub use plugin::{Plugin, PluginRegistry};