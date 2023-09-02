use aes::cipher::{KeyIvInit, StreamCipher};
use rand::Rng;
use ring::digest;
use serde::{Deserialize, Serialize};
use anyhow::Result;

type _Aes128Ctr64LE = ctr::Ctr64LE<aes::Aes256>;

pub type EncryptionKey = &'static str;
pub type DecryptResult<T> = Result<T, DecryptionError>;

#[derive(Debug, Clone, PartialEq)]
pub enum DecryptionError {
    WrongPassword,
    UnexpectedError,
}

#[derive(Serialize, Deserialize)]
pub struct EncryptedData {
    data: Vec<u8>,
    nonce: [u8; 16],
    hash: Vec<u8>,
}

pub fn encrypt_string(passwd: EncryptionKey, string: String) -> EncryptedData {
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

pub fn decrypt_string(passwd: EncryptionKey, mut encrypted_data: EncryptedData) -> DecryptResult<String> {
    let key = digest::digest(&digest::SHA256, passwd.as_bytes());
    let mut cipher = _Aes128Ctr64LE::new(key.as_ref().into(), &encrypted_data.nonce.into());

    cipher.apply_keystream(&mut encrypted_data.data);
    if md5::compute(encrypted_data.data.clone()).to_vec() != encrypted_data.hash {
        return Err(DecryptionError::WrongPassword);
    }

    String::from_utf8(encrypted_data.data).map_err(|_error| DecryptionError::UnexpectedError)
}

#[test]
fn test_encryption() {
    let passwd = "some password!";
    let plaintext = String::from("Hellow rodl!");

    let decryted_string = decrypt_string(passwd, encrypt_string(passwd, plaintext.clone()));

    assert_eq!(decryted_string.unwrap(), plaintext);
}

#[test]
fn test_encryption_data() {
    let passwd = "some password!";
    let plaintext = String::from("Hellow rodl!");

    let encrypted_data = encrypt_string(passwd, plaintext.clone());

    let result = String::from_utf8(encrypted_data.data);
    if let Some(result_str) = result.ok() {
        assert_ne!(result_str, plaintext);
    }
}

#[test]
fn test_encryption_acces_with_wrong_passwd() {
    let result = decrypt_string(
        "Second password!",
        encrypt_string("First password!", String::from("Some string")),
    );
    let expected = DecryptionError::WrongPassword;

    assert_eq!(result, Err(expected));
}
