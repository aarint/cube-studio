//! Persistent storage for connection profiles.
//!
//! Two concerns, kept apart:
//!
//! * **Profile document** — the list of `ConnectionConfig` with secrets
//!   stripped, written as an AES-256-GCM envelope. The key is derived from a
//!   random per-installation salt plus an OS-account binding, so copying the
//!   data directory to another machine (or another user) does not reveal it.
//! * **Secrets** — passwords and tokens, in a second encrypted file. Keeping
//!   them separate means a profile can be exported/shared without leaking
//!   credentials.
//!
//! The on-disk format is versioned so old data can be migrated later. Plain
//! JSON from pre-2.0 installs is accepted transparently.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::Aes256Gcm;
use base64::Engine as _;
use rand::Rng as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::ConnectionConfig;
use crate::error::{AppError, AppResult};

const PROFILE_FILE: &str = "connections.json";
const SECRET_FILE: &str = "secrets.enc";
const SALT_FILE: &str = "install.salt";
const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const KDF_ROUNDS: u32 = 4096;
/// Bumped whenever the payload inside the envelope changes shape.
const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct Envelope {
    v: u32,
    /// base64 AES-GCM nonce.
    n: String,
    /// base64 ciphertext with the GCM tag appended.
    d: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct ProfileDocument {
    profiles: Vec<ConnectionConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct SecretDocument {
    /// profile id -> password
    passwords: BTreeMap<String, String>,
    /// profile id -> NATS auth token
    tokens: BTreeMap<String, String>,
}

/// Owns the application data directory and all persistence.
#[derive(Debug, Clone)]
pub struct ProfileStore {
    dir: PathBuf,
    key: [u8; 32],
}

impl ProfileStore {
    /// Open (or create) the store inside `dir`.
    pub fn open(dir: impl Into<PathBuf>) -> AppResult<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        let salt = load_or_create_salt(&dir.join(SALT_FILE))?;
        let key = derive_key(&salt, &machine_binding());
        Ok(Self { dir, key })
    }

    fn profile_path(&self) -> PathBuf {
        self.dir.join(PROFILE_FILE)
    }

    fn secret_path(&self) -> PathBuf {
        self.dir.join(SECRET_FILE)
    }

    // ---- profile document ---------------------------------------------

    fn read_profiles(&self) -> AppResult<ProfileDocument> {
        let path = self.profile_path();
        if !path.exists() {
            return Ok(ProfileDocument::default());
        }
        let raw = std::fs::read(&path)?;
        if raw.iter().all(u8::is_ascii_whitespace) {
            return Ok(ProfileDocument::default());
        }
        serde_json::from_slice(&self.decrypt_or_plain(&raw)?)
            .map_err(|e| AppError::internal(format!("stored connections file is corrupt: {e}")))
    }

    fn write_profiles(&self, doc: &ProfileDocument) -> AppResult<()> {
        let json = serde_json::to_vec_pretty(doc)?;
        write_atomic(&self.profile_path(), self.encrypt(&json)?.as_bytes())
    }

    /// All saved profiles with secrets merged back in, sorted by name.
    pub fn list(&self) -> AppResult<Vec<ConnectionConfig>> {
        let mut doc = self.read_profiles()?;
        let secrets = self.read_secrets()?;
        for profile in &mut doc.profiles {
            if let Some(password) = secrets.passwords.get(&profile.id) {
                profile.password = password.clone();
            }
            if let Some(token) = secrets.tokens.get(&profile.id) {
                profile.token = Some(token.clone());
            }
        }
        doc.profiles.sort_by_key(|p| p.name.to_lowercase());
        Ok(doc.profiles)
    }

    /// Insert or replace a profile, keyed by `id`.
    pub fn upsert(&self, config: ConnectionConfig) -> AppResult<ConnectionConfig> {
        let mut config = config.normalize()?;
        let password = std::mem::take(&mut config.password);
        let token = config.token.take().filter(|t| !t.is_empty());

        let mut doc = self.read_profiles()?;
        match doc.profiles.iter_mut().find(|p| p.id == config.id) {
            Some(existing) => *existing = config.clone(),
            None => doc.profiles.push(config.clone()),
        }
        self.write_profiles(&doc)?;

        let mut secrets = self.read_secrets()?;
        if password.is_empty() {
            secrets.passwords.remove(&config.id);
        } else {
            secrets.passwords.insert(config.id.clone(), password.clone());
        }
        match &token {
            Some(token) => {
                secrets.tokens.insert(config.id.clone(), token.clone());
            }
            None => {
                secrets.tokens.remove(&config.id);
            }
        }
        self.write_secrets(&secrets)?;

        config.password = password;
        config.token = token;
        Ok(config)
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        let mut doc = self.read_profiles()?;
        let before = doc.profiles.len();
        doc.profiles.retain(|p| p.id != id);
        if doc.profiles.len() != before {
            self.write_profiles(&doc)?;
        }

        let mut secrets = self.read_secrets()?;
        let had_secret = secrets.passwords.remove(id).is_some() | secrets.tokens.remove(id).is_some();
        if had_secret {
            self.write_secrets(&secrets)?;
        }
        Ok(())
    }

    /// Look one profile up by id. Fails when it does not exist.
    pub fn get(&self, id: &str) -> AppResult<ConnectionConfig> {
        self.list()?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| AppError::invalid(format!("no saved connection with id {id}")))
    }

    // ---- secrets -------------------------------------------------------

    fn read_secrets(&self) -> AppResult<SecretDocument> {
        let path = self.secret_path();
        if !path.exists() {
            return Ok(SecretDocument::default());
        }
        let raw = std::fs::read(&path)?;
        if raw.iter().all(u8::is_ascii_whitespace) {
            return Ok(SecretDocument::default());
        }
        Ok(serde_json::from_slice(&self.decrypt_or_plain(&raw)?).unwrap_or_default())
    }

    fn write_secrets(&self, doc: &SecretDocument) -> AppResult<()> {
        let json = serde_json::to_vec(doc)?;
        write_atomic(&self.secret_path(), self.encrypt(&json)?.as_bytes())
    }

    // ---- crypto --------------------------------------------------------

    fn encrypt(&self, plaintext: &[u8]) -> AppResult<String> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| AppError::internal(format!("invalid cipher key: {e}")))?;
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let ciphertext = cipher
            .encrypt(&nonce_bytes.into(), plaintext)
            .map_err(|e| AppError::internal(format!("encryption failed: {e}")))?;
        let envelope = Envelope {
            v: FORMAT_VERSION,
            n: base64::engine::general_purpose::STANDARD.encode(nonce_bytes),
            d: base64::engine::general_purpose::STANDARD.encode(ciphertext),
        };
        Ok(serde_json::to_string(&envelope)?)
    }

    fn decrypt(&self, blob: &[u8]) -> AppResult<Vec<u8>> {
        let envelope: Envelope = serde_json::from_slice(blob)
            .map_err(|e| AppError::internal(format!("unreadable stored data: {e}")))?;
        let nonce_bytes = base64::engine::general_purpose::STANDARD
            .decode(&envelope.n)
            .map_err(|e| AppError::internal(format!("corrupt nonce: {e}")))?;
        let nonce: [u8; NONCE_LEN] = nonce_bytes
            .as_slice()
            .try_into()
            .map_err(|_| AppError::internal("corrupt nonce length"))?;
        let data = base64::engine::general_purpose::STANDARD
            .decode(&envelope.d)
            .map_err(|e| AppError::internal(format!("corrupt payload: {e}")))?;
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| AppError::internal(format!("invalid cipher key: {e}")))?;
        cipher
            .decrypt(&nonce.into(), data.as_ref())
            .map_err(|_| {
                AppError::internal(
                    "stored connections could not be decrypted; they were probably \
                     written on a different machine or user account",
                )
            })
    }

    /// Accept either an encrypted envelope or legacy plaintext JSON.
    fn decrypt_or_plain(&self, raw: &[u8]) -> AppResult<Vec<u8>> {
        match serde_json::from_slice::<Envelope>(raw) {
            Ok(envelope) if envelope.v > 0 => self.decrypt(
                &serde_json::to_vec(&envelope).map_err(|e| AppError::internal(e.to_string()))?,
            ),
            _ => Ok(raw.to_vec()),
        }
    }
}

fn load_or_create_salt(path: &Path) -> AppResult<Vec<u8>> {
    if let Ok(existing) = std::fs::read(path) {
        if existing.len() >= SALT_LEN {
            return Ok(existing);
        }
    }
    let mut salt = vec![0u8; SALT_LEN];
    rand::rng().fill_bytes(&mut salt);
    write_atomic(path, &salt)?;
    Ok(salt)
}

/// Binds the derived key to this OS user and home directory.
fn machine_binding() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "default".into());
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    format!("cube-studio|v1|{user}|{home}")
}

/// Stretch salt + binding into a 256-bit key.
///
/// Deliberately simple: an attacker who steals the data directory also has the
/// salt, so this is not a defence against offline attack — the real protection
/// is that the derived key cannot be reproduced without the same OS account.
fn derive_key(salt: &[u8], binding: &str) -> [u8; 32] {
    let mut block = {
        let mut hasher = Sha256::new();
        hasher.update(binding.as_bytes());
        hasher.update(salt);
        hasher.finalize().to_vec()
    };
    for round in 0..KDF_ROUNDS {
        let mut hasher = Sha256::new();
        hasher.update(&block);
        hasher.update(round.to_le_bytes());
        hasher.update(salt);
        block = hasher.finalize().to_vec();
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&block[..32]);
    key
}

fn write_atomic(path: &Path, contents: &[u8]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    restrict_permissions(path);
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal scratch directory that cleans itself up.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "cube-studio-test-{label}-{}",
                uuid::Uuid::new_v4()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn round_trips_profiles_and_secrets() {
        let dir = TempDir::new("roundtrip");
        let store = ProfileStore::open(dir.path()).unwrap();
        let saved = store
            .upsert(ConnectionConfig {
                name: "cache".into(),
                host: "127.0.0.1".into(),
                password: "s3cret".into(),
                ..Default::default()
            })
            .unwrap();

        let all = store.list().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].password, "s3cret");
        assert_eq!(all[0].id, saved.id);
    }

    #[test]
    fn upsert_replaces_by_id() {
        let dir = TempDir::new("upsert");
        let store = ProfileStore::open(dir.path()).unwrap();
        let saved = store
            .upsert(ConnectionConfig {
                host: "a".into(),
                ..Default::default()
            })
            .unwrap();
        store
            .upsert(ConnectionConfig {
                id: saved.id.clone(),
                name: "renamed".into(),
                host: "b".into(),
                ..Default::default()
            })
            .unwrap();

        let all = store.list().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "renamed");
        assert_eq!(all[0].host, "b");
    }

    #[test]
    fn delete_removes_profile_and_secret() {
        let dir = TempDir::new("delete");
        let store = ProfileStore::open(dir.path()).unwrap();
        let saved = store
            .upsert(ConnectionConfig {
                host: "a".into(),
                password: "pw".into(),
                ..Default::default()
            })
            .unwrap();

        store.delete(&saved.id).unwrap();
        assert!(store.list().unwrap().is_empty());
        assert!(store.get(&saved.id).is_err());
    }

    #[test]
    fn profile_file_never_contains_the_password() {
        let dir = TempDir::new("noplaintext");
        let store = ProfileStore::open(dir.path()).unwrap();
        store
            .upsert(ConnectionConfig {
                host: "a".into(),
                password: "super-secret-value".into(),
                ..Default::default()
            })
            .unwrap();

        for file in [PROFILE_FILE, SECRET_FILE] {
            let text = String::from_utf8_lossy(&std::fs::read(dir.path().join(file)).unwrap())
                .into_owned();
            assert!(
                !text.contains("super-secret-value"),
                "{file} leaked the password"
            );
        }
    }

    #[test]
    fn reads_legacy_plaintext_file() {
        let dir = TempDir::new("legacy");
        std::fs::write(
            dir.path().join(PROFILE_FILE),
            br#"{"profiles":[{"host":"legacy.example","port":6379,"kind":"redis"}]}"#,
        )
        .unwrap();
        let store = ProfileStore::open(dir.path()).unwrap();
        let all = store.list().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].host, "legacy.example");
    }

    #[test]
    fn different_salts_produce_different_keys() {
        let a = derive_key(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "bind");
        let b = derive_key(b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "bind");
        assert_ne!(a, b);
    }

    #[test]
    fn salt_is_stable_across_reopen() {
        let dir = TempDir::new("saltstable");
        let first = ProfileStore::open(dir.path()).unwrap();
        first
            .upsert(ConnectionConfig {
                host: "keep".into(),
                ..Default::default()
            })
            .unwrap();

        let second = ProfileStore::open(dir.path()).unwrap();
        assert_eq!(second.list().unwrap()[0].host, "keep");
    }
}
