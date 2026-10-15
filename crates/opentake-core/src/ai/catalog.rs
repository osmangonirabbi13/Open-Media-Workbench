//! Trusted Curated Model Catalog for Open Media Workbench.

use super::models::{AiModelCapability, AiModelManifest};

/// List of trusted hosts permitted for model artifact downloads.
pub const TRUSTED_HOSTS: &[&str] = &["huggingface.co"];

/// Verifies whether a given URL adheres to the trusted host and HTTPS security policy.
pub fn is_trusted_download_url(url_str: &str) -> bool {
    let lower = url_str.to_lowercase();
    // In test and debug builds, allow local mock server for tests and offline fixtures
    if (cfg!(test) || cfg!(debug_assertions))
        && (lower.starts_with("http://127.0.0.1:") || lower.starts_with("http://localhost:"))
    {
        return true;
    }

    if !lower.starts_with("https://") {
        return false;
    }

    if let Ok(parsed) = reqwest::Url::parse(url_str) {
        if let Some(host) = parsed.host_str() {
            let host_lower = host.to_lowercase();
            return TRUSTED_HOSTS.iter().any(|&trusted| {
                host_lower == trusted || host_lower.ends_with(&format!(".{trusted}"))
            });
        }
    }
    false
}

/// Returns the static, versioned, trusted AI model catalog for Open Media Workbench.
pub fn get_curated_model_catalog() -> Vec<AiModelManifest> {
    vec![
        AiModelManifest {
            id: "whisper-tiny".to_string(),
            name: "Whisper Tiny (Multilingual)".to_string(),
            family: "whisper".to_string(),
            version: "v1".to_string(),
            capability: AiModelCapability::SpeechToText,
            supported_languages: vec![
                "en".into(), "bn".into(), "es".into(), "fr".into(), "de".into(),
                "hi".into(), "ja".into(), "zh".into(), "ar".into(), "ru".into(),
            ],
            runtime_backend: "whisper.cpp".to_string(),
            runtime_format: "ggml".to_string(),
            source_repository: "https://huggingface.co/ggerganov/whisper.cpp".to_string(),
            artifact_url: Some(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"
                    .to_string(),
            ),
            artifact_filename: "ggml-tiny.bin".to_string(),
            expected_size_bytes: 77_691_713,
            expected_sha256: Some(
                "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21".to_string(),
            ),
            license_name: "MIT".to_string(),
            license_url: "https://huggingface.co/openai/whisper/blob/main/LICENSE".to_string(),
            model_card_url: "https://huggingface.co/openai/whisper-tiny".to_string(),
            min_ram_bytes: 1_073_741_824, // 1 GB
            cpu_compatible: true,
            gpu_recommended: false,
            installable: true,
            description: "Fast, lightweight multilingual speech recognition model for quick draft subtitles."
                .to_string(),
            schema_version: 1,
        },
        AiModelManifest {
            id: "whisper-base".to_string(),
            name: "Whisper Base (Multilingual)".to_string(),
            family: "whisper".to_string(),
            version: "v1".to_string(),
            capability: AiModelCapability::SpeechToText,
            supported_languages: vec![
                "en".into(), "bn".into(), "es".into(), "fr".into(), "de".into(),
                "hi".into(), "ja".into(), "zh".into(), "ar".into(), "ru".into(),
            ],
            runtime_backend: "whisper.cpp".to_string(),
            runtime_format: "ggml".to_string(),
            source_repository: "https://huggingface.co/ggerganov/whisper.cpp".to_string(),
            artifact_url: Some(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin"
                    .to_string(),
            ),
            artifact_filename: "ggml-base.bin".to_string(),
            expected_size_bytes: 147_951_465,
            expected_sha256: Some(
                "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe".to_string(),
            ),
            license_name: "MIT".to_string(),
            license_url: "https://huggingface.co/openai/whisper/blob/main/LICENSE".to_string(),
            model_card_url: "https://huggingface.co/openai/whisper-base".to_string(),
            min_ram_bytes: 2_147_483_648, // 2 GB
            cpu_compatible: true,
            gpu_recommended: false,
            installable: true,
            description: "Balanced accuracy and speed for multilingual speech-to-text in production."
                .to_string(),
            schema_version: 1,
        },
        AiModelManifest {
            id: "piper-en-lessac-medium".to_string(),
            name: "Piper Voice (English - Lessac Medium)".to_string(),
            family: "piper".to_string(),
            version: "v1.0.0".to_string(),
            capability: AiModelCapability::TextToSpeech,
            supported_languages: vec!["en-US".into()],
            runtime_backend: "piper".to_string(),
            runtime_format: "onnx".to_string(),
            source_repository: "https://huggingface.co/rhasspy/piper-voices".to_string(),
            artifact_url: Some(
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx"
                    .to_string(),
            ),
            artifact_filename: "en_US-lessac-medium.onnx".to_string(),
            expected_size_bytes: 63_201_294,
            expected_sha256: Some(
                "5efe09e69902187827af646e1a6e9d269dee769f9877d17b16b1b46eeaaf019f".to_string(),
            ),
            license_name: "Public Domain".to_string(),
            license_url: "https://github.com/rhasspy/piper/blob/master/VOICES.md".to_string(),
            model_card_url: "https://huggingface.co/rhasspy/piper-voices".to_string(),
            min_ram_bytes: 1_073_741_824,
            cpu_compatible: true,
            gpu_recommended: false,
            installable: true,
            description: "High-quality neural text-to-speech voice for video localization and voiceover."
                .to_string(),
            schema_version: 1,
        },
        AiModelManifest {
            id: "qwen2.5-0.5b-instruct".to_string(),
            name: "Qwen 2.5 0.5B Instruct (GGUF Q4_K_M)".to_string(),
            family: "qwen".to_string(),
            version: "v2.5".to_string(),
            capability: AiModelCapability::TextGeneration,
            supported_languages: vec![
                "en".into(), "zh".into(), "es".into(), "fr".into(), "de".into(),
                "bn".into(), "ru".into(), "ja".into(),
            ],
            runtime_backend: "llama.cpp".to_string(),
            runtime_format: "gguf".to_string(),
            source_repository: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF".to_string(),
            artifact_url: Some(
                "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf"
                    .to_string(),
            ),
            artifact_filename: "qwen2.5-0.5b-instruct-q4_k_m.gguf".to_string(),
            expected_size_bytes: 491_400_032,
            expected_sha256: Some(
                "74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db".to_string(),
            ),
            license_name: "Apache-2.0".to_string(),
            license_url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct/blob/main/LICENSE"
                .to_string(),
            model_card_url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct".to_string(),
            min_ram_bytes: 2_147_483_648,
            cpu_compatible: true,
            gpu_recommended: false,
            installable: true,
            description: "Compact quantized language model for offline metadata, title, and description generation."
                .to_string(),
            schema_version: 1,
        },
        AiModelManifest {
            id: "opus-mt-en-es".to_string(),
            name: "Opus-MT English to Spanish (Catalog Only)".to_string(),
            family: "opus-mt".to_string(),
            version: "v1.0".to_string(),
            capability: AiModelCapability::Translation,
            supported_languages: vec!["en".into(), "es".into()],
            runtime_backend: "marian".to_string(),
            runtime_format: "onnx".to_string(),
            source_repository: "https://huggingface.co/Helsinki-NLP/opus-mt-en-es".to_string(),
            artifact_url: None, // Catalog only, installation disabled pending runtime
            artifact_filename: "opus-mt-en-es.onnx".to_string(),
            expected_size_bytes: 312_000_000,
            expected_sha256: None,
            license_name: "CC-BY-4.0".to_string(),
            license_url: "https://huggingface.co/Helsinki-NLP/opus-mt-en-es".to_string(),
            model_card_url: "https://huggingface.co/Helsinki-NLP/opus-mt-en-es".to_string(),
            min_ram_bytes: 2_147_483_648,
            cpu_compatible: true,
            gpu_recommended: false,
            installable: false,
            description: "Catalog entry for neural machine translation. Installation disabled pending Marian runtime integration."
                .to_string(),
            schema_version: 1,
        },
    ]
}

/// Finds a model manifest by ID in the curated catalog.
pub fn find_catalog_model(id: &str) -> Option<AiModelManifest> {
    get_curated_model_catalog().into_iter().find(|m| m.id == id)
}
