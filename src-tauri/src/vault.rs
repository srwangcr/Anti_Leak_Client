//! Bóveda opcional: secretos guardados en el keychain nativo del sistema operativo.
//! No se escribe ninguna credencial en disco ni en la base de datos.

use keyring::Entry;

const SERVICE: &str = "anti_leak_client";

fn entry(key: &str) -> Result<Entry, String> {
    if key.trim().is_empty() {
        return Err("La clave no puede estar vacía".into());
    }
    Entry::new(SERVICE, key).map_err(|e| format!("No se pudo acceder al keychain: {}", e))
}

pub fn save_credential(key: &str, value: &str) -> Result<(), String> {
    entry(key)?
        .set_password(value)
        .map_err(|e| format!("No se pudo guardar en el keychain: {}", e))
}

pub fn load_credential(key: &str) -> Result<Option<String>, String> {
    match entry(key)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("No se pudo leer del keychain: {}", e)),
    }
}

pub fn delete_credential(key: &str) -> Result<(), String> {
    match entry(key)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("No se pudo eliminar del keychain: {}", e)),
    }
}
