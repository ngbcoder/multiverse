#!/usr/bin/env python3
import subprocess
import os

def main():
    qemu_path = "C:/msys64/ucrt64/bin/qemu-system-x86_64.exe"
    bios_path = "C:/msys64/ucrt64/share/qemu/edk2-x86_64-code.fd"
    img_path = "multiverse_disk.img"

    if not os.path.exists(img_path):
        print("Image not found. Building image first...")
        subprocess.run(["python", "scripts/make_bootable_img.py"], check=True)

    print("Starting QEMU with Multiverse bootable image...")
    cmd = [
        qemu_path,
        "-drive", f"if=pflash,format=raw,readonly=on,file={bios_path}",
        "-drive", f"file={img_path},format=raw",
        "-m", "2048"
    ]
    subprocess.run(cmd)

if __name__ == "__main__":
    main()
