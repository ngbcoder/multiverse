use multiverse_runtime::{
    InitDispatcher, InitMode, DeviceMapperTable, CompressionFormat, CpioInjector, ChainScriptContext,
    FuseIsoMount, LoopDeviceManager, WinPeMount
};

#[test]
fn test_init_dispatcher_chain() {
    let args = vec!["init".to_string(), "quiet".to_string()];
    assert_eq!(InitDispatcher::dispatch(&args), InitMode::Chain);
}

#[test]
fn test_init_dispatcher_loop() {
    let args = vec!["init".to_string(), "multiverse_loop=1".to_string()];
    assert_eq!(InitDispatcher::dispatch(&args), InitMode::Loop);
}

#[test]
fn test_device_mapper_linear() {
    let dm = DeviceMapperTable::linear("vdev0", 2048, "/dev/sda2", 4096);
    let s = dm.to_table_string();
    assert_eq!(s, "0 2048 linear /dev/sda2 4096");
}

#[test]
fn test_device_mapper_striped() {
    let devs = vec![("/dev/sda2", 0u64), ("/dev/sdb2", 0u64)];
    let dm = DeviceMapperTable::striped("vdev1", 4096, 64, &devs);
    let s = dm.to_table_string();
    assert!(s.contains("striped 2 64 /dev/sda2 0 /dev/sdb2 0"));
}

#[test]
fn test_compression_format_detection() {
    assert_eq!(CompressionFormat::detect(&[0x1F, 0x8B, 0x00]), CompressionFormat::Gzip);
    assert_eq!(CompressionFormat::detect(b"BZh9111"), CompressionFormat::Bzip2);
    assert_eq!(CompressionFormat::detect(b"\xFD7zXZ\x00extra"), CompressionFormat::Xz);
    assert_eq!(CompressionFormat::detect(b"plain_data"), CompressionFormat::Uncompressed);
}

#[test]
fn test_cpio_injector() {
    let cmd = CpioInjector::prepare_injection_layer("runtime.cpio", "/newroot").unwrap();
    assert!(cmd.contains("cpio -idmv < runtime.cpio -D /newroot"));
}

#[test]
fn test_chain_script_context() {
    let ctx = ChainScriptContext::new("ubuntu")
        .with_persistence("/dev/sda3")
        .with_dud_disks(vec!["/dev/sdb".to_string()]);
    let script = ctx.generate_script();
    assert!(script.contains("hook/ubuntu/os-hook.sh"));
    assert!(script.contains("PERSISTENCE_DEV=/dev/sda3"));
    assert!(script.contains("DUD_DISK_0=/dev/sdb"));
    assert!(script.contains("exec switch_root /newroot /sbin/init"));
}

#[test]
fn test_fuse_iso_mount() {
    let cmd = FuseIsoMount::build_mount_command("/ubuntu.iso", "/mnt/iso");
    assert_eq!(cmd, "fuseiso -p /ubuntu.iso /mnt/iso");
}

#[test]
fn test_loop_device_manager() {
    let cmd = LoopDeviceManager::build_losetup_command("/dev/loop0", "/image.iso", 1048576);
    assert_eq!(cmd, "losetup -o 1048576 /dev/loop0 /image.iso");
}

#[test]
fn test_winpe_mount() {
    let cmd = WinPeMount::build_imdisk_command('Y', "/sources/boot.wim");
    assert_eq!(cmd, "imdisk -a -t file -f /sources/boot.wim -m Y:");
}
