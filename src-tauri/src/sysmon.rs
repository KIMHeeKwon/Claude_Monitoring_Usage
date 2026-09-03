//! CPU / memory sampling (ARCHITECTURE §5.3). GPU is added in M2.
//! Emits the `sys:update` event (contract: ARCHITECTURE §4) every second.

use serde::Serialize;
use std::{thread, time::Duration};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tauri::{AppHandle, Emitter};

const GB: f64 = 1024.0 * 1024.0 * 1024.0;

#[derive(Serialize, Clone)]
pub struct Mem {
    pub used_gb: f64,
    pub total_gb: f64,
}

#[derive(Serialize, Clone)]
pub struct SysSnapshot {
    pub cpu_pct: f32,
    pub mem: Mem,
    /// Reserved for M2 (NVML / PDH / IOKit). `None` hides the GPU row.
    pub gpu: Option<()>,
}

pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        // First CPU reading is meaningless (needs two samples); wait the minimum interval.
        sys.refresh_cpu_usage();
        thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        loop {
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            let snap = SysSnapshot {
                cpu_pct: sys.global_cpu_usage(),
                mem: Mem {
                    used_gb: sys.used_memory() as f64 / GB,
                    total_gb: sys.total_memory() as f64 / GB,
                },
                gpu: None,
            };
            // A failed emit only means no window is listening yet; keep sampling.
            let _ = app.emit("sys:update", snap);
            thread::sleep(Duration::from_secs(1));
        }
    });
}
