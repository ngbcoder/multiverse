# M6 Design: UEFI Virtual Block Device + ISO 9660 File System Driver

## Overview

This document describes how to implement Ventoy-style guest OS boot for Multiverse
using a GNU-EFI based ISO 9660 file system driver and a virtual block device.
It assumes a GNU-EFI/EDK2 toolchain is available (which is not the case in the
current development environment).

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    UEFI Firmware                            │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ Boot Mgr │→│ FS Driver    │→│ Virtual Block Device  │  │
│  │ (loads    │  │ (ISO9660)   │  │ (BlockIO protocol)   │  │
│  │ EFI app)  │  │ SimpleFS    │  │ serves ISO sectors   │  │
│  └──────────┘  └──────────────┘  └──────────────────────┘  │
│                                                 ↑            │
│                                    ChunkMap (image→disk LBA) │
│                                                 ↑            │
│                                    Physical disk (FAT volume)│
└─────────────────────────────────────────────────────────────┘
```

## Components

### 1. Virtual Block Device (BlockIO Protocol)

**File:** `crates/firmware/src/virtual_block.rs` (host) + new UEFI shim

**GNU-EFI implementation:**
- Define `EFI_BLOCK_IO_PROTOCOL` with function pointers:
  - `Reset` → no-op (return EFI_SUCCESS)
  - `ReadBlocks` → translate LBA via ChunkMap, read from physical disk
  - `WriteBlocks` → return EFI_WRITE_PROTECTED (read-only)
  - `FlushBlocks` → no-op
- Define `EFI_BLOCK_IO_MEDIA`:
  - `MediaId = 1`
  - `RemovableMedia = TRUE`
  - `MediaPresent = TRUE`
  - `LogicalPartition = FALSE`
  - `ReadOnly = TRUE`
  - `BlockSize = 2048` (ISO sector size) or 512
  - `LastBlock = (iso_size / block_size) - 1`
- Install protocol on a new handle via `gBS->InstallProtocolInterface`

**Sector read path:**
```
ReadBlocks(media_id, lba, buffer_size, buffer):
  1. Validate media_id matches
  2. Translate image LBA → disk LBA via ChunkMap.translate(lba, count)
  3. For each translated extent:
     - ReadSource::Disk: read from physical disk BlockIO at disk_lba
     - ReadSource::Override/Virtual/Ramdisk: copy from in-memory data
  4. Return EFI_SUCCESS
```

**Physical disk access:**
- Open the underlying disk's `EFI_BLOCK_IO_PROTOCOL` via `gBS->OpenProtocol`
- Use its `ReadBlocks` to read sectors at translated disk LBAs

### 2. ISO 9660 File System Driver

**Reference:** `refind-src-0.14.2/filesystems/fsw_iso9660.c`

**Approach:** Port or wrap the rEFInd `fsw` (file system wrapper) framework:
- `fsw_core.c` provides generic FS operations (open, read, close, readdir)
- `fsw_iso9660.c` implements ISO 9660 specific logic
- `fsw_efi.c` adapts fsw to UEFI Simple File System protocol

**Driver entry point (`Iso9660DriverEntry`):**
1. Register driver binding protocol (`EFI_DRIVER_BINDING_PROTOCOL`)
2. In `Supported()`: check if device has BlockIO protocol with ISO 9660 signature
   (read sector 16 (PVD), check for "CD001" at offset 1)
3. In `Start()`:
   - Open BlockIO protocol on the device
   - Allocate private data (FS volume state)
   - Install `EFI_SIMPLE_FILE_SYSTEM_PROTOCOL` on the device handle
4. In `Stop()`:
   - Uninstall SimpleFileSystem protocol
   - Close BlockIO protocol

**Simple File System implementation:**
- `OpenVolume` → return root directory handle
- `Open` → traverse ISO directory tree, return file handle
- `Read` → read file data from ISO sectors
- `Close` → release handle
- `GetInfo` → return file/directory info
- `SetPosition` / `GetPosition` → seek within file

**ISO 9660 specifics:**
- Primary Volume Descriptor at sector 16 (LBA 16)
- Root directory record at PVD offset 156
- Directory records: 33-byte (or 34-byte with XA) entries
- File data at specified LBA, length in bytes
- Rock Ridge extensions (optional): parse SUSP/RRIP for long names
- Joliet extensions (optional): UCS-2 filenames in supplementary VD

### 3. Device Path Construction

For the virtual block device to be bootable, it needs a device path:
- Use a **Vendor-Defined Device Path** node with a unique GUID for Multiverse
- Or use a **Memory-Mapped Device Path** (subtype 3) pointing to a RAM buffer
  (only works if ISO fits in RAM — not for large ISOs)

**Recommended:** Vendor-defined device path node:
```
Hardware Device Path (VENDOR) → Media Device Path (VENDOR) → End
```
The firmware boot manager will match this to the installed BlockIO + SimpleFileSystem protocols.

### 4. Boot Flow Integration

**In the UEFI application (`uefi-test/src/main.rs` or real firmware):**

```c
// 1. Discover ISOs on the FAT volume
iso_list = discover_isos(fat_volume);

// 2. For selected ISO:
//    a. Build ChunkMap (image LBA → disk LBA)
chunk_map = build_chunk_map(iso_file_extents);

//    b. Create virtual block device
vdev = VirtualBlockDevice::new(chunk_map);
vdev = vdev.with_disk_reader(physical_disk_read_fn);

//    c. Install BlockIO protocol
handle = install_block_io_protocol(vdev);

//    d. Install ISO9660 driver binding (or use already-installed driver)
//    The driver's Supported() will detect the ISO signature

//    e. Trigger boot manager to boot from the virtual device
//    gBS->LoadImage(handle, device_path, &image_handle)
//    gBS->StartImage(image_handle)
```

### 5. GNU-EFI Build Setup

**Makefile (Make.gnuefi style):**
```makefile
ARCH ?= x86_64
CC = x86_64-w64-mingw32-gcc
CFLAGS = -I/usr/include/efi -I/usr/include/efi/$(ARCH) \
         -fno-stack-protector -fpic -fshort-wchar -mno-red-zone \
         -DEFI_FUNCTION_WRAPPER -DGNU_EFI_USE_MS_ABI
LDFLAGS = -T /usr/lib/elf_$(ARCH)_efi.lds -shared -Bsymbolic \
          -L/usr/lib -lefi -lgnuefi

# Build ISO9660 driver
iso9660.efi: fsw_iso9660.o fsw_core.o fsw_efi.o ...
    $(LD) $(LDFLAGS) -o $@ $^
```

**Dependencies:**
- `gnu-efi` (lib + headers)
- `rEFInd fsw` source (fsw_iso9660.c, fsw_core.c, fsw_efi.c, fsw_lib.c)
- `multiverse_storage` chunk map logic (ported to C or called via FFI)

### 6. Rust ↔ C Interop

Since Multiverse's chunk map is in Rust, two options:

**Option A: Port chunk map to C**
- Port `crates/core/src/chunkmap.rs` and `crates/storage/src/extent.rs` to C
- Simpler interop, larger porting effort

**Option B: Rust static library + C driver**
- Compile Multiverse core as `libmultiverse.a` (static lib)
- C driver calls Rust functions via FFI for chunk translation
- Requires `#![no_std]` + `extern "C"` exports in Rust
- More complex build, but reuses tested Rust code

### 7. Cleanup on Return

When the guest bootloader returns (or fails):
1. Uninstall SimpleFileSystem protocol
2. Uninstall BlockIO protocol
3. Free virtual device handle
4. Free chunk map memory
5. Return to Multiverse menu

### 8. Testing Strategy

1. **Unit tests:** Chunk map translation (already in `multiverse_core`)
2. **QEMU integration:** Boot multiverse image with `-bios edk2` and verify:
   - Virtual device appears in `map` command
   - `fs0:` shows ISO contents
   - Guest bootloader loads and boots OS
3. **Real hardware:** Test on UEFI-capable machines with various ISOs

### 9. Risk Assessment

| Risk | Mitigation |
|------|-----------|
| ISO9660 driver doesn't match firmware expectations | Test with multiple firmware (OVMF, real hardware) |
| Virtual device not bootable by boot manager | Use vendor-defined device path; test with `bcfg` |
| Large ISO memory usage | Use chunk mapping (disk-backed), not full RAM load |
| Rock Ridge/Joliet compatibility | Start with basic ISO 9660, add extensions later |
| Secure Boot compatibility | Sign the driver with enrolled keys |

### 10. Implementation Order

1. Port `fsw_iso9660.c` + `fsw_efi.c` to build as standalone `.efi` driver
2. Test: load driver manually, verify `fs0:` shows ISO contents on a real ISO
3. Implement virtual block device (BlockIO protocol) in Rust/C
4. Wire chunk map translation
5. Integrate into Multiverse boot flow
6. Test end-to-end in QEMU
7. Handle cleanup and error recovery

## Files to Create/Modify

| File | Action |
|------|--------|
| `crates/firmware/src/uefi_blockio.rs` | New: UEFI BlockIO protocol implementation |
| `crates/firmware/src/uefi_fs_driver.rs` | New: ISO9660 driver binding |
| `filesystems/fsw_iso9660.c` | Port from rEFInd |
| `filesystems/fsw_efi.c` | Port from rEFInd |
| `Makefile` (new) | GNU-EFI build for driver |
| `crates/uefi-test/src/main.rs` | Wire virtual device boot flow |
| `todo.md` | Update M6 checkboxes |

## References

- rEFInd filesystems: `C:\Users\29bragann\Downloads\refind-src-0.14.2\`
- Multiverse design: `DESIGN.md` (M6 section)
- UEFI spec: Block I/O Protocol, Simple File System Protocol
- GNU-EFI: https://sourceforge.net/projects/gnu-efi/
}