use multiverse_core::PlatformId;

/// Detect platform id from compile-time target architecture.
pub fn current_platform() -> PlatformId {
    if cfg!(target_arch = "x86_64") {
        // default to UEFI; legacy BIOS is a separate build target
        PlatformId::X86_64Uefi
    } else if cfg!(target_arch = "x86") {
        PlatformId::Ia32Uefi
    } else if cfg!(target_arch = "aarch64") {
        PlatformId::AArch64Uefi
    } else if cfg!(target_arch = "mips64") {
        PlatformId::Mips64ElUefi
    } else {
        PlatformId::LegacyBios
    }
}

pub fn is_legacy() -> bool {
    matches!(current_platform(), PlatformId::LegacyBios)
}