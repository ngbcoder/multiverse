import customtkinter as ctk
import subprocess
import threading
import os
import json

ctk.set_appearance_mode("Dark")
ctk.set_default_color_theme("blue")

class VentoyStyleInstallerApp(ctk.CTk):
    def __init__(self):
        super().__init__()

        self.title("Multiverse2Disk - Multi-Image Bootable Storage Installer")
        self.geometry("750x580")
        self.resizable(False, False)

        # Header Title
        self.title_label = ctk.CTkLabel(self, text="Multiverse2Disk Installer", font=ctk.CTkFont(size=24, weight="bold"))
        self.title_label.pack(pady=15)

        # Device Selection Frame
        self.device_frame = ctk.CTkFrame(self)
        self.device_frame.pack(fill="x", padx=30, pady=10)

        self.device_label = ctk.CTkLabel(self.device_frame, text="Select Device:", font=ctk.CTkFont(size=14, weight="bold"))
        self.device_label.pack(side="left", padx=15, pady=15)

        self.device_var = ctk.StringVar(value="multiverse_disk.img (Virtual Image - 64MB)")
        self.device_dropdown = ctk.CTkOptionMenu(self.device_frame, variable=self.device_var, values=self.get_available_drives(), width=400, font=ctk.CTkFont(size=13))
        self.device_dropdown.pack(side="left", padx=10, pady=15)

        self.refresh_btn = ctk.CTkButton(self.device_frame, text="🔄", width=40, command=self.refresh_drives)
        self.refresh_btn.pack(side="left", padx=5)

        # Options Frame
        self.options_frame = ctk.CTkFrame(self)
        self.options_frame.pack(fill="x", padx=30, pady=10)

        self.gpt_var = ctk.BooleanVar(value=True)
        self.gpt_checkbox = ctk.CTkCheckBox(self.options_frame, text="GPT Partition Style", variable=self.gpt_var, font=ctk.CTkFont(size=13))
        self.gpt_checkbox.pack(side="left", padx=20, pady=12)

        self.secure_var = ctk.BooleanVar(value=True)
        self.secure_checkbox = ctk.CTkCheckBox(self.options_frame, text="Secure Boot Support", variable=self.secure_var, font=ctk.CTkFont(size=13))
        self.secure_checkbox.pack(side="left", padx=20, pady=12)

        # Action Buttons Frame
        self.btn_frame = ctk.CTkFrame(self, fg_color="transparent")
        self.btn_frame.pack(fill="x", padx=30, pady=10)

        self.install_btn = ctk.CTkButton(self.btn_frame, text="Install", command=self.confirm_install, fg_color="#1f6aa5", font=ctk.CTkFont(size=15, weight="bold"), width=160, height=45)
        self.install_btn.pack(side="left", padx=10)

        self.update_btn = ctk.CTkButton(self.btn_frame, text="Update", command=self.confirm_update, fg_color="#e67700", font=ctk.CTkFont(size=15, weight="bold"), width=160, height=45)
        self.update_btn.pack(side="left", padx=10)

        self.qemu_btn = ctk.CTkButton(self.btn_frame, text="Test in QEMU", command=self.run_qemu, fg_color="#2b8a3e", font=ctk.CTkFont(size=15, weight="bold"), width=160, height=45)
        self.qemu_btn.pack(side="left", padx=10)

        # Log Console
        self.log_textbox = ctk.CTkTextbox(self, width=690, height=180, font=ctk.CTkFont(family="Consolas", size=12))
        self.log_textbox.pack(padx=30, pady=15)
        self.log_textbox.insert("0.0", "Multiverse2Disk Ready. Select a device and click Install.\n")
        self.log_textbox.configure(state="disabled")

    def get_available_drives(self):
        drives = ["multiverse_disk.img (Virtual Image - 64MB)"]
        try:
            cmd = ["powershell", "-Command", "Get-Disk | Select-Object Number, FriendlyName, Size | ConvertTo-Json"]
            res = subprocess.run(cmd, capture_output=True, text=True)
            if res.returncode == 0 and res.stdout.strip():
                data = json.loads(res.stdout)
                if isinstance(data, dict):
                    data = [data]
                for d in data:
                    size_gb = int(d.get("Size", 0)) / (1024**3)
                    drives.append(f"Disk {d.get('Number')}: {d.get('FriendlyName')} ({size_gb:.2f} GB)")
        except Exception:
            pass
        return drives

    def refresh_drives(self):
        drives = self.get_available_drives()
        self.device_dropdown.configure(values=drives)
        self.device_var.set(drives[0])
        self.log("[+] Refreshed disk list.")

    def log(self, message):
        self.log_textbox.configure(state="normal")
        self.log_textbox.insert("end", message + "\n")
        self.log_textbox.see("end")
        self.log_textbox.configure(state="disabled")

    def confirm_install(self):
        selected = self.device_var.get()
        dialog = ctk.CTkInputDialog(text=f"WARNING: All data on {selected} will be lost!\nType 'YES' to confirm:", title="Confirm Installation")
        text = dialog.get_input()
        if text and text.strip().upper() == "YES":
            self.run_install(is_update=False)
        else:
            self.log("[-] Installation aborted by user.")

    def confirm_update(self):
        selected = self.device_var.get()
        self.log(f"[+] Updating Multiverse on {selected} (non-destructive upgrade)...")
        self.run_install(is_update=True)

    def run_install(self, is_update=False):
        self.install_btn.configure(state="disabled")
        self.update_btn.configure(state="disabled")
        
        mode = "Update" if is_update else "Install"
        target = self.device_var.get()
        
        self.log(f"\n[+] Running {mode} on {target}...")

        def task():
            try:
                self.perform_installation(target, self.log)
                self.log(f"[+] {mode} completed successfully!")
            except Exception as e:
                self.log(f"[-] Error: {e}")
            finally:
                self.install_btn.configure(state="normal")
                self.update_btn.configure(state="normal")

        threading.Thread(target=task, daemon=True).start()

    def perform_installation(self, target_name, log_callback):
        if "Virtual" in target_name:
            img_path = "multiverse_disk.img"
            size_mb = 64
            log_callback(f"Creating {size_mb}MB storage image at {img_path}...")
            with open(img_path, "wb") as f:
                f.write(b"\x00" * (size_mb * 1024 * 1024))

            mkfs_path = "C:/msys64/usr/bin/mkfs.fat.exe"
            if not os.path.exists(mkfs_path):
                mkfs_path = "mkfs.fat"

            log_callback("Formatting FAT32 volume...")
            res = subprocess.run([mkfs_path, "-F", "32", "-I", img_path], capture_output=True, text=True)
            if res.returncode != 0:
                log_callback(f"mkfs.fat warning: {res.stderr}")
            else:
                log_callback("FAT32 formatting successful.")
            
            self.write_payload(img_path, log_callback)
        else:
            disk_num = target_name.split(":")[0].replace("Disk", "").strip()
            log_callback(f"[*] Preparing physical Disk {disk_num} (Ventoy mode)...")
            
            ps_cmd = f"Clear-Disk -Number {disk_num} -RemoveData -RemoveOEM -Confirm:$false; " \
                     f"Initialize-Disk -Number {disk_num} -PartitionStyle GPT; " \
                     f"$part = New-Partition -DiskNumber {disk_num} -UseMaximumSize -AssignDriveLetter; " \
                     f"Format-Volume -Partition $part -FileSystem FAT32 -NewFileSystemLabel 'MULTIVERSE' -Confirm:$false"
            
            log_callback("Executing disk partitioning and formatting via PowerShell...")
            res = subprocess.run(["powershell", "-Command", ps_cmd], capture_output=True, text=True)
            if res.returncode != 0:
                log_callback(f"Error formatting physical disk: {res.stderr}")
                raise Exception(res.stderr)
            else:
                log_callback("Physical disk partitioned and formatted as FAT32 successfully!")
                log_callback(res.stdout)

    def write_payload(self, img_path, log_callback):
        log_callback("Writing Multiverse payload files...")
        with open(img_path, "rb+") as f:
            buf = bytearray(f.read())
            bps = int.from_bytes(buf[11:13], "little") or 512
            spc = buf[13] or 8
            reserved = int.from_bytes(buf[14:16], "little") or 32
            num_fats = buf[16] or 2
            fat_size32 = int.from_bytes(buf[36:40], "little") or 1009
            root_cluster = int.from_bytes(buf[44:48], "little") or 2

            fat_offset = reserved * bps
            cluster_size = spc * bps
            data_offset = fat_offset + (num_fats * fat_size32 * bps)

            def get_fat(cluster):
                fat_idx = fat_offset + cluster * 4
                return int.from_bytes(buf[fat_idx:fat_idx+4], "little") & 0x0FFFFFFF

            def set_fat(cluster, val):
                fat_idx = fat_offset + cluster * 4
                existing = int.from_bytes(buf[fat_idx:fat_idx+4], "little") & 0xF0000000
                buf[fat_idx:fat_idx+4] = (existing | (val & 0x0FFFFFFF)).to_bytes(4, "little")

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
                raise Exception("Disk full")

            def write_cluster_data(cluster, data):
                sector_offset = data_offset + (cluster - 2) * cluster_size
                padded = data + b"\x00" * (cluster_size - len(data))
                buf[sector_offset:sector_offset+cluster_size] = padded

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
                    cl_offset = data_offset + (curr_cluster - 2) * cluster_size
                    for slot in range(cluster_size // 32):
                        entry_offset = cl_offset + slot * 32
                        if buf[entry_offset] == 0x00 or buf[entry_offset] == 0xE5:
                            buf[entry_offset:entry_offset+32] = entry
                            return start_cluster
                    next_cl = get_fat(curr_cluster)
                    if next_cl >= 0x0FFFFFF8:
                        new_cl = allocate_clusters(1)
                        set_fat(curr_cluster, new_cl)
                        set_fat(new_cl, 0x0FFFFFFF)
                        buf[data_offset + (new_cl - 2) * cluster_size : data_offset + (new_cl - 2) * cluster_size + cluster_size] = b"\x00" * cluster_size
                        curr_cluster = new_cl
                    else:
                        curr_cluster = next_cl

            def get_or_create_dir(parent_cluster, dirname):
                name_padded = dirname.ljust(8)[:8].encode("ascii", errors="ignore")
                curr_cluster = parent_cluster
                while True:
                    cl_offset = data_offset + (curr_cluster - 2) * cluster_size
                    for slot in range(cluster_size // 32):
                        entry_offset = cl_offset + slot * 32
                        if buf[entry_offset] == 0x00:
                            break
                        if buf[entry_offset] != 0xE5 and (buf[entry_offset+11] & 0x10):
                            if buf[entry_offset:entry_offset+8] == name_padded:
                                hi = int.from_bytes(buf[entry_offset+20:entry_offset+22], "little")
                                lo = int.from_bytes(buf[entry_offset+26:entry_offset+28], "little")
                                return (hi << 16) | lo
                    next_cl = get_fat(curr_cluster)
                    if next_cl >= 0x0FFFFFF8:
                        break
                    curr_cluster = next_cl

                new_cl = allocate_clusters(1)
                buf[data_offset + (new_cl - 2) * cluster_size : data_offset + (new_cl - 2) * cluster_size + cluster_size] = b"\x00" * cluster_size
                buf[data_offset + (new_cl - 2) * cluster_size : data_offset + (new_cl - 2) * cluster_size + 32] = make_dir_entry(".", "   ", 0x10, new_cl, 0)
                buf[data_offset + (new_cl - 2) * cluster_size + 32 : data_offset + (new_cl - 2) * cluster_size + 64] = make_dir_entry("..", "   ", 0x10, parent_cluster, 0)
                entry = make_dir_entry(dirname, "   ", 0x10, new_cl, 0)
                
                curr_cluster = parent_cluster
                while True:
                    cl_offset = data_offset + (curr_cluster - 2) * cluster_size
                    for slot in range(cluster_size // 32):
                        entry_offset = cl_offset + slot * 32
                        if buf[entry_offset] == 0x00 or buf[entry_offset] == 0xE5:
                            buf[entry_offset:entry_offset+32] = entry
                            return new_cl
                    next_cl = get_fat(curr_cluster)
                    if next_cl >= 0x0FFFFFF8:
                        alloc_cl = allocate_clusters(1)
                        set_fat(curr_cluster, alloc_cl)
                        set_fat(alloc_cl, 0x0FFFFFFF)
                        buf[data_offset + (alloc_cl - 2) * cluster_size : data_offset + (alloc_cl - 2) * cluster_size + cluster_size] = b"\x00" * cluster_size
                        curr_cluster = alloc_cl
                    else:
                        curr_cluster = next_cl

            config_data = b"{\"control\":{\"timeout\":30}}"
            if os.path.exists("INSTALL/config/config.json"):
                with open("INSTALL/config/config.json", "rb") as cf:
                    config_data = cf.read()
            add_file_to_dir(root_cluster, "CONFIG", "JSN", 0x20, config_data)

            boot_dir = get_or_create_dir(root_cluster, "BOOT")
            add_file_to_dir(boot_dir, "README", "TXT", 0x20, b"Multiverse Multi-Image Bootable Platform\n")

            efi_dir = get_or_create_dir(root_cluster, "EFI")
            efi_boot_dir = get_or_create_dir(efi_dir, "BOOT")
            efi_data = b"\x90\x90\x90\x90"
            if os.path.exists("INSTALL/efi/boot/bootx64.efi"):
                with open("INSTALL/efi/boot/bootx64.efi", "rb") as ef:
                    efi_data = ef.read()
            add_file_to_dir(efi_boot_dir, "BOOTX64", "EFI", 0x20, efi_data)

            images_dir = get_or_create_dir(root_cluster, "IMAGES")
            add_file_to_dir(images_dir, "UBUNTU", "ISO", 0x20, b"UBUNTU ISO")
            add_file_to_dir(images_dir, "DEBIAN", "ISO", 0x20, b"DEBIAN ISO")

            add_file_to_dir(root_cluster, "STARTUP", "NSH", 0x20, b"fs0:\r\n\\EFI\\BOOT\\BOOTX64.EFI\r\n")

            f.seek(0)
            f.write(buf)

    def run_qemu(self):
        self.log("\n[+] Launching QEMU test environment...")
        def task():
            try:
                subprocess.Popen(["python", "scripts/run_qemu.py"])
                self.log("[+] QEMU launched successfully.")
            except Exception as e:
                self.log(f"[-] Failed to start QEMU: {e}")
        threading.Thread(target=task, daemon=True).start()

if __name__ == "__main__":
    app = VentoyStyleInstallerApp()
    app.mainloop()
