use serde::Serialize;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

#[derive(Debug, Clone, Copy, Serialize)]
pub struct MemoryInfo {
    pub total_mb: u64,
    pub available_mb: u64,
}

/// 读取物理内存总量与可用量（MB）。
pub fn memory_info() -> MemoryInfo {
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    if ok == 0 {
        return MemoryInfo {
            total_mb: 0,
            available_mb: 0,
        };
    }
    MemoryInfo {
        total_mb: status.ullTotalPhys / 1024 / 1024,
        available_mb: status.ullAvailPhys / 1024 / 1024,
    }
}

/// 类似 PCL 的自动分配：约三分之一系统内存，取 512 MB 的整数倍。
pub fn recommended_memory_mb(total_mb: u64) -> u32 {
    let total = if total_mb > 0 {
        total_mb
    } else {
        memory_info().total_mb
    };
    if total == 0 {
        return 4096;
    }
    let target = (total / 3).clamp(1024, 8192);
    ((target / 512) * 512).max(1024) as u32
}
