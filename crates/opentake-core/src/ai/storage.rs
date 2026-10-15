//! Shared AI Model Storage and Repository Management.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use super::errors::AiError;
use super::models::{validate_model_id, AiModelInstallation, AiStorageUsage};
use super::verification::verify_file_integrity;

/// Persistent registry of installed models.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct ModelRegistry {
    pub schema_version: u32,
    pub installations: HashMap<String, AiModelInstallation>,
}

/// Thread-safe storage manager overseeing the shared AI model repository.
#[derive(Clone)]
pub struct AiStorageManager {
    base_dir: PathBuf,
    installed_dir: PathBuf,
    staging_dir: PathBuf,
    registry_file: PathBuf,
    active_locks: Arc<RwLock<HashSet<String>>>,
}

impl AiStorageManager {
    /// Initializes a storage manager rooted at `base_dir`, ensuring directory structure exists.
    pub fn new(base_dir: impl Into<PathBuf>) -> Result<Self, AiError> {
        let base_dir = base_dir.into();
        let installed_dir = base_dir.join("installed");
        let staging_dir = base_dir.join("staging");
        let registry_file = base_dir.join("registry.json");

        fs::create_dir_all(&installed_dir)
            .map_err(|e| AiError::StorageError(format!("Failed to create installed dir: {e}")))?;
        fs::create_dir_all(&staging_dir)
            .map_err(|e| AiError::StorageError(format!("Failed to create staging dir: {e}")))?;

        let manager = Self {
            base_dir,
            installed_dir,
            staging_dir,
            registry_file,
            active_locks: Arc::new(RwLock::new(HashSet::new())),
        };

        manager.reconcile_startup()?;
        Ok(manager)
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn staging_dir(&self) -> &Path {
        &self.staging_dir
    }

    pub fn installed_dir(&self) -> &Path {
        &self.installed_dir
    }

    /// Verifies that a model path is safe, properly formatted, and strictly contained within storage.
    pub fn resolve_model_dir(&self, model_id: &str) -> Result<PathBuf, AiError> {
        if !validate_model_id(model_id) {
            return Err(AiError::InvalidModelId(model_id.to_string()));
        }

        let target = self.installed_dir.join(model_id);
        // Canonicalize base path to verify containment
        let canonical_base = self.installed_dir.canonicalize().map_err(|e| {
            AiError::StorageError(format!("Failed to canonicalize installed dir: {e}"))
        })?;

        if target.exists() {
            let canonical_target = target.canonicalize().map_err(|e| {
                AiError::StorageError(format!("Failed to canonicalize model path: {e}"))
            })?;
            if !canonical_target.starts_with(&canonical_base) {
                return Err(AiError::PathTraversalError(format!(
                    "Model dir escapes installed repository: {model_id}"
                )));
            }
            // Check for symlinks
            let meta = fs::symlink_metadata(&target).map_err(|e| {
                AiError::StorageError(format!("Failed to read symlink metadata: {e}"))
            })?;
            if meta.file_type().is_symlink() {
                return Err(AiError::PathTraversalError(format!(
                    "Symlinks are rejected: {model_id}"
                )));
            }
        }

        Ok(target)
    }

    /// Returns the path to the artifact file for a given model.
    pub fn get_artifact_path(&self, model_id: &str, filename: &str) -> Result<PathBuf, AiError> {
        let model_dir = self.resolve_model_dir(model_id)?;
        if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
            return Err(AiError::PathTraversalError(format!(
                "Invalid artifact filename: {filename}"
            )));
        }
        Ok(model_dir.join(filename))
    }

    /// Generates a staging path for a model download job.
    pub fn create_staging_path(&self, model_id: &str, job_id: &str) -> Result<PathBuf, AiError> {
        if !validate_model_id(model_id) || !validate_model_id(job_id) {
            return Err(AiError::InvalidModelId(format!("{model_id}_{job_id}")));
        }
        Ok(self.staging_dir.join(format!("{model_id}_{job_id}.part")))
    }

    /// Reconciles storage state upon startup: cleans up staging debris and checks installed records.
    pub fn reconcile_startup(&self) -> Result<(), AiError> {
        // Remove orphaned .part files in staging
        if let Ok(entries) = fs::read_dir(&self.staging_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "part") {
                    let _ = fs::remove_file(path);
                }
            }
        }

        // Validate registry entries against disk
        let mut registry = self.load_registry()?;
        let mut modified = false;

        registry.installations.retain(|id, inst| {
            let path = self.installed_dir.join(id).join(&inst.artifact_filename);
            let exists = path.is_file();
            if !exists {
                modified = true;
            }
            exists
        });

        if modified {
            self.save_registry(&registry)?;
        }

        Ok(())
    }

    /// Loads the persistent registry from disk.
    fn load_registry(&self) -> Result<ModelRegistry, AiError> {
        if !self.registry_file.exists() {
            return Ok(ModelRegistry {
                schema_version: 1,
                installations: HashMap::new(),
            });
        }

        let content = fs::read_to_string(&self.registry_file)
            .map_err(|e| AiError::StorageError(format!("Failed to read registry: {e}")))?;

        serde_json::from_str(&content)
            .map_err(|e| AiError::StorageError(format!("Corrupt registry JSON, schema error: {e}")))
    }

    /// Saves the registry atomically using a temporary file and atomic rename.
    fn save_registry(&self, registry: &ModelRegistry) -> Result<(), AiError> {
        let serialized = serde_json::to_string_pretty(registry)
            .map_err(|e| AiError::StorageError(format!("Failed to serialize registry: {e}")))?;

        let temp_file = self.base_dir.join("registry.json.tmp");
        {
            let mut file = File::create(&temp_file).map_err(|e| {
                AiError::StorageError(format!("Failed to create temp registry: {e}"))
            })?;
            file.write_all(serialized.as_bytes()).map_err(|e| {
                AiError::StorageError(format!("Failed to write temp registry: {e}"))
            })?;
            file.sync_all()
                .map_err(|e| AiError::StorageError(format!("Failed to sync temp registry: {e}")))?;
        }

        fs::rename(&temp_file, &self.registry_file).map_err(|e| {
            AiError::StorageError(format!("Failed to commit registry atomically: {e}"))
        })?;

        Ok(())
    }

    /// Retrieves an installed model record by ID if present and valid.
    pub fn get_installation(&self, model_id: &str) -> Result<Option<AiModelInstallation>, AiError> {
        let registry = self.load_registry()?;
        Ok(registry.installations.get(model_id).cloned())
    }

    /// Convenience check whether a model ID is registered as installed.
    pub fn is_installed(&self, model_id: &str) -> bool {
        self.get_installation(model_id).ok().flatten().is_some()
    }

    /// Lists all installed model records.
    pub fn list_installations(&self) -> Result<Vec<AiModelInstallation>, AiError> {
        let registry = self.load_registry()?;
        Ok(registry.installations.into_values().collect())
    }

    /// Atomically commits a downloaded artifact into the installed repository and updates the registry.
    pub fn commit_installation(
        &self,
        staging_path: &Path,
        installation: AiModelInstallation,
    ) -> Result<(), AiError> {
        let model_dir = self.resolve_model_dir(&installation.model_id)?;
        fs::create_dir_all(&model_dir)
            .map_err(|e| AiError::StorageError(format!("Failed to create model dir: {e}")))?;

        let dest_path = model_dir.join(&installation.artifact_filename);
        fs::rename(staging_path, &dest_path).map_err(|e| {
            AiError::StorageError(format!(
                "Failed to move staged artifact to destination {}: {e}",
                dest_path.display()
            ))
        })?;

        let mut registry = self.load_registry()?;
        registry
            .installations
            .insert(installation.model_id.clone(), installation);
        self.save_registry(&registry)?;

        Ok(())
    }

    /// Verifies an installed model's integrity against expected SHA-256 and size.
    pub fn verify_installed_model(
        &self,
        model_id: &str,
        expected_sha256: &str,
        expected_size_bytes: u64,
    ) -> Result<bool, AiError> {
        let inst = self
            .get_installation(model_id)?
            .ok_or_else(|| AiError::ModelNotFound(model_id.to_string()))?;

        let artifact_path = self.get_artifact_path(model_id, &inst.artifact_filename)?;
        match verify_file_integrity(&artifact_path, expected_sha256, expected_size_bytes) {
            Ok(()) => Ok(true),
            Err(e) => Err(e),
        }
    }

    /// Verifies all artifacts of an installed model (including additional configuration artifacts)
    /// against the manifest requirements.
    pub fn verify_installed_artifacts(
        &self,
        manifest: &super::models::AiModelManifest,
    ) -> Result<bool, AiError> {
        let inst = self
            .get_installation(&manifest.id)?
            .ok_or_else(|| AiError::ModelNotFound(manifest.id.clone()))?;

        // 1. Verify primary artifact
        if let Some(expected_sha256) = &manifest.expected_sha256 {
            let primary_path = self.get_artifact_path(&manifest.id, &inst.artifact_filename)?;
            verify_file_integrity(&primary_path, expected_sha256, manifest.expected_size_bytes)?;
        }

        // 2. Verify each additional artifact
        for artifact in &manifest.additional_artifacts {
            let artifact_path = self.get_artifact_path(&manifest.id, &artifact.filename)?;
            if !artifact_path.exists() {
                return Err(AiError::CorruptedModel(format!(
                    "Required artifact {} missing for model {}",
                    artifact.filename, manifest.id
                )));
            }
            if let Some(expected_sha256) = &artifact.expected_sha256 {
                verify_file_integrity(
                    &artifact_path,
                    expected_sha256,
                    artifact.expected_size_bytes,
                )?;
            }
        }

        Ok(true)
    }

    /// Acquires an in-use lock on a model to prevent uninstallation while an inference job runs.
    pub fn acquire_lock(&self, model_id: &str) -> Result<bool, AiError> {
        let mut locks = self.active_locks.write().map_err(|_| {
            AiError::StorageError("Failed to acquire write lock on active locks".into())
        })?;
        Ok(locks.insert(model_id.to_string()))
    }

    /// Releases an in-use lock on a model.
    pub fn release_lock(&self, model_id: &str) {
        if let Ok(mut locks) = self.active_locks.write() {
            locks.remove(model_id);
        }
    }

    /// Checks if a model is currently locked in use.
    pub fn is_locked(&self, model_id: &str) -> bool {
        self.active_locks
            .read()
            .map(|locks| locks.contains(model_id))
            .unwrap_or(false)
    }

    /// Safely removes an installed model from storage and registry.
    pub fn remove_model(&self, model_id: &str) -> Result<(), AiError> {
        if self.is_locked(model_id) {
            return Err(AiError::ModelInUse(model_id.to_string()));
        }

        let model_dir = self.resolve_model_dir(model_id)?;
        if model_dir.exists() {
            fs::remove_dir_all(&model_dir).map_err(|e| {
                AiError::StorageError(format!(
                    "Failed to delete model directory {}: {e}",
                    model_dir.display()
                ))
            })?;
        }

        let mut registry = self.load_registry()?;
        registry.installations.remove(model_id);
        self.save_registry(&registry)?;

        Ok(())
    }

    /// Computes accurate storage usage across installed models and staging directories.
    pub fn compute_storage_usage(&self) -> Result<AiStorageUsage, AiError> {
        let mut total_models_size_bytes: u64 = 0;
        let mut installed_models_count: usize = 0;
        let mut staging_size_bytes: u64 = 0;

        let registry = self.load_registry()?;
        for inst in registry.installations.values() {
            installed_models_count += 1;
            total_models_size_bytes =
                total_models_size_bytes.saturating_add(inst.actual_size_bytes);
        }

        if let Ok(entries) = fs::read_dir(&self.staging_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        staging_size_bytes = staging_size_bytes.saturating_add(meta.len());
                    }
                }
            }
        }

        let hardware = super::hardware::detect_hardware(&self.base_dir);

        Ok(AiStorageUsage {
            base_dir: self.base_dir.to_string_lossy().to_string(),
            total_models_size_bytes,
            installed_models_count,
            staging_size_bytes,
            available_disk_bytes: hardware.storage_available_bytes,
        })
    }
}
