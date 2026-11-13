use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use crate::error::{AppError, AppResult};

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))
}

pub fn verify_password(password: &str, password_hash: &str) -> AppResult<bool> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|e| AppError::Internal(format!("Invalid password hash format: {}", e)))?;

    let argon2 = Argon2::default();
    Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
}

