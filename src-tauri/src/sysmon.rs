//! CPU · 메모리 · GPU 표본 수집 (ARCHITECTURE §5.3).
//! 1초마다 `sys:update` 이벤트를 보낸다 (계약: ARCHITECTURE §4).
//! 핸들(sysinfo, NVML)은 열어 둔 채 재사용한다 — 매 초 자식 프로세스를 띄우지 않는다.

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
    /// 읽지 못하면 `None`이고 창은 GPU 행을 숨긴다.
    pub gpu: Option<crate::gpu::Gpu>,
}

pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        let mut gpu = crate::gpu::Reader::open();
        // 첫 CPU 값은 의미가 없다(두 번 재야 차이가 나온다). 최소 간격만큼 기다린다.
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
                gpu: gpu.sample(),
            };
            // 창이 아직 없으면 전송이 실패한다 — 무시하고 계속 잰다.
            let _ = app.emit("sys:update", snap);
            thread::sleep(Duration::from_secs(1));
        }
    });
}
