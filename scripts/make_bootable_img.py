#!/usr/bin/env python3
import os
import subprocess
import sys
import math

def main():
    print("Building Multiverse bootable disk image (multiverse_disk.img)...")
    img_path = "multiverse_disk.img"

    # Dynamically calculate required disk size based on ISOs in 'iso/' directory
    size_mb = 128
    iso_dir = "iso"
    discovered_iso_names = []
    if os.path.exists(iso_dir):
        total_iso_bytes = 0
        for file in os.listdir(iso_dir):
            if file.lower().endswith(".iso"):
                fpath = os.path.join(iso_dir, file)
                fsize = os.path.getsize(fpath)
                if fsize >= 4 * 1024 * 1024 * 1024:
                    print(f"Note: {file} is {fsize // (1024*1024)} MB (>= 4GB FAT32 limit). Skipping direct single-file storage.")
                    continue
                total_iso_bytes += fsize
                discovered_iso_names.append(file)
                print(f"Found ISO: {file} ({fsize // (1024*1024)} MB)")
        if total_iso_bytes > 0:
            size_mb = max(128, (total_iso_bytes // (1024 * 1024)) + 400)

    print(f"Allocating disk image size: {size_mb} MB")

    # Generate discovered_isos.rs for uefi-test
    rust_iso_code = "pub const DISCOVERED_ISOS: &[&str] = &[\n"
    if discovered_iso_names:
        for name in discovered_iso_names:
            rust_iso_code += f'    "{name}",\n'
    else:
        rust_iso_code += '    "Ubuntu 22.04 LTS (Default)",\n    "Debian 12 Bookworm (Default)",\n'
    rust_iso_code += "];\n"
    
    os.makedirs("crates/uefi-test/src", exist_ok=True)
    with open("crates/uefi-test/src/discovered_isos.rs", "w") as rfile:
        rfile.write(rust_iso_code)
    print("Generated crates/uefi-test/src/discovered_isos.rs with user's actual ISOs.")

    # 1. Create blank image file
    with open(img_path, "wb") as f:
        f.write(b"\x00" * (size_mb * 1024 * 1024))

    # 2. Format as FAT32 using mkfs.fat
    mkfs_path = "C:/msys64/usr/bin/mkfs.fat.exe"
    if not os.path.exists(mkfs_path):
        mkfs_path = "mkfs.fat"

    res = subprocess.run([mkfs_path, "-F", "32", "-I", img_path], capture_output=True, text=True)
    if res.returncode != 0:
        print("Error running mkfs.fat:", res.stderr)
        sys.exit(1)
    print("Formatted disk image as FAT32 successfully.")

    # 3. Read image and populate files via pure Python FAT32 parser/writer
    with open(img_path, "rb+") as f:
        buf = bytearray(f.read())

        # Parse BPB
        bps = int.from_bytes(buf[11:13], "little")
        spc = buf[13]
        reserved = int.from_bytes(buf[14:16], "little")
        num_fats = buf[16]
        fat_size32 = int.from_bytes(buf[36:40], "little")
        root_cluster = int.from_bytes(buf[44:48], "little")

        fat_offset = reserved * bps
        cluster_size = spc * bps
        data_offset = fat_offset + (num_fats * fat_size32 * bps)

        def get_fat(cluster):
            fat_idx = fat_offset + cluster * 4
            return int.from_bytes(buf[fat_idx:fat_idx+4], "little") & 0x0FFFFFFF

        def set_fat(cluster, val):
            fat_idx = fat_offset + cluster * 4
            existing = int.from_bytes(buf[fat_idx:fat_idx+4], "little") & 0xF0000000
            new_val = existing | (val & 0x0FFFFFFF)
            buf[fat_idx:fat_idx+4] = new_val.to_bytes(4, "little")

        def allocate_clusters(num_clusters):
            if num_clusters == 0:
                return 0
            start_cluster = None
            prev_cluster = None
            total_fat_entries = (fat_size32 * bps) // 4

            for c in range(2, total_fat_entries):
                if get_fat(c) == 0:
                    if start_cluster is None:
                        start_cluster = c
                    if prev_cluster is not None:
                        set_fat(prev_cluster, c)
                    prev_cluster = c
                    num_clusters -= 1
                    if num_clusters == 0:
                        set_fat(prev_cluster, 0x0FFFFFFF)
                        return start_cluster
            raise Exception("Disk full: cannot allocate clusters")

        def write_cluster_data(cluster, data):
            sector_offset = data_offset + (cluster - 2) * cluster_size
            padded = data + b"\x00" * (cluster_size - len(data))
            buf[sector_offset:sector_offset+cluster_size] = padded

        def cluster_to_offset(cluster):
            return data_offset + (cluster - 2) * cluster_size

        def make_dir_entry(filename, ext, attr, start_cluster, file_size):
            name_padded = filename.ljust(8)[:8].encode("ascii", errors="ignore")
            ext_padded = ext.ljust(3)[:3].encode("ascii", errors="ignore")
            entry = bytearray(32)
            entry[0:8] = name_padded
            entry[8:11] = ext_padded
            entry[11] = attr
            entry[20:22] = (start_cluster >> 16).to_bytes(2, "little")
            entry[26:28] = (start_cluster & 0xFFFF).to_bytes(2, "little")
            entry[28:32] = file_size.to_bytes(4, "little")
            return bytes(entry)

        def add_file_to_dir(dir_cluster, filename, ext, attr, file_data):
            num_clusters = (len(file_data) + cluster_size - 1) // cluster_size if len(file_data) > 0 else 0
            start_cluster = allocate_clusters(num_clusters)
            if num_clusters > 0:
                write_cluster_data(start_cluster, file_data)

            entry = make_dir_entry(filename, ext, attr, start_cluster, len(file_data))

            curr_cluster = dir_cluster
            while True:
                cl_offset = cluster_to_offset(curr_cluster)
                for slot in range(cluster_size // 32):
                    entry_offset = cl_offset + slot * 32
                    first_byte = buf[entry_offset]
                    if first_byte == 0x00 or first_byte == 0xE5:
                        buf[entry_offset:entry_offset+32] = entry
                        return start_cluster

                next_cl = get_fat(curr_cluster)
                if next_cl >= 0x0FFFFFF8:
                    new_cl = allocate_clusters(1)
                    set_fat(curr_cluster, new_cl)
                    set_fat(new_cl, 0x0FFFFFFF)
                    buf[cluster_to_offset(new_cl):cluster_to_offset(new_cl)+cluster_size] = b"\x00" * cluster_size
                    curr_cluster = new_cl
                else:
                    curr_cluster = next_cl

        def get_or_create_dir(parent_cluster, dirname):
            name_padded = dirname.ljust(8)[:8].encode("ascii", errors="ignore")
            curr_cluster = parent_cluster
            while True:
                cl_offset = cluster_to_offset(curr_cluster)
                for slot in range(cluster_size // 32):
                    entry_offset = cl_offset + slot * 32
                    first_byte = buf[entry_offset]
                    if first_byte == 0x00:
                        break
                    if first_byte != 0xE5 and (buf[entry_offset+11] & 0x10):
                        ent_name = buf[entry_offset:entry_offset+8]
                        if ent_name == name_padded:
                            hi = int.from_bytes(buf[entry_offset+20:entry_offset+22], "little")
                            lo = int.from_bytes(buf[entry_offset+26:entry_offset+28], "little")
                            return (hi << 16) | lo

                next_cl = get_fat(curr_cluster)
                if next_cl >= 0x0FFFFFF8:
                    break
                curr_cluster = next_cl

            new_cl = allocate_clusters(1)
            buf[cluster_to_offset(new_cl):cluster_to_offset(new_cl)+cluster_size] = b"\x00" * cluster_size
            
            dot = make_dir_entry(".", "   ", 0x10, new_cl, 0)
            dotdot = make_dir_entry("..", "   ", 0x10, parent_cluster, 0)
            buf[cluster_to_offset(new_cl):cluster_to_offset(new_cl)+32] = dot
            buf[cluster_to_offset(new_cl)+32:cluster_to_offset(new_cl)+64] = dotdot

            entry = make_dir_entry(dirname, "   ", 0x10, new_cl, 0)

            curr_cluster = parent_cluster
            while True:
                cl_offset = cluster_to_offset(curr_cluster)
                for slot in range(cluster_size // 32):
                    entry_offset = cl_offset + slot * 32
                    first_byte = buf[entry_offset]
                    if first_byte == 0x00 or first_byte == 0xE5:
                        buf[entry_offset:entry_offset+32] = entry
                        return new_cl
                next_cl = get_fat(curr_cluster)
                if next_cl >= 0x0FFFFFF8:
                    alloc_cl = allocate_clusters(1)
                    set_fat(curr_cluster, alloc_cl)
                    set_fat(alloc_cl, 0x0FFFFFFF)
                    buf[cluster_to_offset(alloc_cl):cluster_to_offset(alloc_cl)+cluster_size] = b"\x00" * cluster_size
                    curr_cluster = alloc_cl
                else:
                    curr_cluster = next_cl

        # 1. Read config.json
        config_path = "INSTALL/config/config.json"
        config_data = b"{\"control\":{\"timeout\":30}}"
        if os.path.exists(config_path):
            with open(config_path, "rb") as cf:
                config_data = cf.read()
        add_file_to_dir(root_cluster, "CONFIG", "JSN", 0x20, config_data)
        print(f"Added /CONFIG.JSON ({len(config_data)} bytes)")

        # 2. Add /BOOT/README.TXT
        boot_dir = get_or_create_dir(root_cluster, "BOOT")
        readme_data = b"Multiverse Multi-Image Bootable Platform Test Image\n"
        add_file_to_dir(boot_dir, "README", "TXT", 0x20, readme_data)
        print(f"Added /BOOT/README.TXT ({len(readme_data)} bytes)")

        # 3. Add /EFI/BOOT/BOOTX64.EFI
        efi_dir = get_or_create_dir(root_cluster, "EFI")
        efi_boot_dir = get_or_create_dir(efi_dir, "BOOT")
        
        efi_src = "INSTALL/efi/boot/bootx64.efi"
        efi_data = b"\x90\x90\x90\x90"
        if os.path.exists(efi_src):
            with open(efi_src, "rb") as ef:
                efi_data = ef.read()
        add_file_to_dir(efi_boot_dir, "BOOTX64", "EFI", 0x20, efi_data)
        print(f"Added /EFI/BOOT/BOOTX64.EFI ({len(efi_data)} bytes)")

        # 4. Add user ISOs from 'iso/' directory or fallbacks
        images_dir = get_or_create_dir(root_cluster, "IMAGES")
        added_isos = False
        if os.path.exists(iso_dir):
            for file in os.listdir(iso_dir):
                if file.lower().endswith(".iso"):
                    fpath = os.path.join(iso_dir, file)
                    if not os.path.exists(fpath):
                        continue
                    fsize = os.path.getsize(fpath)
                    if fsize >= 4 * 1024 * 1024 * 1024:
                        continue
                    with open(fpath, "rb") as isof:
                        iso_bytes = isof.read()
                    base_name = os.path.splitext(file)[0].upper()
                    add_file_to_dir(images_dir, base_name[:8], "ISO", 0x20, iso_bytes)
                    print(f"Added custom ISO to /IMAGES/{base_name[:8]}.ISO ({len(iso_bytes)} bytes)")
                    added_isos = True

        if not added_isos:
            add_file_to_dir(images_dir, "UBUNTU", "ISO", 0x20, b"UBUNTU 22.04 ISO BLOB")
            add_file_to_dir(images_dir, "DEBIAN", "ISO", 0x20, b"DEBIAN 12 ISO BLOB")
            print("Added default sample ISOs under /IMAGES/")

        # 5. Add STARTUP.NSH to auto-boot in UEFI shell
        startup_script = b"fs0:\r\n\\EFI\\BOOT\\BOOTX64.EFI\r\n"
        add_file_to_dir(root_cluster, "STARTUP", "NSH", 0x20, startup_script)
        print(f"Added /STARTUP.NSH ({len(startup_script)} bytes)")

        f.seek(0)
        f.write(buf)

    print(f"Successfully created bootable disk image: {img_path} ({size_mb}MB)")

if __name__ == "__main__":
    main()
