use serde::{Deserialize, Serialize};
use std::fs;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum MemoryPressureLevel {
    #[default]
    Normal, // Memória disponível > 20%
    Moderate, // Memória disponível entre 10% e 20% (sinaliza suspensão preventiva)
    Critical, // Memória disponível < 10% (suspensão agressiva imediata de abas inativas)
}

impl MemoryPressureLevel {
    pub fn is_under_pressure(&self) -> bool {
        matches!(
            self,
            MemoryPressureLevel::Moderate | MemoryPressureLevel::Critical
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMemorySnapshot {
    pub total_ram_kb: u64,
    pub available_ram_kb: u64,
    pub free_ram_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
    pub available_percent: f64,
    pub pressure_level: MemoryPressureLevel,
}

impl SystemMemorySnapshot {
    /// Inspeciona o estado real de memória do Linux através de `/proc/meminfo`.
    pub fn current() -> Option<Self> {
        let content = fs::read_to_string("/proc/meminfo").ok()?;

        let mut total_kb = 0u64;
        let mut free_kb = 0u64;
        let mut available_kb = 0u64;
        let mut swap_total_kb = 0u64;
        let mut swap_free_kb = 0u64;

        for line in content.lines() {
            let mut parts = line.split_whitespace();
            let key = match parts.next() {
                Some(k) => k,
                None => continue,
            };
            let val = match parts.next().and_then(|v| v.parse::<u64>().ok()) {
                Some(v) => v,
                None => continue,
            };

            match key {
                "MemTotal:" => total_kb = val,
                "MemFree:" => free_kb = val,
                "MemAvailable:" => available_kb = val,
                "SwapTotal:" => swap_total_kb = val,
                "SwapFree:" => swap_free_kb = val,
                _ => {}
            }
        }

        if total_kb == 0 {
            return None;
        }

        // Se MemAvailable não estiver disponível (kernels muito antigos), usa MemFree como aproximação
        let effective_available = if available_kb > 0 {
            available_kb
        } else {
            free_kb
        };
        let available_percent = (effective_available as f64 / total_kb as f64) * 100.0;

        let pressure_level = if available_percent < 10.0 {
            warn!(
                "Pressão crítica de memória detectada no sistema: {:.1}% disponível",
                available_percent
            );
            MemoryPressureLevel::Critical
        } else if available_percent < 20.0 {
            MemoryPressureLevel::Moderate
        } else {
            MemoryPressureLevel::Normal
        };

        Some(Self {
            total_ram_kb: total_kb,
            available_ram_kb: effective_available,
            free_ram_kb: free_kb,
            swap_total_kb,
            swap_free_kb,
            available_percent,
            pressure_level,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pressure_inspection() {
        let snapshot = SystemMemorySnapshot::current();
        assert!(
            snapshot.is_some(),
            "Falha ao obter estado de memória de /proc/meminfo"
        );
        let s = snapshot.unwrap();
        assert!(s.total_ram_kb > 0);
        assert!(s.available_ram_kb > 0);
        assert!(s.available_percent > 0.0 && s.available_percent <= 100.0);
    }
}
