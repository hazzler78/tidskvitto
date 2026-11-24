use tidskvitto_core::Database;

const SERVICE: &str = "tidskvitto";
const USER: &str = "ai_api_key";

pub struct StoredKey {
    pub value: Option<String>,
    pub storage: String,
}

pub fn load_api_key(db: &Database) -> StoredKey {
    if let Some(value) = keyring_get() {
        return StoredKey {
            value: Some(value),
            storage: "keyring".into(),
        };
    }
    if let Some(value) = db.file_api_key() {
        return StoredKey {
            value: Some(value),
            storage: "database".into(),
        };
    }
    StoredKey {
        value: None,
        storage: "none".into(),
    }
}

pub fn store_api_key(db: &Database, key: &str) -> Result<String, String> {
    if key.trim().is_empty() {
        clear_api_key(db)?;
        return Ok("none".into());
    }
    let key = key.trim();
    if key.chars().count() > 500 {
        return Err("Nyckeln är oväntat lång.".into());
    }
    if let Ok(entry) = keyring::Entry::new(SERVICE, USER) {
        if entry.set_password(key).is_ok() {
            db.clear_file_api_key()?;
            db.set_key_storage("keyring")?;
            return Ok("keyring".into());
        }
    }
    db.set_file_api_key(key)?;
    db.set_key_storage("database")?;
    Ok("database".into())
}

pub fn clear_api_key(db: &Database) -> Result<(), String> {
    if let Ok(entry) = keyring::Entry::new(SERVICE, USER) {
        let _ = entry.delete_credential();
    }
    db.clear_file_api_key()?;
    db.set_key_storage("none")
}

fn keyring_get() -> Option<String> {
    let entry = keyring::Entry::new(SERVICE, USER).ok()?;
    let value = entry.get_password().ok()?;
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
