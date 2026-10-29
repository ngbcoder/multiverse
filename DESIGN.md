# Design Document

## 1. Purpose and Scope

This document describes the design of a **multi-image bootable storage platform**: a system that installs a small, reusable bootloader environment onto a removable or fixed storage device once, after which an arbitrary number of operating-system image files (ISO, WIM, IMG, VHD/VHDX, EFI) can be copied onto the device's data partition and booted directly, without reformatting or extracting the images.

The platform supports:

- Legacy BIOS and UEFI firmware (IA32, x86_64, AArch64, MIPS64EL).
- MBR and GPT partitioning.
- Secure Boot (IA32/x86_64).
- Browsing and booting images located on the local disk of the host machine.
- Runtime integration with the booted operating system (persistence, driver injection, automated installation).

This document is **architectural**: it defines subsystems, on-disk layout, boot and runtime flows, configuration model, and extension points. It intentionally avoids product- or module-specific identifiers; components are named by role.

---

## 2. Design Goals and Non-Goals

### 2.1 Goals

1. **Install once, use many**: a single installation step prepares the device; subsequent use is "copy image files and reboot".
2. **No extraction**: images are booted in place, including images larger than the filesystem's file-size limit and fragmented files.
3. **Multi-OS, multi-arch**: one installation supports Windows, WinPE, Linux, Unix, ChromeOS, VMware/Xen, and others across several firmware architectures.
4. **Non-destructive upgrades**: upgrading the platform preserves user data and image files.
5. **Extensibility**: a plugin/configuration framework allows behavior customization without recompiling.
6. **Resilience**: boot failures degrade gracefully (fallback paths, memdisk mode, debug logging).
7. **Portability of runtime environment**: a small Linux-based runtime is injected into guest Linux kernels to bridge the platform's storage model with the guest OS.

### 2.2 Non-Goals

- The platform is not a general-purpose operating system installer or package manager.
- It does not modify image contents at copy time (images are treated as read-only artifacts).
- It does not provide network-based provisioning as a primary path (network boot is an optional auxiliary).

---

## 3. High-Level Architecture

The platform is composed of five cooperating layers:

```
+---------------------------------------------------------------+
| 5. Configuration & Tooling Layer                              |
|    (GUI/CLI configurator, installers, helper utilities)        |
+---------------------------------------------------------------+
| 4. Runtime Integration Layer                                  |
|    (Linux initramfs injection, FUSE mounts, device-mapper,     |
|     Windows PE mounting helpers)                               |
+---------------------------------------------------------------+
| 3. Bootloader Core                                            |
|    (menu generation, image discovery, plugin engine, OS boot    |
|     adapters, chainloading)                                     |
+---------------------------------------------------------------+
| 2. Firmware Abstraction Layer                                 |
|    (Legacy BIOS GRUB, UEFI application with virtual block I/O) |
+---------------------------------------------------------------+
| 1. Storage / On-Disk Layer                                    |
|    (two-partition layout, image files, config, payloads)        |
+---------------------------------------------------------------+
```

**Data flow (happy path)**:

1. Firmware (BIOS or UEFI) loads the platform's first-stage bootloader from the boot partition.
2. The bootloader core scans the data partition, discovers image files, applies filters/aliases from configuration, and renders a menu.
3. On selection, the bootloader prepares an *image descriptor* (chunk map + OS-specific boot parameters) and either chainloads the image's own bootloader (UEFI virtual block device) or invokes kernel/initrd directly (Linux).
4. For Linux, the platform injects a runtime archive into the kernel's initramfs; the injected `init` transitions to the platform's chain script, which performs OS detection, hooks, persistence setup, and finally execs the real init.
5. For Windows, the platform either chainloads the Windows bootloader from the virtual device or uses memory-disk/PE helpers for WIM/VHD scenarios.

---

## 4. Major Subsystems

### 4.1 Storage and On-Disk Layer

**Responsibility**: define and manage the physical layout of an installed device and the placement of platform payloads, configuration, and user images.

**Key concepts**:

- **Boot partition** (small, FAT16/32, labeled by convention): holds the bootloader binaries, EFI applications, GRUB modules, EFI drivers (ISO9660/UDF), compressed runtime cpio archives, and tools.
- **Data partition** (large, user-chosen filesystem): holds OS image files in arbitrary locations plus a platform directory containing `config.json`, theme assets, persistence backends, and injection files.
- **Image files**: treated as opaque, read-only blobs; may be fragmented; addressed by a chunk map rather than by contiguous extents.

**Invariant**: the boot partition size and starting sector are fixed constants so that firmware and installers can locate it deterministically. The data partition always begins at a fixed sector (2048) and the boot partition always occupies a fixed-size tail region.

### 4.2 Firmware Abstraction Layer

**Responsibility**: present a uniform interface to the bootloader core regardless of whether the machine booted via Legacy BIOS or UEFI.

**Legacy BIOS path**:

- A modified GRUB2 (i386-pc target) is installed into the MBR gap / boot partition.
- GRUB modules (including a large platform-specific module set) provide image discovery, menu rendering, plugin parsing, and OS adapters.
- Memory-disk support allows small images to be loaded entirely into RAM.

**UEFI path**:

- A UEFI application (built with an EDK2-based framework) is launched by firmware from the boot partition's ESP.
- It parses its own command line (debug, fallback, memdisk, sector-size overrides, environment-parameter block).
- It locates the platform's disk by combining size checks with a stored disk signature/GUID.
- It opens the boot partition's filesystem, optionally loads ISO9660/UDF EFI filesystem drivers, and installs a **virtual block I/O device** that represents the selected image file.
- It then boots the image's own EFI bootloader from that virtual device.
- Secure Boot is supported via a shim + machine-owner-key manager; the platform can adjust the SecureBoot UEFI variable to satisfy Linux distributions that require it.

**Common abstractions**: disk-handle, filesystem-handle, block-device, chunk-map, environment-parameter block, platform identifier (legacy/IA32/x64/AArch64/MIPS64EL).

### 4.3 Bootloader Core

**Responsibility**: discover images, build menus, interpret configuration, and dispatch to OS-specific boot adapters.

**Subcomponents**:

- **Image Discovery Engine**: walks the data partition (and optionally local disks), filters by extension and ignore rules, collects metadata (size, fragmentation).
- **Menu Engine**: supports list and tree-view modes, aliases, tips, classes (icons), passwords, and custom entries.
- **Plugin/Config Engine**: parses the JSON configuration file into a typed model and dispatches to registered plugin handlers.
- **OS Boot Adapters**:
  - **Linux adapter**: locates kernel/initrd within ISO, collects initrds, injects runtime archive, constructs kernel command line (including `rdinit`/`init` redirection), handles persistence and driver-update disks.
  - **Windows adapter**: handles ISO/WIM/VHD boot, WIM boot via wimboot, VHD boot via native VHD loader, auto-install answer files.
  - **Unix adapter**: FreeBSD/OpenBSD/NetBSD/DragonFly, with kernel-module-based disk access.
  - **ChromeOS adapter**: dedicated path.
  - **VMware/Xen adapter**: dedicated path.
  - **Generic/other**: memdisk or chainload fallback.
- **Chain Data Builder**: assembles the `chain_head` structure (image location, chunk counts, override/virtual chunks, ramdisk parameters) consumed by the firmware layer.

### 4.4 Runtime Integration Layer

**Responsibility**: bridge the platform's image-as-file model with a booted Linux kernel's expectation of a block device/root filesystem.

**Mechanism**:

- A minimal Linux runtime (busybox-based, per-architecture) is packaged as cpio archives and injected into the guest kernel's initramfs.
- The injected `init` inspects the kernel command line:
  - Normal path: unpack the real initramfs layers, apply injection, then hand off.
  - Chain path: perform chained initramfs extraction with decompression (gzip/bzip2/xz/lzma/lz4/zstd), apply the platform's injection archive, install runtime tools, then exec the platform's chain script.
- The chain script detects OS type from kernel version and filesystem layout, runs OS-specific hooks and live-injection hooks, sets up persistence, and finally execs the real init (switching root if a ramdisk root was used).
- **FUSE-based ISO mount** (`fuse_iso`): mounts the image as an ISO9660/UDF filesystem in userspace for the guest.
- **Device-mapper setup**: builds mapping tables so the kernel sees the image as a block device.
- **Windows PE helper**: a small executable plus imdisk mounts the image inside Windows PE.
- **AoE (ATA over Ethernet)**: optional network-block-device path for netboot scenarios.

### 4.5 Configuration and Tooling Layer

**Responsibility**: user-facing installation, configuration, and maintenance.

**Components**:

- **Installers**: Windows GUI/CLI installer and Linux shell installer. Enumerate physical drives, validate, create partitions, format, write payloads, detect existing installations, support upgrade-in-place.
- **Helper CLI**: small utility for FAT read/write, GPT header repair, partition resizing; used by both installers and runtime.
- **Configurator (web UI)**: a local HTTP server + web frontend that edits the JSON configuration file through a FAT library, with plugin-aware forms.
- **GUI frontends**: GTK/Qt-based Linux installers; a server component for remote/automated installation.
- **Language/i18n**: language files for menus and GUI.

---

## 5. On-Disk Layout (Detailed)

### 5.1 Partition Table

Two partitions are created on the target device.

| Partition | Position | Size | Type | Filesystem | Label | Purpose |
|---|---|---|---|---|---|---|
| 1 (Data) | sector 2048 → end−reserved | remainder | MBR 0x07 / GPT "primary data" | user choice (FAT32/exFAT/NTFS/UDF/XFS/Btrfs/Ext2-4) | (none fixed) | image files + platform config dir |
| 2 (Boot/ESP) | tail region | fixed 65536 sectors (32 MiB) | MBR 0xEF / GPT "EFI system" | FAT16 | fixed label | bootloader, EFI apps, drivers, runtime cpio, tools |

**MBR layout**:

- Partition 1: type 0x07 (or appropriate), bootable flag set.
- Partition 2: type 0xEF.
- A platform signature and GUID are stored at fixed MBR offsets to allow the UEFI application to identify the device.

**GPT layout**:

- Protective MBR + GPT header (repaired by helper CLI after creation).
- Partition 1 named `Data`-equivalent; partition 2 named `Boot`-equivalent (EFI system partition).
- Partition entries carry type GUIDs for data and EFI system.

**Detection**: a disk is recognized as an installed device when (a) partition 1 starts at sector 2048, and (b) partition 2 has exactly the fixed sector count. This enables upgrade detection and re-installation without data loss.

### 5.2 Boot Partition Contents (logical)

```
/boot-partition/
  EFI/BOOT/            # firmware entry points (BOOTIA32/BOOTX64/BOOTAA64/BOOTMIPS.EFI)
                        # shim, MokManager, GRUB EFI images
  platform/              # core EFI apps (core_*.efi, util_*.efi), ipxe kernel,
                        # memdisk, wimboot, ISO/UDF EFI drivers
  grub/                # GRUB core image, modules, themes, fonts, grub.cfg
  tool/                # per-arch busybox, xzcat, hexdump, ash, ...
  cpio/                # runtime injection archives (per arch)
  (locale/help files)
```

### 5.3 Data Partition Contents (logical)

```
/data-partition/
  *.iso, *.wim, *.img, *.vhd(x), *.efi   # arbitrary locations
  platform/
    config.json        # main configuration (UTF-8)
    runtime.cpio        # optional user injection archive
    custom_grub.cfg    # optional custom GRUB entries
    theme/             # theme assets
    (persistence backend images)
    (injection files)
```

---

## 6. Boot Flows

### 6.1 Legacy BIOS Boot Flow

1. Firmware executes MBR code in the boot partition gap.
2. GRUB2 (i386-pc) loads from the boot partition; reads `grub.cfg`.
3. Platform modules initialize: detect platform id (legacy), open data partition filesystem, load config.
4. Image discovery runs; filters and aliases applied; menu rendered (list or tree).
5. User selects an entry (or auto-boot timer fires).
6. Selected image's OS adapter prepares:
   - Linux: extract kernel/initrd from ISO, append runtime injection, build command line, invoke `linux`/`initrd` commands.
   - Windows: prepare WIM/VHD chain data; use wimboot or memdisk as required.
   - Others: dedicated adapter or memdisk fallback.
7. GRUB chainloads/execs; control passes to the guest.

### 6.2 UEFI Boot Flow

1. Firmware loads the platform's EFI application from `/EFI/BOOT/BOOT<ARCH>.EFI` (possibly via shim for Secure Boot).
2. Application parses command line; initializes console and device protocols.
3. **Disk location**: validates candidate disks by size and stored signature/GUID; opens boot partition filesystem.
4. **Driver loading** (optional): loads ISO9660/UDF EFI filesystem drivers if configured.
5. **Image selection** (if not already encoded in command line): the application may present a menu or consume a preselected image index from environment variables.
6. **Virtual block device installation**: builds chunk maps for the image and installs a BlockIo (and optionally SimpleFileSystem) protocol instance representing the image.
7. **Environment persistence**: saves image location, chunk counts, and ramdisk parameters into UEFI variables for the guest/runtime.
8. **Boot**: searches the virtual device for an EFI bootloader (`BOOT<ARCH>.EFI`, GRUB, etc.) and invokes it via `LoadImage`/`StartImage`.
9. **Cleanup**: on return, restores environment and exits.

**Memdisk mode**: if the image is small or forced, the entire image is read into RAM and exposed as a RAM disk; the ramdisk parameter block records its location/size.

**Fallback**: if the primary bootloader fails, the application can retry with fallback flags (e.g., disable virtual-device optimizations, use alternate sector size).

### 6.3 Runtime Linux Integration Flow

1. Linux kernel boots with `rdinit` pointing at the injected runtime `init` (or `init=/runtime/...`).
2. **Dispatch**: `init` chooses `init_chain` (normal) or `init_loop` (chain mode).
3. **Chain mode**:
   - Parse the original initramfs from the kernel command line.
   - Extract nested cpio layers (handling gzip/bzip2/xz/lzma/lz4/zstd), applying the platform's injection archive at each layer.
   - Install runtime tools and libraries into the extracted tree.
   - Exec the platform's chain script.
4. **Chain script**:
   - Detect OS type (from kernel version, `/etc/os-release`, filesystem).
   - Run OS-specific hook (`hook/<os>/os-hook.sh`) and live-injection hook.
   - Set up persistence (if configured): locate backend image, map it.
   - Set up driver-update disks (if configured).
   - Exec the real init, switching root if a ramdisk root is in use.
5. The guest OS now runs with the image mounted via FUSE/device-mapper as needed.

---

## 7. Image Mounting and I/O Redirection

### 7.1 Chunk Map Model

Because image files may be fragmented on the data partition's filesystem, the platform never assumes contiguity. Instead it builds a **chunk map**:

- `chain_head`: image file location (partition, starting cluster/sector), total size, chunk count, flags.
- `img_chunk[]`: maps each contiguous run of the image to a (partition, start sector, length) on disk.
- `override_chunk[]`: sectors that must be read from memory or alternate locations (e.g., modified boot sectors, in-memory patches).
- `virt_chunk[]`: virtual sectors synthesized at runtime (e.g., for ISO9660 path-table fixes).
- `ramdisk_param`: if memdisk mode, records the RAM location and size of the loaded image.

Reads to the virtual device are translated: image LBA → chunk → disk LBA, with overrides and virtual sectors handled inline.

### 7.2 UEFI Virtual Block I/O

- The UEFI application installs a `BlockIo` (and optionally `SimpleFileSystem`) protocol instance whose `ReadBlocks` translates image LBAs to disk LBAs using the chunk map.
- Sector size is configurable (512/2048/4096) with conversion in both directions.
- Write operations are rejected (read-only media semantics), except where a writable overlay is explicitly supported.

### 7.3 Linux Runtime Mounting

- **FUSE ISO mount**: a userspace FUSE daemon presents the image as an ISO9660/UDF filesystem; used when the guest needs file-level access.
- **Device-mapper**: builds linear/striped targets so the kernel exposes the image as `/dev/mapper/...`.
- **Loop device**: for images that are contiguous or after defragmentation, a loop device is used.

### 7.4 Windows PE Mounting

- A helper executable runs in Windows PE, uses a RAM-disk driver (imdisk) to mount the image as a virtual CD-ROM, enabling Windows setup to proceed.

### 7.5 Network (AoE)

- An AoE target (vblade) can export the image over Ethernet for diskless/netboot clients; a client-side initiator consumes it.

---

## 8. Plugin and Configuration Framework

### 8.1 Configuration File

- Location: `<data-partition>/platform/config.json` (UTF-8, JSON).
- Optional companion files: custom GRUB script, injection cpio, theme directory, persistence backends.

### 8.2 Configuration Model

The JSON is parsed into a typed in-memory model with sections:

| Section | Purpose |
|---|---|
| `control` | default menu mode, auto-boot delay, timeout, scroll speed, display options |
| `theme` | colors, fonts, background, layout, resolution |
| `menu_alias` | rename menu entries |
| `menu_tip` | tooltip text per entry |
| `menu_class` | icon/class assignment |
| `menu_extra` | inject arbitrary GRUB entries |
| `image_list` | whitelist/blacklist of image paths (glob) |
| `auto_install` | paths to Windows unattend / Linux preseed/kickstart files, with variable expansion |
| `injection` | files to inject into runtime environment |
| `persistence` | backend image files for Linux persistence |
| `conf_replace` | replace boot config files inside images |
| `dud` | driver-update disks for Linux |
| `password` | global and per-image passwords |
| `vhdboot` | VHD/VHDX boot settings |
| `vdisk_boot` | Linux vDisk boot settings |
| `memdisk` | force memdisk mode for named images |

### 8.3 Plugin Engine

- Plugins are registered with a name, a check function (should this plugin handle the current image/config?), and an entry function (apply behavior).
- At boot, for each discovered image, the engine evaluates applicable plugins and invokes them in priority order.
- Plugins can alter menu presentation, boot parameters, injection, and persistence.

### 8.4 Configurator (Web UI)

- A local HTTP server (started on the target platform) plus a web frontend provides forms for editing `config.json`.
- It uses a FAT read/write library to modify the data partition's filesystem from the host, including from Windows where no native ext/ntfs write support is assumed.
- Changes are validated before write; backups are recommended.

---

## 9. Filesystem and Format Support

### 9.1 Data Partition Filesystems

FAT32, exFAT, NTFS, UDF, XFS, Btrfs, Ext2/3/4. Selection is made at install time; the platform's filesystem driver (GRUB + UEFI app + runtime tools) must support the chosen filesystem for both read (boot) and, where applicable, write (config editing).

### 9.2 Boot Partition Filesystem

FAT16/FAT32 only (firmware requirement).

### 9.3 Image-Internal Filesystems

ISO9660 and UDF, via EFI filesystem drivers (UEFI path) and FUSE/runtime tools (Linux path).

### 9.4 Compression and Archive Formats

gzip, bzip2, xz, lzma, lz4, zstd, lzip; squashfs; cpio (possibly nested/compressed). Used for initramfs layers, runtime archives, and image compression.

### 9.5 Image Formats

ISO, WIM, IMG (raw disk), VHD, VHDX, EFI (executable). Windows-specific: WIM via wimboot, VHD via native loader, auto-install via answer files.

---

## 10. Multi-Architecture Support

| Architecture | Firmware | Bootloader | Runtime |
|---|---|---|---|
| x86 Legacy BIOS | BIOS | GRUB i386-pc | x86 busybox |
| IA32 UEFI | UEFI 32-bit | UEFI app + GRUB ia32 | x86 busybox |
| x86_64 UEFI | UEFI 64-bit | UEFI app + GRUB x64 | x86_64 busybox |
| AArch64 UEFI | UEFI 64-bit ARM | UEFI app + GRUB aa64 | aarch64 busybox |
| MIPS64EL UEFI | UEFI 64-bit MIPS | UEFI app + GRUB mips64el | mips64el busybox |

Toolchains and binaries are built per architecture; cpio archives, tools, and EFI drivers are shipped per-arch. Secure Boot is supported on IA32/x86_64 via shim.

---

## 11. Security Design

- **Secure Boot**: shim verifies the platform's EFI application and GRUB using enrolled keys; MokManager allows user enrollment. The platform can toggle the SecureBoot variable to accommodate distributions.
- **Read-only image semantics**: images are never modified; writes to the virtual device are rejected.
- **Password protection**: global and per-image passwords gate menu access and boot.
- **Integrity**: build artifacts are accompanied by SHA-256 hashes; binaries are verified against documented hashes.
- **Privilege**: installers require administrator/root; runtime injection runs in the guest's init context.

---

## 12. Build and Packaging (Generic)

- The platform is assembled from many components, each built by its own build script; an all-in-one script orchestrates a full build.
- Components are grouped by architecture; third-party tools (busybox, squashfs, compression libs, EFI drivers) are either built from source or fetched as verified binaries.
- The final installable image is a directory tree (`INSTALL/`) plus per-OS installer executables; a packaging script produces the release archive.
- Reproducibility: build environment is documented (host OS, packages, toolchains, download URLs, hashes).

---

## 13. Key Data Structures and Interfaces

### 13.1 UEFI-Side Structures

- `os_param`: persisted UEFI variable carrying image location, chunk counts, platform id, and flags between the application and the guest.
- `chain_head`: image descriptor (partition, start sector, size, chunk count, flags).
- `img_chunk`: disk extent describing a contiguous run of the image.
- `override_chunk`: in-memory/alternate sector overrides.
- `virt_chunk`: synthesized virtual sectors.
- `block_data`: per-virtual-device state (BlockIo media, chunk arrays, sector-size converters).
- `ramdisk_param`: location/size of an image loaded into RAM.

### 13.2 GRUB-Side Structures

- `CONFIG_MODEL`: parsed configuration model.
- `plugin_entry`: name + check function + entry function.
- Partition info structures: start sector, sector count, filesystem type, label.
- Image metadata: path, size, fragmentation list, OS type hint.

### 13.3 Runtime Structures

- `get_os_type` result: OS family enum driving hook selection.
- Initramfs layer descriptor: compression type, offset, size.
- Persistence map: backend file → mount point.

### 13.4 Interfaces

- **Firmware → Bootloader**: MBR/ESP entry points; UEFI protocols (BlockIo, SimpleFileSystem, LoadedImage).
- **Bootloader → Firmware**: virtual BlockIo/SimpleFileSystem installation; `LoadImage`/`StartImage`.
- **Bootloader → Guest**: kernel command line, initrd injection, UEFI variables.
- **Runtime → Guest**: exec of real init; mount points; environment variables.
- **Configurator → Storage**: FAT read/write library; JSON read/write.

---

## 14. Error Handling and Recovery

- **Install validation**: checks target device size, existing partitions, and refuses dangerous operations; supports dry-run/force flags.
- **Upgrade detection**: recognizes existing installations and upgrades in place without data loss.
- **Boot fallback**: UEFI app retries with fallback flags; memdisk mode bypasses fragmentation issues.
- **Debug mode**: `debug` command-line flag enables verbose logging to console/file.
- **Graceful degradation**: if an image cannot be booted, control returns to the menu rather than halting; unsupported images fall back to memdisk or are skipped.
- **Runtime safety**: injection failures log and continue where possible; missing hooks do not abort boot.

---

## 15. Performance Considerations

- **Zero-copy boot**: images are not extracted; only chunk maps are built (O(number of extents), not O(image size)).
- **Memdisk for small images**: avoids runtime translation overhead for tiny images.
- **Lazy decompression**: initramfs layers are decompressed only as needed.
- **Caching**: filesystem metadata and image lists may be cached across menu navigations.
- **I/O alignment**: sector-size conversion handles 512/2048/4096 efficiently.

---

## 16. Extensibility

- **New OS support**: add a GRUB OS adapter + a runtime hook directory + (if needed) a firmware path.
- **New filesystem**: implement read support in GRUB, UEFI app, and runtime tools; add to installer format options.
- **New architecture**: add toolchain, EFI app target, busybox, and cpio archive; replicate build steps.
- **New plugin**: register in the plugin engine with check/entry functions; extend JSON schema.
- **New image format**: extend discovery filter and add an adapter.

---

## 17. Glossary

- **Image**: an OS installation/distribution artifact (ISO/WIM/IMG/VHD/VHDX/EFI).
- **Boot partition**: the small FAT partition holding bootloader and runtime payloads.
- **Data partition**: the large partition holding images and configuration.
- **Chunk map**: the translation table from image sectors to disk sectors.
- **Virtual block device**: a firmware-level block I/O instance backed by an image file.
- **Runtime injection**: the process of inserting the platform's tools into a guest Linux initramfs.
- **Persistence**: a writable backend attached to a live Linux session.
- **Memdisk mode**: loading an entire image into RAM and booting from RAM.
- **Plugin**: a registered handler that customizes behavior per image/config.

---

*End of design document.*
