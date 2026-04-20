use keyring::Entry;

use crate::secure_store::SecureConfigError;

pub trait MasterKeyProvider: Send + Sync + 'static {
    fn load_key(&self) -> Result<Option<Vec<u8>>, SecureConfigError>;
    fn store_key(&self, key: &[u8]) -> Result<(), SecureConfigError>;
}

#[derive(Debug, Clone)]
pub struct KeyringProvider {
    service: String,
    account: String,
}

impl KeyringProvider {
    pub fn new(service: impl Into<String>, account: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            account: account.into(),
        }
    }

    fn entry(&self) -> Result<Entry, SecureConfigError> {
        Entry::new(&self.service, &self.account)
            .map_err(|e| SecureConfigError::KeyProvider(e.to_string()))
    }
}

impl MasterKeyProvider for KeyringProvider {
    fn load_key(&self) -> Result<Option<Vec<u8>>, SecureConfigError> {
        let entry = self.entry()?;
        match entry.get_password() {
            Ok(raw) => Ok(Some(raw.into_bytes())),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(SecureConfigError::KeyProvider(e.to_string())),
        }
    }

    fn store_key(&self, key: &[u8]) -> Result<(), SecureConfigError> {
        let entry = self.entry()?;
        entry
            .set_password(&String::from_utf8_lossy(key))
            .map_err(|e| SecureConfigError::KeyProvider(e.to_string()))
    }
}
