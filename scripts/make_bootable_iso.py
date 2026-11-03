#!/usr/bin/env python3
"""Build a bootable UEFI ISO (El Torito) containing the Multiverse bootloader
and any guest ISOs found in ./iso/."""

import os
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ISO_DIR = os.path.join(ROOT, "iso")
INSTALL_DIR = os.path.join(ROOT, "INSTALL")
UEFI_TARGET = os.path.join(ROOT, "crates", "uefi-test", "target", "x86_64-unknown-uefi", "debug", "uefi-test.efi")
DISCOVERED = os.path.join(ROOT, "crates", "uefi-test", "src", "discovered_isos.rs")
OUTPUT = os.path.join(ROOT, "multiverse.iso")

def run(cmd, **kw):
    print(">", " ".join(cmd))
    return subprocess.run(cmd, **kw)

def main():
    os.chdir(ROOT)

    # 1. Discover guest ISOs
    guest_isos = []
    if os.path.isdir(ISO_DIR):
        for name in sorted(os.listdir(ISO_DIR)):
            if name.lower().endswith(".iso"):
                guest_isos.append(name)
    print(f"Found {len(guest_isos)} guest ISO(s).")

    # 2. Generate discovered_isos.rs
    lines = ["pub const DISCOVERED_ISOS: &[&str] = &["]
    for name in guest_isos:
        lines.append(f'    "{name}",')
    if not guest_isos:
        lines.append('    "Ubuntu 22.04 LTS (Default)",')
        lines.append('    "Debian 12 Bookworm (Default)",')
    lines.append("];\n")
    os.makedirs(os.path.dirname(DISCOVERED), exist_ok=True)
    with open(DISCOVERED, "w") as f:
        f.write("\n".join(lines))
    print("Generated discovered_isos.rs")

    # 3. Build UEFI app (regenerate discovered_isos.rs may have changed)
    print("Building uefi-test...")
    r = run(["cargo", "build", "--manifest-path", os.path.join(ROOT, "crates", "uefi-test", "Cargo.toml"),
             "--target", "x86_64-unknown-uefi"], cwd=ROOT)
    if r.returncode != 0:
        print("UEFI build failed.")
        if not os.path.exists(UEFI_TARGET):
            sys.exit(1)
    if not os.path.exists(UEFI_TARGET):
        print("ERROR: UEFI bootloader binary not found at", UEFI_TARGET)
        sys.exit(1)

    # 4. Prepare ISO root
    iso_root = tempfile.mkdtemp(prefix="multiverse_iso_", dir=ROOT)
    try:
        os.makedirs(os.path.join(iso_root, "EFI", "BOOT"), exist_ok=True)
        os.makedirs(os.path.join(iso_root, "CONFIG"), exist_ok=True)
        os.makedirs(os.path.join(iso_root, "IMAGES"), exist_ok=True)

        shutil.copy2(UEFI_TARGET, os.path.join(iso_root, "EFI", "BOOT", "BOOTX64.EFI"))

        cfg_src = os.path.join(INSTALL_DIR, "config", "config.json")
        if os.path.exists(cfg_src):
            shutil.copy2(cfg_src, os.path.join(iso_root, "CONFIG", "CONFIG.JSON"))
        else:
            with open(os.path.join(iso_root, "CONFIG", "CONFIG.JSON"), "w") as f:
                f.write('{"control":{"timeout":30}}')

        # EFI system partition image for El Torito (FAT16, 16MB)
        boot_img_name = "efi_fat.img"
        if os.path.exists(os.path.join(INSTALL_DIR, "efi_fat.img")):
            shutil.copy2(os.path.join(INSTALL_DIR, "efi_fat.img"), os.path.join(iso_root, boot_img_name))
        else:
            with open(os.path.join(iso_root, boot_img_name), "wb") as f:
                f.write(b"\x00" * (16 * 1024 * 1024))

        # Copy guest ISOs (skip files larger than 4GB for FAT/ISO limits)
        for name in guest_isos:
            src = os.path.join(ISO_DIR, name)
            dst = os.path.join(iso_root, "IMAGES", name)
            size = os.path.getsize(src)
            if size >= 4 * 1024 * 1024 * 1024:
                print(f"Skipping {name} (>= 4GB).")
                continue
            try:
                shutil.copy2(src, dst)
                print(f"Included guest ISO: {name}")
            except Exception as e:
                print(f"WARNING: could not include {name}: {e}")
                try:
                    if os.path.exists(dst):
                        os.remove(dst)
                except Exception:
                    pass

        # 5. Build ISO with xorriso
        if shutil.which("xorriso") is None:
            print("ERROR: xorriso not found.")
            sys.exit(1)

        cmd = [
            "xorriso", "-as", "mkisofs",
            "-r", "-J", "-V", "MULTIVERSE",
            "-o", OUTPUT,
            "-e", boot_img_name, "-no-emul-boot",
            iso_root,
        ]
        r = run(cmd)
        if r.returncode != 0:
            print("ISO creation failed.")
            sys.exit(1)

        print(f"Successfully created bootable ISO: {OUTPUT}")
    finally:
        shutil.rmtree(iso_root, ignore_errors=True)

if __name__ == "__main__":
    main()