//! Disclaimer acceptance state, stored with tamper-resistant signing.
//!
//! Primary store (Windows): `HKCU\Software\XDownload`
//!   - `disclaimer_accepted` (REG_DWORD): `1` when accepted
//!   - `disclaimer_sig` (REG_BINARY): HMAC-SHA256 signature
//!
//! Fallback (non-Windows): a hidden file under the app config dir holding the
//! same signature, so the store stays self-contained on all platforms.
//!
//! The acceptance flag is only honored when its signature verifies against a
//! compiled-in key. Editing the flag (or the JSON config, or the fallback
//! file) without also producing a matching signature makes the app treat it
//! as "not accepted", so the disclaimer cannot be silently skipped.

use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const REG_KEY_PATH: &str = r"Software\XDownload";
const VALUE_ACCEPTED: &str = "disclaimer_accepted";
const VALUE_SIGNATURE: &str = "disclaimer_sig";
#[cfg(not(windows))]
const FALLBACK_FILE: &str = ".disclaimer_state";

/// Compiled-in secret key for the HMAC. Treat it like an application secret:
/// rotating it invalidates every previously stored acceptance.
const HMAC_KEY: &[u8] = b"XDownload::disclaimer::hmac::v1::7f3d9c2e8b1a4d5f";

/// The canonical payload that gets signed.
const PAYLOAD: &[u8] = b"XDownload:disclaimer:accepted";

/// Compute the expected signature for an accepted state.
fn sign() -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(HMAC_KEY).expect("HMAC accepts any key length");
    mac.update(PAYLOAD);
    mac.finalize().into_bytes().to_vec()
}

/// Constant-time comparison of a stored signature against the expected one.
fn verify(sig: &[u8]) -> bool {
    let expected = sign();
    if expected.len() != sig.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in expected.iter().zip(sig.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// Mark the disclaimer as accepted and persist a signed record.
pub fn accept() -> Result<()> {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        use winreg::RegValue;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _disp) = hkcu
            .create_subkey(REG_KEY_PATH)
            .context("failed to create HKCU\\Software\\XDownload")?;
        key.set_value(VALUE_ACCEPTED, &1u32)
            .context("failed to write disclaimer_accepted")?;
        let sig = RegValue {
            bytes: sign(),
            vtype: RegType::REG_BINARY,
        };
        key.set_raw_value(VALUE_SIGNATURE, &sig)
            .context("failed to write disclaimer_sig")?;
        tracing::info!("[disclaimer] accepted and signed (registry)");
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let path = crate::utils::app_home::AppHome::config_dir().join(FALLBACK_FILE);
        let sig = sign();
        std::fs::write(&path, &sig).context("failed to write disclaimer state file")?;
        tracing::info!("[disclaimer] accepted and signed (file fallback)");
        Ok(())
    }
}

/// Whether the disclaimer has been accepted AND its signature is valid.
pub fn is_accepted() -> bool {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let Ok(key) = hkcu.open_subkey(REG_KEY_PATH) else {
            return false;
        };
        let accepted: u32 = key.get_value(VALUE_ACCEPTED).unwrap_or(0);
        if accepted != 1 {
            return false;
        }
        let Ok(raw) = key.get_raw_value(VALUE_SIGNATURE) else {
            return false;
        };
        verify(&raw.bytes)
    }
    #[cfg(not(windows))]
    {
        let path = crate::utils::app_home::AppHome::config_dir().join(FALLBACK_FILE);
        let Ok(bytes) = std::fs::read(&path) else {
            return false;
        };
        verify(&bytes)
    }
}
