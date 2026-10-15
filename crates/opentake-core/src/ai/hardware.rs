//! Local Hardware and System Environment Detection for AI Operations.

use super::models::AiHardwareInfo;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Detects local hardware capabilities, available memory, and storage metrics safely.
pub fn detect_hardware(storage_path: &Path) -> AiHardwareInfo {
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    let logical_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let (total_ram_bytes, available_ram_bytes) = read_system_ram();
    let (storage_total_bytes, storage_available_bytes) = read_storage_space(storage_path);
    let (gpu_detected, gpu_device_name) = detect_gpu_presence();

    AiHardwareInfo {
        os,
        arch,
        logical_cpus,
        total_ram_bytes,
        available_ram_bytes,
        gpu_detected,
        gpu_device_name,
        gpu_backend_installed: false,
        gpu_runtime_usable: false,
        storage_total_bytes,
        storage_available_bytes,
        supported_runtime_backends: vec![],
    }
}

/// Reads total and available RAM from /proc/meminfo on Linux, or returns None.
fn read_system_ram() -> (Option<u64>, Option<u64>) {
    let path = Path::new("/proc/meminfo");
    if !path.exists() {
        return (None, None);
    }

    let Ok(file) = File::open(path) else {
        return (None, None);
    };

    let reader = BufReader::new(file);
    let mut total_kb: Option<u64> = None;
    let mut available_kb: Option<u64> = None;

    for line in reader.lines().map_while(Result::ok) {
        if line.starts_with("MemTotal:") {
            total_kb = parse_meminfo_kb(&line);
        } else if line.starts_with("MemAvailable:") {
            available_kb = parse_meminfo_kb(&line);
        }
        if total_kb.is_some() && available_kb.is_some() {
            break;
        }
    }

    (
        total_kb.map(|kb| kb.saturating_mul(1024)),
        available_kb.map(|kb| kb.saturating_mul(1024)),
    )
}

fn parse_meminfo_kb(line: &str) -> Option<u64> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse::<u64>().ok()
    } else {
        None
    }
}

/// Reads storage total and available bytes using statvfs on POSIX systems.
fn read_storage_space(path: &Path) -> (Option<u64>, Option<u64>) {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    // Resolve an existing ancestor path if target directory does not yet exist
    let mut current = path.to_path_buf();
    while !current.exists() {
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    let Ok(c_path) = CString::new(current.to_string_lossy().as_bytes()) else {
        return (None, None);
    };

    unsafe {
        let mut stat = MaybeUninit::<libc::statvfs>::uninit();
        if libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) == 0 {
            let stat = stat.assume_init();
            let block_size = stat.f_frsize;
            let total = stat.f_blocks.saturating_mul(block_size);
            let available = stat.f_bavail.saturating_mul(block_size);
            (Some(total), Some(available))
        } else {
            (None, None)
        }
    }
}

/// Best-effort GPU presence detection on Linux via /sys/class/drm and /proc/driver/nvidia.
fn detect_gpu_presence() -> (bool, Option<String>) {
    // Check NVIDIA proc driver
    let nvidia_path = Path::new("/proc/driver/nvidia/version");
    if nvidia_path.exists() {
        if let Ok(content) = std::fs::read_to_string(nvidia_path) {
            let first_line = content.lines().next().unwrap_or("NVIDIA GPU").trim();
            return (true, Some(first_line.to_string()));
        }
        return (true, Some("NVIDIA GPU (Driver Detected)".to_string()));
    }

    // Check DRI devices in /sys/class/drm
    let drm_path = Path::new("/sys/class/drm");
    if drm_path.exists() {
        if let Ok(entries) = std::fs::read_dir(drm_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("card") && !name.contains('-') {
                    // Check vendor if available
                    let vendor_path = entry.path().join("device/vendor");
                    if vendor_path.exists() {
                        if let Ok(vendor_hex) = std::fs::read_to_string(vendor_path) {
                            let vendor_clean = vendor_hex.trim().to_lowercase();
                            let vendor_str = match vendor_clean.as_str() {
                                "0x10de" => "NVIDIA GPU",
                                "0x1002" => "AMD Radeon GPU",
                                "0x8086" => "Intel Integrated Graphics",
                                _ => "PCI GPU",
                            };
                            return (true, Some(vendor_str.to_string()));
                        }
                    }
                    return (true, Some("Display GPU".to_string()));
                }
            }
        }
    }

    (false, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_detection_returns_valid_data() {
        let temp = tempfile::tempdir().expect("tempdir");
        let info = detect_hardware(temp.path());

        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(info.logical_cpus >= 1);
        assert!(!info.gpu_backend_installed);
        assert!(!info.gpu_runtime_usable);

        if cfg!(target_os = "linux") {
            assert!(info.storage_total_bytes.is_some());
            assert!(info.storage_available_bytes.is_some());
        }
    }
}
