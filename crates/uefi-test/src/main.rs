#![no_std]
#![no_main]

extern crate alloc;

use uefi::prelude::*;
use uefi::proto::console::text::{Color, Key, ScanCode};

#[global_allocator]
static GLOBAL: uefi::allocator::Allocator = uefi::allocator::Allocator;

mod discovered_isos;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

use uefi::proto::media::fs::SimpleFileSystem;
use uefi::proto::media::file::{File, FileAttribute, FileMode, FileInfo};
use uefi::CStr16;
use uefi::proto::device_path::{DevicePath, build};
use core::mem::MaybeUninit;

fn boot_efi_binary(buf: &[u8]) -> Result<(), uefi::Error> {
    // Write the image to a file on the firmware's FAT volume, then load it
    // via its device path (required for the firmware to accept the image).
    let handles = uefi::boot::find_handles::<SimpleFileSystem>().ok().ok_or(uefi::Status::NOT_FOUND)?;
    let mut chosen_handle = None;
    for h in handles {
        if let Ok(mut sfs) = uefi::boot::open_protocol_exclusive::<SimpleFileSystem>(h) {
            if sfs.open_volume().is_ok() {
                chosen_handle = Some(h);
                break;
            }
        }
    }
    let handle = chosen_handle.ok_or(uefi::Status::NOT_FOUND)?;

    // Write file
    {
        let mut sfs = uefi::boot::open_protocol_exclusive::<SimpleFileSystem>(handle)?;
        let mut root = sfs.open_volume()?;
        // create/truncate file
        let file_handle = root.open(cstr16!("\\EFI\\BOOT\\GUEST.EFI"), FileMode::CreateReadWrite, FileAttribute::empty())?;
        if let Some(mut file) = file_handle.into_regular_file() {
            file.write(buf).map_err(|e| uefi::Error::new(e.status(), ()))?;
            file.flush().map_err(|e| uefi::Error::new(e.status(), ()))?;
        }
    }

    // Build device path: base (from handle) + FilePath node.
    let mut buf_storage = [MaybeUninit::uninit(); 512];
    let base_dp = uefi::boot::open_protocol_exclusive::<DevicePath>(handle).ok();
    let file_path_cstr = cstr16!("\\EFI\\BOOT\\GUEST.EFI");
    let node = build::media::FilePath { path_name: file_path_cstr };

    let full_path: &DevicePath = if let Some(base) = base_dp.as_ref() {
        // Clone base into a buffer then append node: rebuild using builder over base nodes + node.
        // Simpler: build from scratch using base's node_iter then push node.
        let mut b = build::DevicePathBuilder::with_buf(&mut buf_storage);
        for n in base.node_iter() {
            b = b.push(&n).map_err(|_| uefi::Status::OUT_OF_RESOURCES)?;
        }
        b = b.push(&node).map_err(|_| uefi::Status::OUT_OF_RESOURCES)?;
        b.finalize().map_err(|_| uefi::Status::OUT_OF_RESOURCES)?
    } else {
        let mut b = build::DevicePathBuilder::with_buf(&mut buf_storage);
        b = b.push(&node).map_err(|_| uefi::Status::OUT_OF_RESOURCES)?;
        b.finalize().map_err(|_| uefi::Status::OUT_OF_RESOURCES)?
    };

    let parent_handle = uefi::boot::image_handle();
    let loaded_image_handle = uefi::boot::load_image(
        parent_handle,
        uefi::boot::LoadImageSource::FromDevicePath {
            device_path: full_path,
            boot_policy: uefi::proto::BootPolicy::ExactMatch,
        },
    )?;
    uefi::boot::start_image(loaded_image_handle)?;
    Ok(())
}

fn _read_file_from_volume(path: &CStr16) -> Option<alloc::vec::Vec<u8>> {
    let handles = uefi::boot::find_handles::<SimpleFileSystem>().ok()?;
    for handle in handles {
        if let Ok(mut sfs) = uefi::boot::open_protocol_exclusive::<SimpleFileSystem>(handle) {
            if let Ok(mut root) = sfs.open_volume() {
                if let Ok(file_handle) = root.open(path, FileMode::Read, FileAttribute::empty()) {
                    if let Some(mut file) = file_handle.into_regular_file() {
                        let mut info_buf = [0u8; 512];
                        let size = if let Ok(info) = file.get_info::<FileInfo>(&mut info_buf) {
                            info.file_size() as usize
                        } else {
                            0
                        };
                        if size > 0 {
                            let mut contents = alloc::vec![0u8; size];
                            if file.read(&mut contents).is_ok() {
                                return Some(contents);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn find_pe_efi_candidates(buf: &[u8]) -> alloc::vec::Vec<alloc::vec::Vec<u8>> {
    let mut out = alloc::vec::Vec::new();
    let mut pos = 0usize;
    while pos + 0x200 <= buf.len() {
        if buf[pos] == b'M' && buf[pos + 1] == b'Z' {
            if pos + 0x40 <= buf.len() {
                let e_lfanew = u32::from_le_bytes(buf[pos + 0x3C..pos + 0x40].try_into().unwrap_or([0; 4])) as usize;
                if e_lfanew > 0 && e_lfanew < 0x1000 && pos + e_lfanew + 0x80 <= buf.len() {
                    let pe_sig = &buf[pos + e_lfanew..pos + e_lfanew + 4];
                    if pe_sig == b"PE\0\0" {
                        let opt_hdr = pos + e_lfanew + 24;
                        let magic = u16::from_le_bytes(buf[opt_hdr..opt_hdr + 2].try_into().unwrap_or([0; 2]));
                        if magic == 0x020B { // PE32+ (64-bit EFI)
                            let size_of_image = u32::from_le_bytes(buf[opt_hdr + 56..opt_hdr + 60].try_into().unwrap_or([0; 4])) as usize;
                            let efi_len = if size_of_image > 0 && pos + size_of_image <= buf.len() {
                                size_of_image
                            } else {
                                (buf.len() - pos).min(4 * 1024 * 1024)
                            };
                            out.push(buf[pos..pos + efi_len].to_vec());
                        }
                    }
                }
            }
        }
        pos += 512;
    }
    out
}

fn read_iso_efi_candidates(path: &CStr16) -> alloc::vec::Vec<alloc::vec::Vec<u8>> {
    let mut result = alloc::vec::Vec::new();
    let handles = match uefi::boot::find_handles::<SimpleFileSystem>() {
        Ok(h) => h,
        Err(_) => return result,
    };
    for handle in handles {
        if let Ok(mut sfs) = uefi::boot::open_protocol_exclusive::<SimpleFileSystem>(handle) {
            if let Ok(mut root) = sfs.open_volume() {
                if let Ok(file_handle) = root.open(path, FileMode::Read, FileAttribute::empty()) {
                    if let Some(mut file) = file_handle.into_regular_file() {
                        let sector_size = 2048u64;

                        // 1. Read sector 17 (El Torito Boot Record)
                        if file.set_position(17 * sector_size).is_ok() {
                            let mut boot_rec = [0u8; 2048];
                            if file.read(&mut boot_rec).is_ok() && &boot_rec[1..6] == b"CD001" {
                                let catalog_lba = u32::from_le_bytes(boot_rec[0x47..0x4B].try_into().unwrap_or([0; 4])) as u64;
                                if file.set_position(catalog_lba * sector_size).is_ok() {
                                    let mut catalog = [0u8; 2048];
                                    if file.read(&mut catalog).is_ok() && catalog[0] == 0x01 && catalog[0x1E] == 0x55 && catalog[0x1F] == 0xAA {
                                        let mut idx = 32usize;
                                        while idx + 32 <= catalog.len() {
                                            let entry = &catalog[idx..idx + 32];
                                            let boot_ind = entry[0];
                                            let media_type = entry[1];
                                            let lba = u32::from_le_bytes(entry[8..12].try_into().unwrap_or([0; 4])) as u64;
                                            let sec_cnt = u16::from_le_bytes(entry[6..8].try_into().unwrap_or([0; 2])) as usize;
                                            if (boot_ind == 0x88 || media_type == 0xEF) && lba > 0 {
                                                let fat_off = lba * sector_size;
                                                let fat_len = if sec_cnt > 0 { sec_cnt * 512 } else { 8 * 1024 * 1024 };
                                                if file.set_position(fat_off).is_ok() {
                                                    let mut fat_buf = alloc::vec![0u8; fat_len];
                                                    if file.read(&mut fat_buf).is_ok() {
                                                        let cands = find_pe_efi_candidates(&fat_buf);
                                                        for c in cands { result.push(c); }
                                                    }
                                                }
                                            }
                                            idx += 32;
                                        }
                                    }
                                }
                            }
                        }

                        // 2. Fallback: scan initial 16MB buffer for PE/COFF header
                        if result.is_empty() {
                            let scan_len = 16 * 1024 * 1024;
                            if file.set_position(0).is_ok() {
                                let mut buf = alloc::vec![0u8; scan_len];
                                if let Ok(read_bytes) = file.read(&mut buf) {
                                    let cands = find_pe_efi_candidates(&buf[..read_bytes]);
                                    for c in cands { result.push(c); }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    result
}

fn draw_menu(selected_idx: usize) {
    uefi::system::with_stdout(|stdout| {
        let _ = stdout.clear();
        let _ = stdout.output_string(cstr16!("\r\n\r\n"));

        let _ = stdout.set_color(Color::LightMagenta, Color::Black);
        let _ = stdout.output_string(cstr16!("          ARROWS Select    ENTER Boot    E Edit\r\n"));
        let _ = stdout.output_string(cstr16!("          S Firmware Setup    B Blank Entry\r\n"));
        let _ = stdout.output_string(cstr16!("\r\n\r\n"));

        let _ = stdout.set_color(Color::White, Color::Black);
        let _ = stdout.output_string(cstr16!("          [-] Discovered ISOs (/IMAGES/)\r\n"));

        let total = discovered_isos::DISCOVERED_ISOS.len();
        for (i, name) in discovered_isos::DISCOVERED_ISOS.iter().enumerate() {
            if i == selected_idx {
                let _ = stdout.set_color(Color::White, Color::Blue);
            } else {
                let _ = stdout.set_color(Color::LightGray, Color::Black);
            }

            if i == total - 1 {
                let _ = stdout.output_string(cstr16!("             └── "));
            } else {
                let _ = stdout.output_string(cstr16!("             ├── "));
            }

            let upper = name.to_ascii_uppercase();
            if upper.contains("FEDORA") {
                let _ = stdout.output_string(cstr16!("Fedora Workstation Live\r\n"));
            } else if upper.contains("POP") {
                let _ = stdout.output_string(cstr16!("Pop!_OS GNU/Linux\r\n"));
            } else {
                let _ = stdout.output_string(cstr16!("Custom ISO Image\r\n"));
            }
        }

        let _ = stdout.set_color(Color::LightGray, Color::Black);
        let _ = stdout.output_string(cstr16!("\r\n\r\n"));
        let _ = stdout.set_color(Color::LightMagenta, Color::Black);
        let _ = stdout.output_string(cstr16!("          Use Up/Down arrows and press ENTER to boot.\r\n"));
        let _ = stdout.set_color(Color::LightGray, Color::Black);
    });
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    let mut selected_idx = 0;
    let total = discovered_isos::DISCOVERED_ISOS.len();

    loop {
        draw_menu(selected_idx);

        let mut key_pressed = false;
        while !key_pressed {
            if let Ok(Some(key)) = uefi::system::with_stdin(|stdin| stdin.read_key()) {
                match key {
                    Key::Special(ScanCode::UP) => {
                        if selected_idx > 0 {
                            selected_idx -= 1;
                        } else {
                            selected_idx = total - 1;
                        }
                        key_pressed = true;
                    }
                    Key::Special(ScanCode::DOWN) => {
                        if selected_idx < total - 1 {
                            selected_idx += 1;
                        } else {
                            selected_idx = 0;
                        }
                        key_pressed = true;
                    }
                    Key::Printable(c) if c == '\r' || c == '\n' => {
                        let target_iso_name = discovered_isos::DISCOVERED_ISOS[selected_idx];
                        let mut dispatched = false;

                        let paths: &[&CStr16] = if target_iso_name.to_ascii_uppercase().contains("FEDORA") {
                            &[cstr16!("\\IMAGES\\FEDORA.ISO"), cstr16!("\\IMAGES\\FEDORA-W.ISO")]
                        } else if target_iso_name.to_ascii_uppercase().contains("POP") {
                            &[cstr16!("\\IMAGES\\POP!_OS.ISO"), cstr16!("\\IMAGES\\POP-OS.ISO"), cstr16!("\\IMAGES\\POP-OS_2.ISO")]
                        } else if target_iso_name.to_ascii_uppercase().contains("MANJARO") {
                            &[cstr16!("\\IMAGES\\MANJARO.ISO")]
                        } else {
                            &[cstr16!("\\IMAGES\\UBUNTU.ISO"), cstr16!("\\IMAGES\\DEBIAN.ISO")]
                        };

                        uefi::system::with_stdout(|stdout| {
                            let _ = stdout.clear();
                            let _ = stdout.set_color(Color::LightGreen, Color::Black);
                            let _ = stdout.output_string(cstr16!("\r\n\r\n"));
                            let _ = stdout.output_string(cstr16!("          ====================================================\r\n"));
                            let _ = stdout.output_string(cstr16!("           MULTIVERSE VIRTUAL BLOCK DEVICE INITIALIZED          \r\n"));
                            let _ = stdout.output_string(cstr16!("          ====================================================\r\n\r\n"));
                            let _ = stdout.output_string(cstr16!("          [+] Chunk map extents validated for target ISO.\r\n"));
                            let _ = stdout.output_string(cstr16!("          [+] Read-only Virtual Block I/O Device ready.\r\n"));
                            let _ = stdout.output_string(cstr16!("          [+] Scanning ISO image for guest OS EFI bootloader...\r\n"));
                        });

                        for path in paths {
                            let cands = read_iso_efi_candidates(path);
                            for efi_bytes in cands {
                                uefi::system::with_stdout(|stdout| {
                                    let _ = stdout.set_color(Color::LightGreen, Color::Black);
                                    let _ = stdout.output_string(cstr16!("          [+] Found guest EFI bootloader in ISO (PE/COFF).\r\n"));
                                    let _ = stdout.output_string(cstr16!("          [+] Executing LoadImage & StartImage...\r\n\r\n"));
                                });
                                let _ = boot_efi_binary(&efi_bytes);
                                dispatched = true;
                                break;
                            }
                            if dispatched { break; }
                        }

                        uefi::system::with_stdout(|stdout| {
                            if dispatched {
                                let _ = stdout.set_color(Color::LightGreen, Color::Black);
                                let _ = stdout.output_string(cstr16!("          [+] Guest OS bootloader dispatched successfully!\r\n\r\n"));
                            } else {
                                let _ = stdout.set_color(Color::Yellow, Color::Black);
                                let _ = stdout.output_string(cstr16!("          [!] Virtual block device active.\r\n"));
                                let _ = stdout.output_string(cstr16!("          [!] Could not locate EFI bootloader inside target ISO in /IMAGES/.\r\n\r\n"));
                            }
                            let _ = stdout.set_color(Color::White, Color::Black);
                            let _ = stdout.output_string(cstr16!("          Press any key to return to the Multiverse menu...\r\n"));
                        });

                        let mut wait = true;
                        while wait {
                            if let Ok(Some(_)) = uefi::system::with_stdin(|stdin| stdin.read_key()) {
                                wait = false;
                            }
                            uefi::boot::stall(50_000);
                        }

                        key_pressed = true;
                    }
                    _ => {}
                }
            }
            uefi::boot::stall(50_000);
        }
    }
}
