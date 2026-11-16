//! Immutable bundles stored only in the operating system credential store.
use base64::{engine::general_purpose::STANDARD, Engine};
use custos_core::contracts::credentials::{
    credential_part_count, CredentialLocation, CredentialVault, CredentialVaultError as Error,
    CREDENTIAL_SEGMENT_SIZE, MAX_CREDENTIAL_BYTES,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Default)]
pub struct KeyringCredentialVault;

#[derive(Serialize, Deserialize)]
struct Manifest {
    version: u8,
    vault: String,
    account: String,
    generation: String,
    parts: usize,
    length: usize,
    digest: String,
}

impl KeyringCredentialVault {
    fn entry(location: &CredentialLocation, suffix: &str) -> Result<keyring::Entry, Error> {
        if ![&location.vault_id, &location.generation]
            .iter()
            .all(|value| {
                value.len() <= 64
                    && !value.is_empty()
                    && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            })
        {
            return Err(Error::Invalid);
        }
        let account = format!("{:x}", Sha256::digest(location.account_id.as_bytes()));
        let service = format!("custos.provider.{}.{}", location.vault_id, account);
        keyring::Entry::new(&service, &format!("{}.{}", location.generation, suffix))
            .map_err(|_| Error::Unavailable)
    }
    fn password(location: &CredentialLocation, suffix: &str) -> Result<Option<String>, Error> {
        match Self::entry(location, suffix)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(Error::Unavailable),
        }
    }
}

impl CredentialVault for KeyringCredentialVault {
    fn write(&self, location: &CredentialLocation, secret: &[u8]) -> Result<(), Error> {
        if secret.is_empty() || secret.len() > MAX_CREDENTIAL_BYTES {
            return Err(Error::Invalid);
        }
        // A manifest is written last and is the completion marker for recovery.
        if Self::password(location, "manifest")?.is_some() {
            return Err(Error::SaveFailed);
        }
        let encoded = STANDARD.encode(secret);
        for (index, chunk) in encoded
            .as_bytes()
            .chunks(CREDENTIAL_SEGMENT_SIZE)
            .enumerate()
        {
            let value = std::str::from_utf8(chunk).map_err(|_| Error::Invalid)?;
            Self::entry(location, &index.to_string())?
                .set_password(value)
                .map_err(|_| Error::Unavailable)?;
        }
        let manifest = Manifest {
            version: 1,
            vault: location.vault_id.clone(),
            account: format!("{:x}", Sha256::digest(location.account_id.as_bytes())),
            generation: location.generation.clone(),
            parts: credential_part_count(secret.len()),
            length: secret.len(),
            digest: format!("{:x}", Sha256::digest(secret)),
        };
        let json = serde_json::to_string(&manifest).map_err(|_| Error::Invalid)?;
        Self::entry(location, "manifest")?
            .set_password(&json)
            .map_err(|_| Error::Unavailable)?;
        Ok(())
    }
    fn read(&self, location: &CredentialLocation) -> Result<Option<Vec<u8>>, Error> {
        let Some(json) = Self::password(location, "manifest")? else {
            return Ok(None);
        };
        if json.len() > 1024 {
            return Err(Error::Corrupt);
        }
        let manifest: Manifest = serde_json::from_str(&json).map_err(|_| Error::Corrupt)?;
        if manifest.version != 1
            || manifest.vault != location.vault_id
            || manifest.generation != location.generation
            || manifest.account != format!("{:x}", Sha256::digest(location.account_id.as_bytes()))
            || manifest.length == 0
            || manifest.length > MAX_CREDENTIAL_BYTES
            || manifest.parts != credential_part_count(manifest.length)
        {
            return Err(Error::Corrupt);
        }
        let mut encoded = String::new();
        for index in 0..manifest.parts {
            let part = Self::password(location, &index.to_string())?.ok_or(Error::Corrupt)?;
            if part.len() > CREDENTIAL_SEGMENT_SIZE || !part.is_ascii() {
                return Err(Error::Corrupt);
            }
            encoded.push_str(&part);
        }
        let bytes = STANDARD.decode(encoded).map_err(|_| Error::Corrupt)?;
        if bytes.len() != manifest.length
            || format!("{:x}", Sha256::digest(&bytes)) != manifest.digest
        {
            return Err(Error::Corrupt);
        }
        Ok(Some(bytes))
    }
    fn delete(&self, location: &CredentialLocation, parts: usize) -> Result<(), Error> {
        if parts > credential_part_count(MAX_CREDENTIAL_BYTES) {
            return Err(Error::Invalid);
        }
        for suffix in
            std::iter::once("manifest".to_string()).chain((0..parts).map(|index| index.to_string()))
        {
            match Self::entry(location, &suffix)?.delete_password() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(_) => return Err(Error::CleanupPending),
            }
        }
        Ok(())
    }
}

use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct FileCredentialVault {
    base_dir: PathBuf,
}

impl FileCredentialVault {
    pub fn new(base_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&base_dir);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&base_dir, fs::Permissions::from_mode(0o700));
        }
        Self { base_dir }
    }

    pub fn default_dir() -> PathBuf {
        if let Ok(path) = std::env::var("CUSTOS_VAULT_DIR") {
            return PathBuf::from(path);
        }
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".custos").join("vault")
    }

    fn entry_path(&self, location: &CredentialLocation, suffix: &str) -> Result<PathBuf, Error> {
        if ![&location.vault_id, &location.generation]
            .iter()
            .all(|value| {
                value.len() <= 64
                    && !value.is_empty()
                    && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            })
        {
            return Err(Error::Invalid);
        }
        let account = format!("{:x}", Sha256::digest(location.account_id.as_bytes()));
        let dir = self.base_dir.join(&location.vault_id).join(&account);
        fs::create_dir_all(&dir).map_err(|_| Error::Unavailable)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
        }
        Ok(dir.join(format!("{}.{}.dat", location.generation, suffix)))
    }
}

impl CredentialVault for FileCredentialVault {
    fn write(&self, location: &CredentialLocation, secret: &[u8]) -> Result<(), Error> {
        if secret.is_empty() || secret.len() > MAX_CREDENTIAL_BYTES {
            return Err(Error::Invalid);
        }
        let manifest_path = self.entry_path(location, "manifest")?;
        if manifest_path.exists() {
            return Err(Error::SaveFailed);
        }
        let encoded = STANDARD.encode(secret);
        for (index, chunk) in encoded
            .as_bytes()
            .chunks(CREDENTIAL_SEGMENT_SIZE)
            .enumerate()
        {
            let value = std::str::from_utf8(chunk).map_err(|_| Error::Invalid)?;
            let part_path = self.entry_path(location, &index.to_string())?;
            fs::write(&part_path, value.as_bytes()).map_err(|_| Error::Unavailable)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&part_path, fs::Permissions::from_mode(0o600));
            }
        }
        let manifest = Manifest {
            version: 1,
            vault: location.vault_id.clone(),
            account: format!("{:x}", Sha256::digest(location.account_id.as_bytes())),
            generation: location.generation.clone(),
            parts: credential_part_count(secret.len()),
            length: secret.len(),
            digest: format!("{:x}", Sha256::digest(secret)),
        };
        let json = serde_json::to_string(&manifest).map_err(|_| Error::Invalid)?;
        fs::write(&manifest_path, json.as_bytes()).map_err(|_| Error::Unavailable)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&manifest_path, fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    fn read(&self, location: &CredentialLocation) -> Result<Option<Vec<u8>>, Error> {
        let manifest_path = self.entry_path(location, "manifest")?;
        if !manifest_path.exists() {
            return Ok(None);
        }
        let json = fs::read_to_string(&manifest_path).map_err(|_| Error::Corrupt)?;
        if json.len() > 1024 {
            return Err(Error::Corrupt);
        }
        let manifest: Manifest = serde_json::from_str(&json).map_err(|_| Error::Corrupt)?;
        if manifest.version != 1
            || manifest.vault != location.vault_id
            || manifest.generation != location.generation
            || manifest.account != format!("{:x}", Sha256::digest(location.account_id.as_bytes()))
            || manifest.length == 0
            || manifest.length > MAX_CREDENTIAL_BYTES
            || manifest.parts != credential_part_count(manifest.length)
        {
            return Err(Error::Corrupt);
        }
        let mut encoded = String::new();
        for index in 0..manifest.parts {
            let part_path = self.entry_path(location, &index.to_string())?;
            if !part_path.exists() {
                return Err(Error::Corrupt);
            }
            let part = fs::read_to_string(&part_path).map_err(|_| Error::Corrupt)?;
            if part.len() > CREDENTIAL_SEGMENT_SIZE || !part.is_ascii() {
                return Err(Error::Corrupt);
            }
            encoded.push_str(&part);
        }
        let bytes = STANDARD.decode(encoded).map_err(|_| Error::Corrupt)?;
        if bytes.len() != manifest.length
            || format!("{:x}", Sha256::digest(&bytes)) != manifest.digest
        {
            return Err(Error::Corrupt);
        }
        Ok(Some(bytes))
    }

    fn delete(&self, location: &CredentialLocation, parts: usize) -> Result<(), Error> {
        if parts > credential_part_count(MAX_CREDENTIAL_BYTES) {
            return Err(Error::Invalid);
        }
        for suffix in
            std::iter::once("manifest".to_string()).chain((0..parts).map(|index| index.to_string()))
        {
            if let Ok(path) = self.entry_path(location, &suffix) {
                let _ = fs::remove_file(path);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Writes synthetic credentials to an isolated OS keyring namespace"]
    fn native_keyring_long_unicode_bundle_corruption_and_cleanup() {
        let location = CredentialLocation {
            vault_id: uuid::Uuid::new_v4().to_string(),
            account_id: "synthetic-smoke-account".into(),
            generation: uuid::Uuid::new_v4().to_string(),
        };
        let bytes = "synthetic-token-日本語-".repeat(600).into_bytes();
        struct Cleanup(CredentialLocation, usize);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = KeyringCredentialVault.delete(&self.0, self.1);
            }
        }
        let _cleanup = Cleanup(location.clone(), credential_part_count(bytes.len()));
        KeyringCredentialVault.write(&location, &bytes).unwrap();
        assert_eq!(
            KeyringCredentialVault.read(&location).unwrap().unwrap(),
            bytes
        );
        KeyringCredentialVault::entry(&location, "0")
            .unwrap()
            .set_password("corrupt-part")
            .unwrap();
        assert!(matches!(
            KeyringCredentialVault.read(&location),
            Err(Error::Corrupt)
        ));
        KeyringCredentialVault
            .delete(&location, credential_part_count(bytes.len()))
            .unwrap();
        KeyringCredentialVault
            .delete(&location, credential_part_count(bytes.len()))
            .unwrap();
        assert!(KeyringCredentialVault.read(&location).unwrap().is_none());
    }
}
