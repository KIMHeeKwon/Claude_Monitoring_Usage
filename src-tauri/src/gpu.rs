//! GPU 사용률과 전용 메모리 (ARCHITECTURE §5.3).
//!
//! 읽지 못하면 `None`을 돌려주고 창은 GPU 행을 숨긴다 — GPU가 없거나 드라이버가 다른 PC에서도
//! 앱은 그대로 돌아야 한다.
//!
//! | 플랫폼 | 방법 | 상태 |
//! |---|---|---|
//! | Windows·Linux + NVIDIA | NVML (드라이버가 설치하는 `nvml.dll`을 실행 중에 연다) | 구현·실측 완료 |
//! | Windows + 그 밖의 GPU | 성능 카운터 `\GPU Engine(*)\Utilization Percentage` | 미구현 (필요해지면 추가) |
//! | macOS | IOKit `IOAccelerator`의 `PerformanceStatistics` | 미구현 (M3에서 실기기로) |

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Gpu {
    pub name: String,
    pub util_pct: f64,
    pub mem_used_gb: f64,
    pub mem_total_gb: f64,
}

const GB: f64 = 1024.0 * 1024.0 * 1024.0;

#[cfg(not(target_os = "macos"))]
mod nvidia {
    use super::{Gpu, GB};
    use nvml_wrapper::Nvml;

    pub struct Reader {
        nvml: Nvml,
        name: Option<String>,
    }

    impl Reader {
        /// 드라이버가 없으면 여기서 실패하고, 앱은 GPU 없이 계속 돈다.
        pub fn open() -> Option<Self> {
            Nvml::init().ok().map(|nvml| Self { nvml, name: None })
        }

        pub fn sample(&mut self) -> Option<Gpu> {
            let dev = self.nvml.device_by_index(0).ok()?;
            let util = dev.utilization_rates().ok()?;
            let mem = dev.memory_info().ok()?;
            // 이름은 바뀌지 않으므로 한 번만 읽는다.
            if self.name.is_none() {
                self.name = dev.name().ok();
            }
            Some(Gpu {
                name: self.name.clone().unwrap_or_else(|| "GPU".into()),
                util_pct: util.gpu as f64,
                mem_used_gb: mem.used as f64 / GB,
                mem_total_gb: mem.total as f64 / GB,
            })
        }
    }
}

#[cfg(target_os = "macos")]
mod nvidia {
    use super::Gpu;
    pub struct Reader;
    impl Reader {
        pub fn open() -> Option<Self> { None }
        pub fn sample(&mut self) -> Option<Gpu> { None }
    }
}

/// 샘플링 동안 살아 있는 읽기 핸들. 매 초 다시 여는 대신 한 번 열고 재사용한다.
pub struct Reader(Option<nvidia::Reader>);

impl Reader {
    pub fn open() -> Self {
        Self(nvidia::Reader::open())
    }

    pub fn sample(&mut self) -> Option<Gpu> {
        self.0.as_mut()?.sample()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 이 PC(RTX 4090)에서 실제 값이 읽히는지 본다. GPU가 없는 곳에서는 조용히 통과한다 —
    /// 읽지 못하는 것도 정상 동작이기 때문이다.
    #[test]
    fn reads_gpu_when_present() {
        let mut r = Reader::open();
        match r.sample() {
            Some(g) => {
                println!("GPU: {} · {:.0}% · {:.1}/{:.1} GB", g.name, g.util_pct, g.mem_used_gb, g.mem_total_gb);
                assert!(!g.name.is_empty());
                assert!((0.0..=100.0).contains(&g.util_pct));
                assert!(g.mem_total_gb > 0.0 && g.mem_used_gb <= g.mem_total_gb);
            }
            None => println!("GPU를 읽지 못했다 — 이 환경에서는 GPU 행이 숨겨진다"),
        }
    }
}
