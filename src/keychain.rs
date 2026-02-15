use anyhow::{Context, Result};
use rand_core::RngCore;
use security_framework::os::macos::keychain::SecKeychain;
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

const SERVICE: &str = "op-loader cache key";
const ACCOUNT: &str = "default";

pub fn get_or_create_key() -> Result<[u8; 32]> {
    if let Some(existing) = try_get_key()? {
        return Ok(existing);
    }

    let mut key = [0u8; 32];
    rand_core::OsRng.fill_bytes(&mut key);

    set_generic_password(SERVICE, ACCOUNT, &key)
        .context("Failed to store cache key in Keychain")?;

    Ok(key)
}

pub fn delete_key() -> Result<()> {
    if get_generic_password(SERVICE, ACCOUNT).is_ok() {
        delete_generic_password(SERVICE, ACCOUNT)
            .context("Failed to delete cache key from Keychain")?;
    }
    Ok(())
}

fn try_get_key() -> Result<Option<[u8; 32]>> {
    match get_generic_password(SERVICE, ACCOUNT) {
        Ok(bytes) => {
            if bytes.len() != 32 {
                anyhow::bail!(
                    "Invalid Keychain cache key length: expected 32 bytes, got {}",
                    bytes.len()
                );
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            Ok(Some(key))
        }
        Err(_) => Ok(None),
    }
}

pub fn assert_keychain_available() -> Result<()> {
    SecKeychain::default().context("Failed to access default Keychain")?;
    Ok(())
}
