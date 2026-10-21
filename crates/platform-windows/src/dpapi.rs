//! Proteção de dados sensíveis em repouso via DPAPI (CryptProtectData).
//!
//! A chave é derivada das credenciais do usuário pelo próprio Windows — não
//! há chave para gerenciar, e o dado só é legível pelo mesmo usuário na
//! mesma máquina.

use dictos_history::{ProtectError, Protector};
use windows::Win32::Foundation::LocalFree;
use windows::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB,
};

/// Protetor DPAPI no escopo do usuário atual.
pub struct DpapiProtector;

fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    }
}

/// Lê o blob de saída e devolve o buffer ao Windows (LocalFree).
unsafe fn take_output(out: &CRYPT_INTEGER_BLOB) -> Vec<u8> {
    let bytes = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
    let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(
        out.pbData as *mut _,
    )));
    bytes
}

impl Protector for DpapiProtector {
    fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, ProtectError> {
        unsafe {
            let mut out = CRYPT_INTEGER_BLOB::default();
            CryptProtectData(&blob(plaintext), None, None, None, None, 0, &mut out)
                .map_err(|e| ProtectError(format!("CryptProtectData: {e}")))?;
            Ok(take_output(&out))
        }
    }

    fn unprotect(&self, cipher: &[u8]) -> Result<Vec<u8>, ProtectError> {
        unsafe {
            let mut out = CRYPT_INTEGER_BLOB::default();
            CryptUnprotectData(&blob(cipher), None, None, None, None, 0, &mut out)
                .map_err(|e| ProtectError(format!("CryptUnprotectData: {e}")))?;
            Ok(take_output(&out))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpapi_roundtrip() {
        let protector = DpapiProtector;
        let plain = b"serendipity";
        let cipher = protector.protect(plain).unwrap();
        assert_ne!(cipher, plain);
        assert_eq!(protector.unprotect(&cipher).unwrap(), plain);
    }

    #[test]
    fn cifra_nao_contem_texto_puro() {
        let cipher = DpapiProtector.protect(b"serendipity").unwrap();
        let haystack = String::from_utf8_lossy(&cipher);
        assert!(!haystack.contains("serendipity"));
    }
}
