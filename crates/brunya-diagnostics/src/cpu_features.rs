use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuCapabilities {
    pub architecture: String,
    pub has_sse4_2: bool,
    pub has_aes_ni: bool,
    pub has_avx2: bool,
    pub has_avx512: bool,
    pub has_bmi2: bool,
    pub has_rdrand: bool,
}

impl CpuCapabilities {
    /// Detecta dinamicamente em tempo de execução os conjuntos de instruções do processador host,
    /// garantindo que o binário universal x86-64 funcione sem travamentos em CPUs antigas.
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self {
                architecture: "x86_64".to_string(),
                has_sse4_2: std::is_x86_feature_detected!("sse4.2"),
                has_aes_ni: std::is_x86_feature_detected!("aes"),
                has_avx2: std::is_x86_feature_detected!("avx2"),
                has_avx512: std::is_x86_feature_detected!("avx512f"),
                has_bmi2: std::is_x86_feature_detected!("bmi2"),
                has_rdrand: std::is_x86_feature_detected!("rdrand"),
            }
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            Self {
                architecture: std::env::consts::ARCH.to_string(),
                has_sse4_2: false,
                has_aes_ni: false,
                has_avx2: false,
                has_avx512: false,
                has_bmi2: false,
                has_rdrand: false,
            }
        }
    }

    pub fn summary_string(&self) -> String {
        let mut feats = Vec::new();
        if self.has_sse4_2 {
            feats.push("SSE4.2");
        }
        if self.has_aes_ni {
            feats.push("AES-NI");
        }
        if self.has_avx2 {
            feats.push("AVX2");
        }
        if self.has_avx512 {
            feats.push("AVX-512");
        }
        if self.has_bmi2 {
            feats.push("BMI2");
        }
        if self.has_rdrand {
            feats.push("RDRAND");
        }

        if feats.is_empty() {
            format!("{} (Conjunto x86-64 Base)", self.architecture)
        } else {
            format!("{} [Extensões: {}]", self.architecture, feats.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_feature_detection() {
        let caps = CpuCapabilities::detect();
        assert_eq!(caps.architecture, "x86_64");
        // No host de teste AMD Ryzen moderno, SSE4.2 e AES-NI devem estar disponíveis
        println!("CPU Capabilities detectadas: {}", caps.summary_string());
    }
}
