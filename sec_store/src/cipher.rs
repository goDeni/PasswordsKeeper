use aes::cipher::{KeyIvInit, StreamCipher};
use anyhow::Result;
use rand::Rng;
use ring::digest;
use serde::{Deserialize, Serialize};

type _Aes128Ctr64LE = ctr::Ctr64LE<aes::Aes256>;

pub type EncryptionKey = String;
pub type DecryptResult<T> = Result<T, DecryptionError>;

#[derive(Debug, Clone, PartialEq)]
pub enum DecryptionError {
    WrongPassword,
    EncodingError(std::string::FromUtf8Error),
}

#[derive(Serialize, Deserialize)]
pub struct EncryptedData {
    data: Vec<u8>,
    nonce: [u8; 16],
    hash: Vec<u8>,
}

pub fn encrypt_string(passwd: &EncryptionKey, string: String) -> EncryptedData {
    let key = digest::digest(&digest::SHA256, passwd.as_bytes());

    let mut encrypted_data = EncryptedData {
        data: string.as_bytes().to_vec(),
        nonce: rand::thread_rng().gen(),
        hash: md5::compute(string).to_vec(),
    };

    let mut cipher = _Aes128Ctr64LE::new(key.as_ref().into(), &encrypted_data.nonce.into());
    cipher.apply_keystream(&mut encrypted_data.data);

    encrypted_data
}

pub fn decrypt_string(
    passwd: &EncryptionKey,
    mut encrypted_data: EncryptedData,
) -> DecryptResult<String> {
    let key = digest::digest(&digest::SHA256, passwd.as_bytes());
    let mut cipher = _Aes128Ctr64LE::new(key.as_ref().into(), &encrypted_data.nonce.into());

    cipher.apply_keystream(&mut encrypted_data.data);
    if md5::compute(encrypted_data.data.clone()).to_vec() != encrypted_data.hash {
        return Err(DecryptionError::WrongPassword);
    }

    String::from_utf8(encrypted_data.data).map_err(DecryptionError::EncodingError)
}

#[test]
fn test_encryption() {
    let passwd = "some password!".to_string();
    let plaintext = String::from("Hellow rodl!");

    let decryted_string = decrypt_string(&passwd, encrypt_string(&passwd, plaintext.clone()));

    assert_eq!(decryted_string.unwrap(), plaintext);
}

#[test]
fn test_encryption_data() {
    let passwd = "some password!".to_string();
    let plaintext = String::from("Hellow rodl!");

    let encrypted_data = encrypt_string(&passwd, plaintext.clone());

    let result = String::from_utf8(encrypted_data.data);
    if let Ok(result_str) = result {
        assert_ne!(result_str, plaintext);
    }
}

#[test]
fn test_encryption_acces_with_wrong_passwd() {
    let result = decrypt_string(
        &"Second password!".to_string(),
        encrypt_string(&"First password!".to_string(), String::from("Some string")),
    );
    let expected = DecryptionError::WrongPassword;

    assert_eq!(result, Err(expected));
}
