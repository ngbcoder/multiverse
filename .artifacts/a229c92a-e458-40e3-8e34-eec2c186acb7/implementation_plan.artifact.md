# Milestone 4: OS Adapters & Chain Data Implementation Plan

Implement OS-specific boot adapters and Chain Data Builder in `crates/bootcore` to support booting Linux, Windows, Unix, and generic OS images via chunk maps and parameter blocks.

## User Review Required

- **Adapter Architecture**: We will introduce an `OsAdapter` trait and concrete implementations (`LinuxAdapter`, `WindowsAdapter`, `UnixAdapter`, `GenericAdapter`) along with a `ChainDataBuilder` in `crates/bootcore`.
- **Chain Data Generation**: Assembles `ChainHead`, `ImgChunk`, `OverrideChunk`, `VirtChunk`, and `RamdiskParam` required by the UEFI boot flow and Linux runtime.

## Open Questions

- None at this stage; requirements align with `DESIGN.md` and `todo.md`.

## Proposed Changes

### [bootcore]

#### [NEW] [adapters.rs](file:///C:/Users/melis/NBprojects/rust/multiverse/crates/bootcore/src/adapters.rs)
- Define `OsType` enum (Linux, Windows, Unix, ChromeOS, VmwareXen, Generic).
- Define `OsAdapter` trait and implementations for Linux (kernel/initrd detection, cmdline generation), Windows (WIM/VHD/wimboot config), Unix, and Generic fallback.

#### [NEW] [chain.rs](file:///C:/Users/melis/NBprojects/rust/multiverse/crates/bootcore/src/chain.rs)
- Implement `ChainDataBuilder` to assemble `ChainHead`, chunk maps, override chunks, virtual chunks, and ramdisk parameters from image metadata and partition info.

#### [MODIFY] [lib.rs](file:///C:/Users/melis/NBprojects/rust/multiverse/crates/bootcore/src/lib.rs)
- Export `adapters` and `chain` modules and public types (`OsType`, `OsAdapter`, `ChainDataBuilder`).

#### [NEW] [adapters_tests.rs](file:///C:/Users/melis/NBprojects/rust/multiverse/crates/bootcore/tests/adapters_tests.rs)
- Add comprehensive unit tests verifying Linux adapter kernel/initrd discovery, Windows adapter configuration, and chain data assembly.

## Verification Plan

### Automated Tests
- Run `cargo test` across the workspace to ensure all unit tests pass successfully.
