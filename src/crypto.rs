//! Cryptographic utilities for secure storage

use anyhow::{Result, anyhow};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};
use argon2::{Argon2, password_hash::{SaltString, PasswordHasher, PasswordVerifier, PasswordHash}};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rand::RngCore;

/// Encrypt a string using AES-256-GCM
pub fn encrypt_string(plaintext: &str, key: &[u8]) -> Result<String> {
    // Ensure key is 32 bytes
    if key.len() != 32 {
        return Err(anyhow!("Encryption key must be 32 bytes"));
    }
    
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    
    // Generate a random 96-bit nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    // Encrypt
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;
    
    // Combine nonce and ciphertext
    let mut combined = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    
    // Encode as base64
    Ok(BASE64.encode(combined))
}

/// Decrypt a string encrypted with encrypt_string
pub fn decrypt_string(ciphertext: &str, key: &[u8]) -> Result<String> {
    // Ensure key is 32 bytes
    if key.len() != 32 {
        return Err(anyhow!("Decryption key must be 32 bytes"));
    }
    
    // Decode from base64
    let combined = BASE64.decode(ciphertext)
        .map_err(|e| anyhow!("Invalid base64: {}", e))?;
    
    // Extract nonce and ciphertext
    if combined.len() < 12 {
        return Err(anyhow!("Invalid ciphertext: too short"));
    }
    
    let (nonce_bytes, ciphertext_bytes) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    
    // Decrypt
    let plaintext_bytes = cipher.decrypt(nonce, ciphertext_bytes)
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;
    
    String::from_utf8(plaintext_bytes)
        .map_err(|e| anyhow!("Invalid UTF-8: {}", e))
}

/// Derive an encryption key from a master key and context
pub fn derive_key(master_key: &[u8], context: &str) -> Result<Vec<u8>> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    
    type HmacSha256 = Hmac<Sha256>;
    
    let mut mac = <HmacSha256 as Mac>::new_from_slice(master_key)
        .map_err(|e| anyhow!("Invalid key length: {}", e))?;
    
    mac.update(b"JauAuth-Server-Encryption-");
    mac.update(context.as_bytes());
    
    Ok(mac.finalize().into_bytes().to_vec())
}

/// Generate a random encryption key
pub fn generate_key() -> Vec<u8> {
    let mut key = vec![0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

/// Hash a password using Argon2
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let password_hash = argon2.hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("Failed to hash password: {}", e))?;
    
    Ok(password_hash.to_string())
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| anyhow!("Invalid password hash: {}", e))?;
    
    let argon2 = Argon2::default();
    Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encrypt_decrypt() {
        let key = generate_key();
        let plaintext = "Hello, World! This is a secret.";
        
        let encrypted = encrypt_string(plaintext, &key).unwrap();
        let decrypted = decrypt_string(&encrypted, &key).unwrap();
        
        assert_eq!(plaintext, decrypted);
    }
    
    #[test]
    fn test_encrypt_different_each_time() {
        let key = generate_key();
        let plaintext = "Same text";
        
        let encrypted1 = encrypt_string(plaintext, &key).unwrap();
        let encrypted2 = encrypt_string(plaintext, &key).unwrap();
        
        // Should be different due to random nonce
        assert_ne!(encrypted1, encrypted2);
        
        // But both should decrypt to same value
        assert_eq!(decrypt_string(&encrypted1, &key).unwrap(), plaintext);
        assert_eq!(decrypt_string(&encrypted2, &key).unwrap(), plaintext);
    }
    
    #[test]
    fn test_wrong_key_fails() {
        let key1 = generate_key();
        let key2 = generate_key();
        let plaintext = "Secret";
        
        let encrypted = encrypt_string(plaintext, &key1).unwrap();
        let result = decrypt_string(&encrypted, &key2);
        
        assert!(result.is_err());
    }
}