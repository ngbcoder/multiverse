# Multiverse

Multiverse is a Rust workspace for building a multi-image bootable storage platform. The project prepares a bootable device once and then lets users copy and boot OS images directly from a data partition without reformatting or extracting the image files each time.

The codebase centers on:

- a bootloader/core runtime for image discovery and menu generation
- firmware abstractions for BIOS and UEFI flows
- storage and partition helpers
- installer tooling for preparing target media
- runtime integration for Linux and Windows-style boot chains

## Goals

- Install once, boot many images
- Boot ISO, WIM, IMG, VHD/VHDX, and EFI images in place
- Support both BIOS and UEFI systems
- Work with MBR and GPT layouts
- Provide secure-boot aware boot paths and runtime injection hooks
- Keep the platform extensible through configuration and plug-in style behavior

## Repository layout

- `Cargo.toml` — workspace configuration
- `crates/bootcore` — boot menu logic, discovery, adapters, plugin/config flow
- `crates/core` — core chunk map and shared logic
- `crates/firmware` — BIOS/UEFI boot flow and platform abstractions
- `crates/installer` — installer CLI and related automation
- `crates/runtime` — runtime helpers, cpio, ISO mounting, Windows PE support
- `crates/storage` — partitioning, filesystem, GPT/MBR, FAT logic
- `DESIGN.md` — architecture and on-disk layout details
- `INSTALL/` — install-related assets and configuration files
- `scripts/` — build and helper scripts

## Requirements

- Rust toolchain (stable edition 2021)
- Cargo
- Optional: QEMU or another virtualization environment for boot testing
- Linux/macOS/Windows host depending on the tooling you want to exercise

## Quick start

From the repository root:

```bash
cargo build --workspace
```

To run the installer crate directly:

```bash
cargo run -p multiverse-installer -- --help
```

To run the workspace tests:

```bash
cargo test --workspace
```

## Architecture summary

The project is designed around a two-partition layout:

- a data partition for image files and configuration
- a boot/EFI partition for firmware payloads, bootloader files, and runtime assets

The platform discovers available images, builds a menu, and then dispatches to OS-specific boot adapters. Linux runtime injection and Windows PE helpers are included to bridge the image-as-file model with real guest OS expectations.

## Documentation

- `DESIGN.md` — design overview and subsystem details
- `DESIGN_M6.md` — additional design notes and milestone material
- `todo.md` — current project task tracking

## Notes

This project is an active engineering workspace and is best treated as a platform prototype or research codebase. The boot stack and installer behaviors are intentionally modular and designed to evolve as more hardware and OS scenarios are supported.

## License

This project is published under the workspace license settings defined in `Cargo.toml`.
