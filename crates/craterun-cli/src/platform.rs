/// Detect the default build/package target based on the current OS and architecture.
pub fn default_build_target() -> String {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "macos-arm64".to_string();
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "macos-x64".to_string();
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "linux-x64".to_string();
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "linux-arm64".to_string();
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "windows-x64".to_string();
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    return format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
}

pub fn current_os() -> &'static str {
    #[cfg(target_os = "windows")]
    return "windows";
    #[cfg(target_os = "macos")]
    return "macos";
    #[cfg(target_os = "linux")]
    return "linux";
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return std::env::consts::OS;
}
