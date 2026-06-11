//! Host memory and CPU capability detection (Windows v1).

const GB: u64 = 1024 * 1024 * 1024;

/// Minimum available RAM to preload Whisper on normal boot.
pub const PRELOAD_MIN_AVAIL_RAM_BYTES: u64 = 2 * GB;
/// Minimum available RAM to preload Whisper when started minimized.
pub const MINIMIZED_PRELOAD_MIN_AVAIL_RAM_BYTES: u64 = 4 * GB;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemCapabilities {
    pub total_ram_bytes: u64,
    pub avail_ram_bytes: u64,
    pub cpu_logical_cores: u32,
    pub gpu_compiled: bool,
}

impl SystemCapabilities {
    pub fn detect() -> Self {
        let (total_ram_bytes, avail_ram_bytes) = detect_memory_bytes();
        let cpu_logical_cores = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4);
        Self {
            total_ram_bytes,
            avail_ram_bytes,
            cpu_logical_cores,
            gpu_compiled: cfg!(feature = "gpu"),
        }
    }

    pub fn total_ram_gb(&self) -> f64 {
        self.total_ram_bytes as f64 / GB as f64
    }

    pub fn avail_ram_gb(&self) -> f64 {
        self.avail_ram_bytes as f64 / GB as f64
    }
}

#[cfg(windows)]
fn detect_memory_bytes() -> (u64, u64) {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe {
        if GlobalMemoryStatusEx(&mut status).is_ok() {
            let total = status.ullTotalPhys;
            let avail = status.ullAvailPhys;
            return (total, avail);
        }
    }
    fallback_memory_bytes()
}

#[cfg(not(windows))]
fn detect_memory_bytes() -> (u64, u64) {
    fallback_memory_bytes()
}

fn fallback_memory_bytes() -> (u64, u64) {
    // Conservative default aligned with PLAN target (16 Go).
    (16 * GB, 8 * GB)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_positive_values() {
        let caps = SystemCapabilities::detect();
        assert!(caps.total_ram_bytes > 0);
        assert!(caps.avail_ram_bytes > 0);
        assert!(caps.cpu_logical_cores >= 1);
    }
}
