//! Comprehensive unit and integration tests for AI Model Manager domain engine.

use std::fs;
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::watch;

use opentake_core::ai::catalog::*;
use opentake_core::ai::downloads::*;
use opentake_core::ai::errors::AiError;
use opentake_core::ai::hardware::*;
use opentake_core::ai::models::*;
use opentake_core::ai::runtime::*;
use opentake_core::ai::storage::*;
use opentake_core::ai::verification::*;
use opentake_core::ai::AiModelManager;

// Test helper: computes SHA-256 of in-memory bytes
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[tokio::test]
async fn test_01_model_manifest_schema_validation() {
    let catalog = get_curated_model_catalog();
    assert!(!catalog.is_empty());

    for manifest in catalog {
        assert!(!manifest.id.is_empty());
        assert!(!manifest.name.is_empty());
        assert!(!manifest.family.is_empty());
        assert!(!manifest.version.is_empty());
        assert!(!manifest.runtime_backend.is_empty());
        assert!(!manifest.runtime_format.is_empty());
        assert!(!manifest.source_repository.is_empty());
        assert!(!manifest.license_name.is_empty());
        assert!(manifest.expected_size_bytes > 0);
        assert_eq!(manifest.schema_version, 1);

        if manifest.installable {
            assert!(manifest.artifact_url.is_some());
            assert!(manifest.expected_sha256.is_some());
            let sha = manifest.expected_sha256.as_ref().unwrap();
            assert_eq!(sha.len(), 64, "SHA-256 must be exactly 64 hex characters");
        }
    }
}

#[test]
fn test_02_catalog_loading_offline() {
    let catalog = get_curated_model_catalog();
    assert!(catalog.len() >= 4);

    let tiny = find_catalog_model("whisper-tiny").expect("whisper-tiny exists");
    assert_eq!(tiny.capability, AiModelCapability::SpeechToText);
    assert_eq!(tiny.runtime_backend, "whisper.cpp");
    assert!(tiny.installable);

    let opus = find_catalog_model("opus-mt-en-es").expect("opus-mt-en-es exists");
    assert_eq!(opus.capability, AiModelCapability::Translation);
    assert!(
        !opus.installable,
        "Catalog-only model must not be installable"
    );
}

#[test]
fn test_03_invalid_model_id_rejection() {
    assert!(validate_model_id("whisper-tiny"));
    assert!(validate_model_id("piper_voice-1.0"));
    assert!(validate_model_id("qwen2.5-0.5b"));

    // Traversal & malicious patterns
    assert!(!validate_model_id(""));
    assert!(!validate_model_id("../etc/passwd"));
    assert!(!validate_model_id("model/with/slashes"));
    assert!(!validate_model_id("model\\backslash"));
    assert!(!validate_model_id(".hidden"));
    assert!(!validate_model_id("trail."));
    assert!(!validate_model_id("contains..dotdot"));
    assert!(!validate_model_id(&"a".repeat(100)));
}

#[test]
fn test_04_trusted_url_rules_and_05_untrusted_rejection() {
    // Trusted
    assert!(is_trusted_download_url(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"
    ));
    assert!(is_trusted_download_url(
        "https://cdn-lfs.huggingface.co/repos/1234"
    ));

    // Untrusted / Insecure
    assert!(!is_trusted_download_url("http://huggingface.co/insecure"));
    assert!(!is_trusted_download_url(
        "https://malicious.com/fake-whisper.bin"
    ));
    assert!(!is_trusted_download_url("file:///etc/passwd"));
    assert!(!is_trusted_download_url("ftp://huggingface.co/model"));
    assert!(!is_trusted_download_url("javascript:alert(1)"));
}

#[tokio::test]
async fn test_06_sha256_success_and_07_sha256_mismatch() {
    let temp = tempdir().expect("tempdir");
    let test_file = temp.path().join("model.bin");

    let payload = b"MODEL_WEIGHT_TEST_DATA_VERIFICATION";
    fs::write(&test_file, payload).expect("write");

    let real_sha = sha256_hex(payload);
    let size = payload.len() as u64;

    // Success
    assert!(verify_file_integrity(&test_file, &real_sha, size).is_ok());

    // SHA-256 mismatch
    let bad_sha = "0000000000000000000000000000000000000000000000000000000000000000";
    let res = verify_file_integrity(&test_file, bad_sha, size);
    assert!(matches!(res, Err(AiError::Sha256Mismatch { .. })));
}

#[tokio::test]
async fn test_08_size_mismatch() {
    let temp = tempdir().expect("tempdir");
    let test_file = temp.path().join("model.bin");

    let payload = b"12345678";
    fs::write(&test_file, payload).expect("write");
    let real_sha = sha256_hex(payload);

    let res = verify_file_integrity(&test_file, &real_sha, 9999);
    assert!(matches!(
        res,
        Err(AiError::SizeMismatch {
            expected: 9999,
            actual: 8
        })
    ));
}

// Local mock HTTP server for deterministic network tests
async fn spawn_mock_server(
    payload: Vec<u8>,
    status_code: u16,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    let url = format!("http://127.0.0.1:{}/model.bin", addr.port());

    let handle = tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;

            let response = if status_code == 200 {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    payload.len()
                )
            } else {
                format!("HTTP/1.1 {status_code} Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            };

            let _ = socket.write_all(response.as_bytes()).await;
            if status_code == 200 {
                let _ = socket.write_all(&payload).await;
            }
            let _ = socket.flush().await;
        }
    });

    (url, handle)
}

#[tokio::test]
async fn test_09_download_success_and_atomic_installation() {
    let payload = b"VERIFIED_MOCK_MODEL_WEIGHTS_4096_BYTES".repeat(100);
    let expected_sha = sha256_hex(&payload);
    let expected_size = payload.len() as u64;

    let (mock_url, _server) = spawn_mock_server(payload, 200).await;

    let temp = tempdir().expect("tempdir");
    let storage = AiStorageManager::new(temp.path()).expect("create storage");

    let manifest = AiModelManifest {
        id: "mock-model-1".to_string(),
        name: "Mock Model 1".to_string(),
        family: "mock".to_string(),
        version: "v1".to_string(),
        capability: AiModelCapability::SpeechToText,
        supported_languages: vec!["en".into()],
        runtime_backend: "mock".to_string(),
        runtime_format: "bin".to_string(),
        source_repository: "http://127.0.0.1".to_string(),
        artifact_url: Some(mock_url),
        artifact_filename: "mock_model.bin".to_string(),
        expected_size_bytes: expected_size,
        expected_sha256: Some(expected_sha.clone()),
        additional_artifacts: vec![],
        license_name: "MIT".to_string(),
        license_url: "http://127.0.0.1/license".to_string(),
        model_card_url: "http://127.0.0.1/card".to_string(),
        min_ram_bytes: 1024,
        cpu_compatible: true,
        gpu_recommended: false,
        installable: true,
        description: "Test".to_string(),
        schema_version: 1,
    };

    let staging_path = storage
        .create_staging_path("mock-model-1", "job-1")
        .expect("staging path");
    let (_cancel_tx, cancel_rx) = watch::channel(false);

    let (computed_sha, downloaded_bytes) =
        download_model_artifact(&manifest, &staging_path, cancel_rx, |_progress| {})
            .await
            .expect("download artifact");

    assert_eq!(computed_sha, expected_sha);
    assert_eq!(downloaded_bytes, expected_size);
    assert!(staging_path.exists());

    // Commit atomically
    let inst = AiModelInstallation {
        model_id: "mock-model-1".to_string(),
        installed_at: "2026-10-15T00:00:00Z".to_string(),
        installed_version: "v1".to_string(),
        artifact_filename: "mock_model.bin".to_string(),
        actual_size_bytes: downloaded_bytes,
        verified_sha256: computed_sha,
        installed_artifacts: vec![],
        in_use: false,
    };

    storage
        .commit_installation(&staging_path, inst)
        .expect("commit");

    assert!(
        !staging_path.exists(),
        "Staging path must be moved atomically"
    );
    let installed_file = storage
        .get_artifact_path("mock-model-1", "mock_model.bin")
        .expect("artifact path");
    assert!(installed_file.exists());

    // Verify registry
    let record = storage
        .get_installation("mock-model-1")
        .expect("load")
        .expect("record");
    assert_eq!(record.actual_size_bytes, expected_size);
    assert_eq!(record.verified_sha256, expected_sha);
}

#[tokio::test]
async fn test_10_download_cancellation_cleans_up_staging() {
    let payload = b"LARGE_TEST_PAYLOAD".repeat(5000);
    let (mock_url, _server) = spawn_mock_server(payload, 200).await;

    let temp = tempdir().expect("tempdir");
    let storage = AiStorageManager::new(temp.path()).expect("create storage");

    let manifest = AiModelManifest {
        id: "cancel-model".to_string(),
        name: "Cancel Model".to_string(),
        family: "mock".to_string(),
        version: "v1".to_string(),
        capability: AiModelCapability::SpeechToText,
        supported_languages: vec!["en".into()],
        runtime_backend: "mock".to_string(),
        runtime_format: "bin".to_string(),
        source_repository: "http://127.0.0.1".to_string(),
        artifact_url: Some(mock_url),
        artifact_filename: "cancel.bin".to_string(),
        expected_size_bytes: 90000,
        expected_sha256: Some("any".into()),
        additional_artifacts: vec![],
        license_name: "MIT".to_string(),
        license_url: "http://127.0.0.1".to_string(),
        model_card_url: "http://127.0.0.1".to_string(),
        min_ram_bytes: 1024,
        cpu_compatible: true,
        gpu_recommended: false,
        installable: true,
        description: "Test".to_string(),
        schema_version: 1,
    };

    let staging_path = storage
        .create_staging_path("cancel-model", "job-cancel")
        .expect("staging path");
    let (cancel_tx, cancel_rx) = watch::channel(false);

    // Cancel immediately
    let _ = cancel_tx.send(true);

    let res = download_model_artifact(&manifest, &staging_path, cancel_rx, |_progress| {}).await;

    assert!(matches!(res, Err(AiError::DownloadCancelled)));
    assert!(
        !staging_path.exists(),
        "Cancelled download must clean up staging file"
    );
}

#[tokio::test]
async fn test_11_http_error_handling() {
    let (mock_url, _server) = spawn_mock_server(vec![], 404).await;

    let temp = tempdir().expect("tempdir");
    let storage = AiStorageManager::new(temp.path()).expect("create storage");

    let manifest = AiModelManifest {
        id: "err-model".to_string(),
        name: "Err Model".to_string(),
        family: "mock".to_string(),
        version: "v1".to_string(),
        capability: AiModelCapability::SpeechToText,
        supported_languages: vec!["en".into()],
        runtime_backend: "mock".to_string(),
        runtime_format: "bin".to_string(),
        source_repository: "http://127.0.0.1".to_string(),
        artifact_url: Some(mock_url),
        artifact_filename: "err.bin".to_string(),
        expected_size_bytes: 100,
        expected_sha256: Some("any".into()),
        additional_artifacts: vec![],
        license_name: "MIT".to_string(),
        license_url: "http://127.0.0.1".to_string(),
        model_card_url: "http://127.0.0.1".to_string(),
        min_ram_bytes: 1024,
        cpu_compatible: true,
        gpu_recommended: false,
        installable: true,
        description: "Test".to_string(),
        schema_version: 1,
    };

    let staging_path = storage
        .create_staging_path("err-model", "job-err")
        .expect("staging path");
    let (_cancel_tx, cancel_rx) = watch::channel(false);

    let res = download_model_artifact(&manifest, &staging_path, cancel_rx, |_| {}).await;
    assert!(matches!(res, Err(AiError::HttpError(_))));
    assert!(!staging_path.exists());
}

#[test]
fn test_18_uninstall_success_and_19_in_use_rejection() {
    let temp = tempdir().expect("tempdir");
    let storage = AiStorageManager::new(temp.path()).expect("create storage");

    // Mock installation
    let model_dir = storage.resolve_model_dir("test-model").expect("resolve");
    fs::create_dir_all(&model_dir).expect("mkdir");
    fs::write(model_dir.join("model.bin"), b"MODEL_DATA").expect("write");

    let inst = AiModelInstallation {
        model_id: "test-model".to_string(),
        installed_at: "2026-10-15T00:00:00Z".to_string(),
        installed_version: "v1".to_string(),
        artifact_filename: "model.bin".to_string(),
        actual_size_bytes: 10,
        verified_sha256: "testsha".to_string(),
        installed_artifacts: vec![],
        in_use: false,
    };

    // Staging commit
    let staging = storage.staging_dir().join("dummy.part");
    fs::write(&staging, b"MODEL_DATA").expect("write staging");
    storage.commit_installation(&staging, inst).expect("commit");

    assert!(storage.get_installation("test-model").unwrap().is_some());

    // Lock model (simulate active inference job)
    assert!(storage.acquire_lock("test-model").unwrap());
    assert!(storage.is_locked("test-model"));

    // Attempt uninstall while in use -> must fail!
    let err = storage.remove_model("test-model");
    assert!(matches!(err, Err(AiError::ModelInUse(_))));

    // Release lock
    storage.release_lock("test-model");
    assert!(!storage.is_locked("test-model"));

    // Successful uninstall
    storage.remove_model("test-model").expect("remove");
    assert!(storage.get_installation("test-model").unwrap().is_none());
    assert!(!model_dir.exists(), "Directory must be removed");
}

#[test]
fn test_21_storage_path_traversal_rejection() {
    let temp = tempdir().expect("tempdir");
    let storage = AiStorageManager::new(temp.path()).expect("create storage");

    assert!(matches!(
        storage.resolve_model_dir("../../etc/passwd"),
        Err(AiError::InvalidModelId(_))
    ));
    assert!(matches!(
        storage.get_artifact_path("valid-id", "../../../root/file"),
        Err(AiError::PathTraversalError(_))
    ));
}

#[test]
fn test_24_startup_reconciliation_cleans_orphaned_staging() {
    let temp = tempdir().expect("tempdir");
    let staging = temp.path().join("staging");
    fs::create_dir_all(&staging).expect("mkdir");

    let orphan1 = staging.join("orphan_1.part");
    let orphan2 = staging.join("orphan_2.part");
    fs::write(&orphan1, b"PARTIAL_DATA").expect("write");
    fs::write(&orphan2, b"PARTIAL_DATA").expect("write");

    assert!(orphan1.exists());
    assert!(orphan2.exists());

    // Creating new manager triggers reconcile_startup
    let _storage = AiStorageManager::new(temp.path()).expect("create storage");

    assert!(
        !orphan1.exists(),
        "Orphaned part file must be cleaned on startup"
    );
    assert!(
        !orphan2.exists(),
        "Orphaned part file must be cleaned on startup"
    );
}

#[test]
fn test_27_hardware_detection_and_28_runtime_unavailable() {
    let temp = tempdir().expect("tempdir");
    let hw = detect_hardware(temp.path());

    assert!(!hw.os.is_empty());
    assert!(!hw.arch.is_empty());
    assert!(hw.logical_cpus >= 1);
    assert!(!hw.gpu_backend_installed);
    assert!(!hw.gpu_runtime_usable);

    let manifest = find_catalog_model("whisper-tiny").expect("tiny");
    let compat = evaluate_model_compatibility(&manifest, &hw);

    assert!(
        !compat.runtime_available,
        "Runtime must not be falsely marked available in Step 18"
    );
    assert!(!compat.errors.is_empty());
    assert!(compat.errors[0].contains("is planned for future integration"));
}

#[test]
fn test_32_no_automatic_model_downloads() {
    let temp = tempdir().expect("tempdir");
    let manager = AiModelManager::new(temp.path()).expect("manager");

    let models = manager.list_models().expect("list");
    assert!(!models.is_empty());

    for model in models {
        assert_eq!(
            model.status,
            AiModelStatus::NotInstalled,
            "Fresh installation must have zero automatically downloaded models"
        );
        assert!(model.installation.is_none());
        assert!(model.download_progress.is_none());
    }

    // Storage footprint must be 0 bytes
    let usage = manager.get_storage_usage().expect("usage");
    assert_eq!(usage.total_models_size_bytes, 0);
    assert_eq!(usage.installed_models_count, 0);
    assert_eq!(usage.staging_size_bytes, 0);
}

#[tokio::test]
async fn test_14_duplicate_installation_and_17_corrupted_file_detection() {
    let temp = tempdir().expect("tempdir");
    let manager = AiModelManager::new(temp.path()).expect("manager");

    // Manually install a valid record for whisper-tiny
    let model_dir = manager
        .storage()
        .resolve_model_dir("whisper-tiny")
        .expect("dir");
    fs::create_dir_all(&model_dir).expect("create dir");

    let artifact_path = model_dir.join("ggml-tiny.bin");
    fs::write(&artifact_path, b"TINY_WEIGHT_BYTES").expect("write");

    let inst = AiModelInstallation {
        model_id: "whisper-tiny".to_string(),
        installed_at: "2026-10-15T00:00:00Z".to_string(),
        installed_version: "v1".to_string(),
        artifact_filename: "ggml-tiny.bin".to_string(),
        actual_size_bytes: 17,
        verified_sha256: "dummysha".to_string(),
        installed_artifacts: vec![],
        in_use: false,
    };

    let staging = manager.storage().staging_dir().join("dummy.part");
    fs::write(&staging, b"TINY_WEIGHT_BYTES").expect("write");
    manager
        .storage()
        .commit_installation(&staging, inst)
        .expect("commit");

    // Attempting download while installed must return ModelAlreadyInstalled
    let duplicate_res = manager.start_download("whisper-tiny").await;
    assert!(matches!(
        duplicate_res,
        Err(AiError::ModelAlreadyInstalled(_))
    ));

    // Delete artifact file to simulate corruption/tampering
    fs::remove_file(&artifact_path).expect("delete artifact");

    // Query status -> must detect Corrupted!
    let summary = manager.get_model_status("whisper-tiny").expect("status");
    assert_eq!(summary.status, AiModelStatus::Corrupted);
}

#[test]
fn test_30_large_file_bounded_memory_verification() {
    let temp = tempdir().expect("tempdir");
    let file_path = temp.path().join("large_file.bin");

    // Create a 1 MB file
    let chunk = [0x5au8; 65536];
    let mut file = fs::File::create(&file_path).expect("create");
    for _ in 0..16 {
        use std::io::Write;
        file.write_all(&chunk).expect("write chunk");
    }
    file.sync_all().expect("sync");

    // Verify hashing completes with bounded memory
    let hash = compute_file_sha256(&file_path).expect("hash");
    assert_eq!(hash.len(), 64);
}
