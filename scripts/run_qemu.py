#!/usr/bin/env python3
"""Run Multiverse in QEMU (works on Linux/WSL and Windows)."""

import argparse
import os
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

def find_qemu():
    for name in ("qemu-system-x86_64", "qemu-system-x86_64.exe"):
        path = shutil.which(name)
        if path:
            return path
    # Common Windows (msys2) location
    win = "C:/msys64/ucrt64/bin/qemu-system-x86_64.exe"
    if os.path.exists(win):
        return win
    return None

def find_firmware():
    candidates = [
        "/usr/share/edk2/x64/OVMF.4m.fd",
        "/usr/share/edk2/x64/OVMF_CODE.4m.fd",
        "/usr/share/OVMF/x64/OVMF_CODE.fd",
        "C:/msys64/ucrt64/share/qemu/edk2-x86_64-code.fd",
        "C:/msys64/usr/share/qemu/edk2-x86_64-code.fd",
        "C:/msys64/ucrt64/share/edk2-ovmf/x64/OVMF_CODE.fd",
    ]
    # Prefer exact known firmware files
    candidates = [
        "C:/msys64/ucrt64/share/qemu/edk2-x86_64-code.fd",
        "C:/msys64/ucrt64/share/qemu/edk2-x86_64-secure-code.fd",
        os.path.join(ROOT, "ovmf.fd"),
    ] + candidates
    for c in candidates:
        if c and os.path.exists(c):
            return c
    # Last resort: repo ovmf.fd (may be invalid)
    repo_ovmf = os.path.join(ROOT, "ovmf.fd")
    if os.path.exists(repo_ovmf):
        return repo_ovmf
    return None

def main():
    parser = argparse.ArgumentParser(description="Run Multiverse in QEMU")
    parser.add_argument("--iso", action="store_true", help="Boot the generated multiverse.iso instead of the disk image")
    parser.add_argument("--disk", default="multiverse_disk.img", help="Disk image path")
    parser.add_argument("--iso-path", default="multiverse.iso", help="ISO path")
    parser.add_argument("--mem", default="2048", help="RAM in MB")
    parser.add_argument("--nographic", action="store_true", help="Run QEMU without a graphical window")
    args = parser.parse_args()

    os.chdir(ROOT)

    qemu = find_qemu()
    if not qemu:
        print("ERROR: qemu-system-x86_64 not found.")
        sys.exit(1)

    fw = find_firmware()
    if not fw:
        print("ERROR: OVMF firmware not found.")
        sys.exit(1)
    print("Selected firmware:", fw, "size:", os.path.getsize(fw))

    # Create a writable vars file for pflash (QEMU needs a separate vars image).
    import tempfile
    vars_path = os.path.join(tempfile.gettempdir(), "multiverse_ovmf_vars.fd")
    if not os.path.exists(vars_path):
        with open(vars_path, "wb") as vf:
            vf.write(b"\\x00" * (128 * 1024))

    if args.iso:
        img_path = args.iso_path
        if not os.path.exists(img_path):
            print("ISO not found. Building ISO first...")
            subprocess.run([sys.executable, "scripts/make_bootable_iso.py"], check=True)
        cmd = [
            qemu,
            "-drive", f"if=pflash,format=raw,readonly=on,file={fw}",
            "-drive", f"if=pflash,format=raw,file={vars_path}",
            "-cdrom", img_path,
            "-m", args.mem,
            "-boot", "d",
        ]
    else:
        img_path = args.disk
        if not os.path.exists(img_path):
            print("Disk image not found. Building image first...")
            subprocess.run([sys.executable, "scripts/make_bootable_img.py"], check=True)
        cmd = [
            qemu,
            "-drive", f"if=pflash,format=raw,readonly=on,file={fw}",
            "-drive", f"if=pflash,format=raw,file={vars_path}",
            "-drive", f"file={img_path},format=raw",
            "-m", args.mem,
        ]
    if args.nographic:
        cmd.append("-nographic")

    print("Starting QEMU with Multiverse...")
    print("Command:", " ".join(cmd))
    subprocess.run(cmd)

if __name__ == "__main__":
    main()