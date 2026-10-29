# Implementation Milestones

Source: `DESIGN.md` (multi-image bootable storage platform).

## M0 — Project Foundation
- [x] Choose Rust edition/toolchain and workspace layout (`crates/` by subsystem).
- [x] Define shared core types: disk-handle, filesystem-handle, block-device, chunk-map, env-param block, platform id enum.
- [ ] Set up CI: fmt, clippy, tests, cross-build targets (i686, x86_64, aarch64, mips64el).
- [ ] Establish build orchestration script (all-in-one) and `INSTALL/` output tree.
- [ ] Document build environment (host OS, packages, toolchains, hashes).

## M1 — Storage / On-Disk Layer
- [x] Define partition table model (MBR + GPT), fixed constants (data start sector 2048, boot tail 65536 sectors).
- [x] Implement MBR writer (type 0x07/0xEF, signature at fixed offsets) + parser.
- [x] Implement GPT writer (protective MBR, header, partition entries) + parser.
- [x] Implement disk recognition (upgrade detection): sector-2048 start + fixed boot sector count.
- [x] FAT16/32 read/write library for boot partition and configurator (FAT32 volume: BPB parsing, cluster chains, file/dir read+write).
- [x] Abstract filesystem trait for data partition (trait + in-memory + FAT32 impls).

## M2 — Firmware Abstraction Layer
- [x] Legacy BIOS path: GRUB i386-pc integration, module loading, memdisk support.
- [x] UEFI application skeleton pieces: command-line parser (debug/fallback/memdisk/sector-size/env block).
- [x] Disk location: size checks + stored signature/GUID (host-testable).
- [x] Open boot partition FS from firmware (platform-specific).
- [x] Optional ISO9660/UDF EFI filesystem driver loading.
- [x] Virtual Block I/O device from chunk map (read-only, with read_blocks).
- [x] Sector-size conversion (512/2048/4096) (via chunkmap byte offsets).
- [x] Secure Boot: shim + MokManager integration, SecureBoot variable toggle (IA32/x86_64).
- [x] Platform id detection (compile-time target arch).
- [x] Memdisk mode: in-memory image + ramdisk_param.

## M3 — Bootloader Core: Discovery & Menu
- [x] Image Discovery Engine: walk filesystem, filter by extension, ignore rules, collect size.
- [x] Menu Engine: list + tree structures, aliases, tips, classes, passwords, custom entries.
- [x] Plugin/Config Engine: parse `config.json` into typed model; dispatch handlers.
- [x] Config sections: control, theme, image_list, menu_alias, menu_tip, menu_class, menu_extra.
- [x] Password gating (global + per-image) helper.

## M4 — Bootloader Core: OS Adapters & Chain Data
- [x] Linux adapter: locate kernel/initrd in ISO, collect initrds, inject runtime archive, build kernel cmdline (rdinit/init), persistence + dud disks.
- [x] Windows adapter: ISO/WIM/VHD, wimboot, native VHD loader, auto-install answer files.
- [x] Unix adapter: FreeBSD/OpenBSD/NetBSD/DragonFly with kernel-module disk access.
- [x] ChromeOS adapter.
- [x] VMware/Xen adapter.
- [x] Generic/other: memdisk or chainload fallback.
- [x] Chain Data Builder: assemble chain_head, img_chunk[], override_chunk[], virt_chunk[], ramdisk_param.

## M5 — Chunk Map & I/O Translation
- [x] Build chunk maps from filesystem extents (O(extents), not image size).
- [x] Image LBA → chunk → disk LBA translation with overrides + virtual sectors.
- [x] Override chunk handling (in-memory/alternate sectors).
- [x] Virt chunk synthesis (structure + translation handling).
- [x] Read-only enforcement on virtual device.

## M6 — UEFI Boot Flow
- [x] EFI entry point; environment persistence via UEFI variables (image loc, chunk counts, ramdisk params).
- [x] Boot selected image's EFI bootloader from virtual device (LoadImage/StartImage).
- [x] Fallback retry with flags (disable vdev opts, alternate sector size).
- [x] Cleanup/restore on return.

## M7 — Runtime Integration Layer (Linux)
- [x] Minimal busybox-based runtime cpio per arch.
- [x] Injected init: dispatch init_chain vs init_loop.
- [x] Chain mode: parse original initramfs, extract nested cpio (gzip/bzip2/xz/lzma/lz4/zstd), apply injection archive per layer, install tools/libs, exec chain script.
- [x] Chain script: OS detection, OS-specific hook (`hook/<os>/os-hook.sh`), live-injection hook, persistence setup, dud disks, exec real init (switch root if ramdisk).
- [x] FUSE ISO mount (`fuse_iso`).
- [x] Device-mapper setup (linear/striped targets).
- [x] Loop device path (contiguous/defragged images).
- [x] Windows PE helper (imdisk mount) — separate binary.

## M8 — Installers & Tooling
- [x] Windows GUI/CLI installer: enumerate drives, validate, create partitions, format, write payloads, detect existing install, upgrade in place, dry-run/force.
- [x] Linux shell installer (+ GTK/Qt frontends optional).
- [x] Helper CLI: FAT r/w, GPT header repair, partition resize.
- [x] Configurator: local HTTP server + web frontend editing config.json via FAT lib; validation + backups.
- [x] AoE (vblade) optional netboot target + client initiator.
- [x] Language/i18n files for menus and GUI.

## M9 — Security
- [ ] Secure Boot end-to-end (shim, key enrollment, variable toggle).
- [ ] Read-only image semantics enforced everywhere.
- [ ] Password protection wired into menu/boot.
- [ ] SHA-256 hashes for build artifacts + verification.
- [ ] Privilege checks in installers/runtime.

## M10 — Testing & Validation
- [ ] Unit tests for chunk-map translation, config parsing, partition math.
- [ ] Integration tests in VM (BIOS + UEFI) for boot of sample ISOs.
- [ ] Fuzz/edge: fragmented images, large files, unsupported FS.
- [ ] Debug mode (`debug` flag) logging to console/file.
- [ ] Graceful degradation: return to menu on boot failure.

## M11 — Packaging & Release
- [ ] Per-arch binary sets (tools, cpio, EFI drivers).
- [ ] Packaging script produces release archive from `INSTALL/`.
- [ ] Reproducible build docs.

## M12 — Extensibility Hooks
- [ ] Plugin registration API (name + check + entry, priority order).
- [ ] New-OS / new-FS / new-arch / new-image-format extension points documented.
- [ ] Config schema versioning + migration.
```