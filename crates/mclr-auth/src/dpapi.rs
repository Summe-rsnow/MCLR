//! DPAPI wrapper for Windows master key protection.

use crate::error::AuthError;
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use windows::core::PCWSTR;

const ENTROPY: &[u8] = b"MCLR account key";

pub fn protect_bytes(data: &[u8]) -> Result<Vec<u8>, AuthError> {
    unsafe {
        let data_in = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let entropy_blob = CRYPT_INTEGER_BLOB {
            cbData: ENTROPY.len() as u32,
            pbData: ENTROPY.as_ptr() as *mut u8,
        };
        let mut data_out = CRYPT_INTEGER_BLOB::default();

        CryptProtectData(
            &data_in,
            PCWSTR::null(),
            Some(&entropy_blob),
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut data_out,
        )
        .map_err(|e| AuthError::Crypto(format!("CryptProtectData failed: {e}")))?;

        let result = std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(data_out.pbData as _));
        Ok(result)
    }
}

pub fn unprotect_bytes(encrypted: &[u8]) -> Result<Vec<u8>, AuthError> {
    unsafe {
        let data_in = CRYPT_INTEGER_BLOB {
            cbData: encrypted.len() as u32,
            pbData: encrypted.as_ptr() as *mut u8,
        };
        let entropy_blob = CRYPT_INTEGER_BLOB {
            cbData: ENTROPY.len() as u32,
            pbData: ENTROPY.as_ptr() as *mut u8,
        };
        let mut data_out = CRYPT_INTEGER_BLOB::default();

        CryptUnprotectData(
            &data_in,
            None,
            Some(&entropy_blob),
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut data_out,
        )
        .map_err(|e| AuthError::Crypto(format!("CryptUnprotectData failed: {e}")))?;

        let result = std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(data_out.pbData as _));
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpapi_round_trip() {
        let secret = b"my ultra secret 32-byte master key!!";
        let encrypted = protect_bytes(secret).unwrap();
        assert_ne!(encrypted, secret);
        let decrypted = unprotect_bytes(&encrypted).unwrap();
        assert_eq!(decrypted, secret);
    }
}
