//! Diagnósticos de desempenho e medição precisa de recursos do sistema para o Brunya Browser.

use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Instant;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMemoryReport {
    pub resident_set_size_kb: u64,
    pub virtual_memory_kb: u64,
    pub shared_memory_kb: u64,
}

impl ProcessMemoryReport {
    /// Mede o consumo real do processo no Linux inspecionando `/proc/self/statm`.
    pub fn current_process() -> Option<Self> {
        let content = fs::read_to_string("/proc/self/statm").ok()?;
        let mut parts = content.split_whitespace();

        let vms_pages: u64 = parts.next()?.parse().ok()?;
        let rss_pages: u64 = parts.next()?.parse().ok()?;
        let shared_pages: u64 = parts.next()?.parse().ok()?;

        let page_size_kb = match unsafe { libc::sysconf(libc::_SC_PAGESIZE) } {
            s if s > 0 => (s as u64) / 1024,
            _ => 4, // 4KB por página padrão x86-64
        };

        Some(Self {
            resident_set_size_kb: rss_pages * page_size_kb,
            virtual_memory_kb: vms_pages * page_size_kb,
            shared_memory_kb: shared_pages * page_size_kb,
        })
    }
}

pub struct StartupBenchmark {
    start_time: Instant,
}

impl StartupBenchmark {
    pub fn start() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    pub fn mark_interactive(&self, label: &str) -> u128 {
        let elapsed = self.start_time.elapsed().as_millis();
        info!("Benchmark [{}]: {} ms até prontidão interativa", label, elapsed);
        elapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_report_linux() {
        let report = ProcessMemoryReport::current_process();
        assert!(report.is_some(), "Falha ao ler /proc/self/statm no Linux");
        let mem = report.unwrap();
        assert!(mem.resident_set_size_kb > 0);
        assert!(mem.virtual_memory_kb > 0);
    }

    #[test]
    fn test_benchmark_timer() {
        let timer = StartupBenchmark::start();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let ms = timer.mark_interactive("test");
        assert!(ms >= 4);
    }
}
