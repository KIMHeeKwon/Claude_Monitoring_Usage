//! GPU 사용률과 전용 메모리 (ARCHITECTURE §5.3).
//!
//! 읽지 못하면 `None`을 돌려주고 창은 GPU 행을 숨긴다 — GPU가 없거나 드라이버가 다른 PC에서도
//! 앱은 그대로 돌아야 한다.
//!
//! **사용률은 Windows 성능 카운터로 읽는다.** 작업 관리자와 같은 값을 보여 주기 위해서다.
//! NVIDIA 라이브러리(NVML)의 `utilization.gpu`는 "커널이 하나라도 돌던 시간의 비율"이라
//! 짧은 작업이 잦으면 크게 부풀려진다 — 같은 순간에 NVML 28~33%, 작업 관리자 3.3%였다
//! (2026-09-03 실측). 사용자가 대조하는 기준은 작업 관리자이므로 그쪽을 따른다.
//!
//! | 값 | 출처 | 비고 |
//! |---|---|---|
//! | 사용률 | 성능 카운터 `\GPU Engine(*)\Utilization Percentage` | 엔진(3D·인코딩·복사 등)별로 합산한 뒤 그중 최대 — 작업 관리자와 같은 계산 |
//! | 이름·전용 메모리 | NVML | NVIDIA 전용. 없으면 이름은 "GPU", 메모리는 표시하지 않는다 |
//! | macOS | 미구현 (M3에서 IOKit) | 그때까지 GPU 행이 숨는다 |

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Gpu {
    pub name: String,
    pub util_pct: f64,
    /// NVIDIA가 아니면 `None` — 창은 VRAM 표시를 생략한다.
    pub mem_used_gb: Option<f64>,
    pub mem_total_gb: Option<f64>,
}

#[allow(dead_code)]
const GB: f64 = 1024.0 * 1024.0 * 1024.0;

// ---------- 사용률: Windows 성능 카운터 ----------

#[cfg(windows)]
mod pdh {
    use std::collections::HashMap;
    use windows::core::w;
    use windows::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
    };

    pub struct Counter {
        query: isize,
        counter: isize,
        buf: Vec<u8>,
    }

    impl Drop for Counter {
        fn drop(&mut self) {
            unsafe { PdhCloseQuery(self.query) };
        }
    }

    impl Counter {
        pub fn open() -> Option<Self> {
            unsafe {
                let mut query = 0isize;
                if PdhOpenQueryW(None, 0, &mut query) != 0 {
                    return None;
                }
                let mut counter = 0isize;
                // 영문 카운터 이름을 쓴다 — 한국어 Windows에서도 같은 이름으로 열린다.
                if PdhAddEnglishCounterW(query, w!("\\GPU Engine(*)\\Utilization Percentage"), 0, &mut counter) != 0 {
                    PdhCloseQuery(query);
                    return None;
                }
                // 비율 카운터는 두 번 걷어야 값이 나온다. 첫 표본을 여기서 잡아 둔다.
                PdhCollectQueryData(query);
                Some(Self { query, counter, buf: vec![0u8; 64 * 1024] })
            }
        }

        /// 엔진별로 모든 프로세스를 합산한 뒤 그중 최대를 돌려준다 (작업 관리자와 같은 계산).
        pub fn sample(&mut self) -> Option<f64> {
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return None;
                }
                let mut size = self.buf.len() as u32;
                let mut count = 0u32;
                let mut rc = PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    Some(self.buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
                );
                // 버퍼가 모자라면 필요한 크기로 늘려 한 번 더 시도한다 (인스턴스가 수백 개일 수 있다).
                if rc != 0 && size as usize > self.buf.len() {
                    self.buf.resize(size as usize, 0);
                    size = self.buf.len() as u32;
                    rc = PdhGetFormattedCounterArrayW(
                        self.counter,
                        PDH_FMT_DOUBLE,
                        &mut size,
                        &mut count,
                        Some(self.buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
                    );
                }
                if rc != 0 {
                    return None;
                }

                let items = std::slice::from_raw_parts(
                    self.buf.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
                    count as usize,
                );
                let mut per_engine: HashMap<String, f64> = HashMap::new();
                for it in items {
                    let value = it.FmtValue.Anonymous.doubleValue;
                    if !value.is_finite() || value <= 0.0 {
                        continue;
                    }
                    // 인스턴스 이름 예: pid_1234_luid_0x0_0xABCD_phys_0_eng_0_engtype_3D
                    let name = it.szName.to_string().unwrap_or_default();
                    let engine = match name.rsplit_once("engtype_") {
                        Some((_, e)) => e.to_string(),
                        None => continue,
                    };
                    *per_engine.entry(engine).or_insert(0.0) += value;
                }
                let max = per_engine.values().copied().fold(0.0_f64, f64::max);
                Some(max.clamp(0.0, 100.0))
            }
        }
    }
}

#[cfg(not(windows))]
mod pdh {
    pub struct Counter;
    impl Counter {
        pub fn open() -> Option<Self> { None }
        pub fn sample(&mut self) -> Option<f64> { None }
    }
}

// ---------- 이름·전용 메모리: NVML (NVIDIA 전용) ----------

#[cfg(not(target_os = "macos"))]
mod nvidia {
    use super::GB;
    use nvml_wrapper::Nvml;

    pub struct Reader {
        nvml: Nvml,
        name: Option<String>,
    }

    /// 이름과 전용 메모리(사용/전체 GB). 사용률은 성능 카운터가 맡는다.
    pub struct Info {
        pub name: String,
        pub mem_used_gb: f64,
        pub mem_total_gb: f64,
    }

    impl Reader {
        pub fn open() -> Option<Self> {
            Nvml::init().ok().map(|nvml| Self { nvml, name: None })
        }

        pub fn info(&mut self) -> Option<Info> {
            let dev = self.nvml.device_by_index(0).ok()?;
            let mem = dev.memory_info().ok()?;
            if self.name.is_none() {
                self.name = dev.name().ok();
            }
            Some(Info {
                name: self.name.clone().unwrap_or_else(|| "GPU".into()),
                mem_used_gb: mem.used as f64 / GB,
                mem_total_gb: mem.total as f64 / GB,
            })
        }
    }
}

#[cfg(target_os = "macos")]
mod nvidia {
    pub struct Reader;
    pub struct Info {
        pub name: String,
        pub mem_used_gb: f64,
        pub mem_total_gb: f64,
    }
    impl Reader {
        pub fn open() -> Option<Self> { None }
        pub fn info(&mut self) -> Option<Info> { None }
    }
}

// ---------- 합치기 ----------

/// 샘플링 동안 살아 있는 읽기 핸들. 매 초 다시 여는 대신 한 번 열고 재사용한다.
pub struct Reader {
    util: Option<pdh::Counter>,
    nv: Option<nvidia::Reader>,
}

impl Reader {
    pub fn open() -> Self {
        Self { util: pdh::Counter::open(), nv: nvidia::Reader::open() }
    }

    pub fn sample(&mut self) -> Option<Gpu> {
        let util_pct = self.util.as_mut()?.sample()?;
        match self.nv.as_mut().and_then(|n| n.info()) {
            Some(i) => Some(Gpu {
                name: i.name,
                util_pct,
                mem_used_gb: Some(i.mem_used_gb),
                mem_total_gb: Some(i.mem_total_gb),
            }),
            // NVIDIA가 아니면 사용률만 보여 준다.
            None => Some(Gpu { name: "GPU".into(), util_pct, mem_used_gb: None, mem_total_gb: None }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 실제 값이 읽히는지, 그리고 작업 관리자와 같은 자릿수인지 눈으로 확인한다.
    /// GPU를 읽지 못하는 환경에서는 조용히 통과한다 — 그것도 정상 동작이기 때문이다.
    #[test]
    fn reads_gpu_when_present() {
        let mut r = Reader::open();
        // 비율 카운터는 두 번째 표본부터 값이 나온다.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        match r.sample() {
            Some(g) => {
                let mem = match (g.mem_used_gb, g.mem_total_gb) {
                    (Some(u), Some(t)) => format!("{u:.1}/{t:.1} GB"),
                    _ => "메모리 없음".into(),
                };
                println!("GPU: {} · {:.1}% · {}", g.name, g.util_pct, mem);
                assert!(!g.name.is_empty());
                assert!((0.0..=100.0).contains(&g.util_pct));
            }
            None => println!("GPU를 읽지 못했다 — 이 환경에서는 GPU 행이 숨겨진다"),
        }
    }
}
