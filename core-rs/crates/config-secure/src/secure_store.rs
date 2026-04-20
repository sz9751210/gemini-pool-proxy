use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::Engine;
use rand::RngCore;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

use crate::provider::MasterKeyProvider;

#[derive(Debug, Error)]
pub enum SecureConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("encryption error")]
    Encrypt,
    #[error("decryption error")]
    Decrypt,
    #[error("invalid key length")]
    InvalidKey,
    #[error("key provider error: {0}")]
    KeyProvider(String),
}

#[derive(Debug, Clone)]
pub struct SecureConfigStore<P: MasterKeyProvider> {
    key_provider: P,
    config_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedEnvelope {
    version: u8,
    nonce_b64: String,
    ciphertext_b64: String,
}

impl<P: MasterKeyProvider> SecureConfigStore<P> {
    pub fn new(key_provider: P, config_path: impl Into<PathBuf>) -> Self {
        Self {
            key_provider,
            config_path: config_path.into(),
        }
    }

    pub fn save<T: Serialize>(&self, value: &T) -> Result<(), SecureConfigError> {
        let json = serde_json::to_vec(value)?;
        let key = self.load_or_create_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| SecureConfigError::InvalidKey)?;

        let mut nonce = [0_u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), json.as_ref())
            .map_err(|_| SecureConfigError::Encrypt)?;

        let envelope = EncryptedEnvelope {
            version: 1,
            nonce_b64: base64::engine::general_purpose::STANDARD.encode(nonce),
            ciphertext_b64: base64::engine::general_purpose::STANDARD.encode(ciphertext),
        };

        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.config_path, serde_json::to_vec_pretty(&envelope)?)?;
        Ok(())
    }

    pub fn load<T: DeserializeOwned>(&self) -> Result<Option<T>, SecureConfigError> {
        if !self.config_path.exists() {
            return Ok(None);
        }

        let raw = fs::read(&self.config_path)?;
        let envelope: EncryptedEnvelope = serde_json::from_slice(&raw)?;
        let key = self.load_or_create_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| SecureConfigError::InvalidKey)?;

        let nonce = base64::engine::general_purpose::STANDARD
            .decode(envelope.nonce_b64)
            .map_err(|_| SecureConfigError::Decrypt)?;
        let ciphertext = base64::engine::general_purpose::STANDARD
            .decode(envelope.ciphertext_b64)
            .map_err(|_| SecureConfigError::Decrypt)?;

        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| SecureConfigError::Decrypt)?;

        let data = serde_json::from_slice::<T>(&plaintext)?;
        Ok(Some(data))
    }

    fn load_or_create_key(&self) -> Result<Vec<u8>, SecureConfigError> {
        if let Some(existing) = self.key_provider.load_key()? {
            if existing.len() == 32 {
                return Ok(existing);
            }
        }

        let mut key = vec![0_u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        self.key_provider.store_key(&key)?;
        Ok(key)
    }
}

pub fn read_legacy_env(
    path: impl AsRef<Path>,
) -> Result<HashMap<String, String>, SecureConfigError> {
    let content = fs::read_to_string(path)?;
    let mut map = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((k, v)) = trimmed.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
        }
    }

    Ok(map)
}
