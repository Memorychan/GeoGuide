use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::{rngs::OsRng, RngCore};
use crate::error::{AppError, AppResult};

#[allow(dead_code)]
pub fn encrypt_field(plain_text: &str, key_32_bytes: &[u8; 32]) -> AppResult<String> {
    let cipher = Aes256Gcm::new(key_32_bytes.into());
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plain_text.as_bytes())
        .map_err(|e| AppError::Internal(format!("Field encryption error: {:?}", e)))?;

    // Combine nonce (12 bytes) + ciphertext into hex string
    let mut combined = nonce_bytes.to_vec();
    combined.extend(ciphertext);
    Ok(hex::encode(combined))
}

#[allow(dead_code)]
pub fn decrypt_field(encrypted_hex: &str, key_32_bytes: &[u8; 32]) -> AppResult<String> {
    let combined = hex::decode(encrypted_hex)
        .map_err(|e| AppError::BadRequest(format!("Invalid encrypted hex: {}", e)))?;

    if combined.len() < 12 {
        return Err(AppError::BadRequest("Ciphertext is too short".to_string()));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let cipher = Aes256Gcm::new(key_32_bytes.into());
    let nonce = Nonce::from_slice(nonce_bytes);

    let decrypted_bytes = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AppError::Internal(format!("Field decryption error: {:?}", e)))?;

    String::from_utf8(decrypted_bytes)
        .map_err(|e| AppError::Internal(format!("Invalid UTF-8 in decrypted data: {}", e)))
}
