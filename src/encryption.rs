use std::ffi::OsStr;
use std::fs::Permissions;
use std::os::unix::prelude::PermissionsExt;
use std::path::PathBuf;
use std::{io, path::Path};

use argon2::Argon2;
use async_compression::tokio::write::{XzEncoder, XzDecoder};
use chacha20poly1305::{
    aead::stream::DecryptorBE32, aead::stream::EncryptorBE32, KeyInit, XChaCha20Poly1305,
};
use rand_core::{OsRng, RngCore};
use thiserror::Error;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::process::Command;
use tokio::fs::File;
use zeroize::{Zeroize, Zeroizing};

const BUF_LEN: usize = 1024;
const SALT_LEN: usize = 32;
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 19;
const AEAD_LEN: usize = 16;

type Passwd = Zeroizing<String>;

#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("Failed to hash password")]
    PasswdHashingError,
    #[error("Invalid key length")]
    KeyLengthError,
//    #[error("Failed to compress file")]
//    CompressionFailed,
//    #[error("Failed to compress file")]
//    DecompressionFailed,
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Encryption failed")]
    DecryptionFailed,
    #[error("Shredding failed")]
    ShreddingFailed,
    #[error("Invalid path")]
    InvalidPath,
    #[error("Failed to get nonce and key")]
    CouldntGetNonceAndKey,
    #[error("IO error: `{0}`")]
    IOError(#[from] io::Error),
}

pub async fn shred<S: AsRef<OsStr>>(path: S) -> Result<(), EncryptionError> {
    let mut shredder = match Command::new("shred")
        .args(&[
            OsStr::new("-f").as_ref(),
            OsStr::new("-z").as_ref(),
            OsStr::new("-u").as_ref(),
            path.as_ref(),
        ])
        .spawn()
    {
        Ok(shredder) => shredder,
        Err(_) => return Err(EncryptionError::ShreddingFailed),
    };
    let status = match shredder.wait().await {
        Ok(status) => status,
        Err(_) => return Err(EncryptionError::ShreddingFailed),
    };
    if status.success() {
        Ok(())
    } else {
        Err(EncryptionError::ShreddingFailed)
    }
}

/// Compresses file from src_path into dest_path
pub async fn compress<P: AsRef<Path>>(src_path: P, dest_path: P) -> io::Result<()> {
    let mut src = File::open(src_path).await?;
    let dest = File::create(dest_path).await?;
    let mut encoder = XzEncoder::new(dest);
    let mut buf = [0u8; BUF_LEN];
    loop {
        let n = src.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        encoder.write_all(&buf[..n]).await?;
    }
    encoder.flush().await?;
    Ok(())
}

/// Decompresses file from src_path into dest_path
pub async fn decompress<P: AsRef<Path>>(src_path: P, dest_path: P) -> io::Result<()> {
    let mut src = File::open(src_path).await?;
    let dest = File::create(dest_path).await?;
    let mut encoder = XzDecoder::new(dest);
    let mut buf = [0u8; BUF_LEN];
    loop {
        let n = src.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        encoder.write_all(&buf[..n]).await?;
    }
    encoder.flush().await?;
    Ok(())
}

fn into_key(passwd: &Passwd) -> Result<(Vec<u8>, Vec<u8>), EncryptionError> {
    let argon2 = Argon2::default();
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let mut key = [0u8; KEY_LEN];
    if let Err(err) = argon2.hash_password_into(passwd.as_bytes(), &salt, &mut key) {
        log::error!("{:?}", err);
        return Err(EncryptionError::PasswdHashingError);
    }
    Ok((Vec::from(key), Vec::from(salt)))
}

fn derive_key(passwd: &Passwd, salt: Vec<u8>) -> Result<Vec<u8>, EncryptionError> {
    let argon2 = Argon2::default();
    let mut key = [0u8; KEY_LEN];
    if let Err(err) = argon2.hash_password_into(passwd.as_bytes(), &salt, &mut key) {
        log::error!("{:?}", err);
        return Err(EncryptionError::PasswdHashingError);
    }
    Ok(Vec::from(key))
}

/// Encrypts a file and then shreds it.
/// <p><b>passwd</b>: Password used to encrypt the file.
/// <p><b>src_path</b>: Path of the file to encrypt.
/// <p><b>dest_path</b>: Destination of the encrypted file.
pub async fn encrypt<P: AsRef<Path>>(
    passwd: &Passwd,
    src_path: P,
    dest_path: P,
) -> Result<(), EncryptionError> {
    let (mut key, mut salt) = into_key(passwd)?;
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);

    let cipher = match XChaCha20Poly1305::new_from_slice(&key) {
        Ok(key) => key,
        Err(_) => return Err(EncryptionError::KeyLengthError),
    };
    let mut stream_encryptor = EncryptorBE32::from_aead(cipher, nonce.as_ref().into());

    let mut dest = match File::create(dest_path).await {
        Ok(file) => file,
        Err(err) => return Err(EncryptionError::IOError(err)),
    };

    // Writing salt and nonce
    if let Err(err) = dest.write_all(&salt).await {
        return Err(EncryptionError::IOError(err));
    }
    if let Err(err) = dest.write_all(&nonce).await {
        return Err(EncryptionError::IOError(err));
    }

    let mut src = match File::open(&src_path).await {
        Ok(file) => file,
        Err(err) => return Err(EncryptionError::IOError(err)),
    };

    let mut buffer = [0u8; BUF_LEN];
    loop {
        let n = src.read(&mut buffer).await?;
        if n == BUF_LEN {
            let ciphertext = match stream_encryptor.encrypt_next(buffer.as_slice()) {
                Ok(ciphertext) => ciphertext,
                Err(_) => return Err(EncryptionError::EncryptionFailed),
            };
            if let Err(err) = dest.write_all(&ciphertext).await {
                return Err(EncryptionError::IOError(err));
            };
        } else {
            let ciphertext = match stream_encryptor.encrypt_last(&buffer[..n]) {
                Ok(ciphertext) => ciphertext,
                Err(_) => return Err(EncryptionError::EncryptionFailed),
            };
            if let Err(err) = dest.write_all(&ciphertext).await {
                return Err(EncryptionError::IOError(err));
            };
            break;
        }
    }
    if let Err(err) = dest.shutdown().await {
        return Err(EncryptionError::IOError(err));
    }

    key.zeroize();
    salt.zeroize();
    nonce.zeroize();
    buffer.zeroize();
    shred(src_path.as_ref().as_os_str()).await?;
    Ok(())
}

/// Creates a temporary file where the encrypted file will be decrypted, then returns the path to the temporary file.
/// <p><b>passwd</b>: Password to decrypt the file.
/// <p><b>src_path</b>: Path of the file to be decrypted.
/// <p><b>tmp_dir</b>: Directory where the temporary file will be created.
pub async fn decrypt<P: AsRef<Path>>(
    passwd: &Passwd,
    src_path: P,
    tmp_dir: P,
) -> Result<String, EncryptionError>
where
    PathBuf: std::convert::From<P>,
{
    let mut src = match File::open(src_path).await {
        Ok(src) => src,
        Err(err) => return Err(EncryptionError::IOError(err)),
    };
    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    {
        let n = match src.read(&mut salt).await {
            Ok(n) => n,
            Err(err) => return Err(EncryptionError::IOError(err)),
        };
        if n != SALT_LEN {
            return Err(EncryptionError::CouldntGetNonceAndKey);
        }
        let n = match src.read(&mut nonce).await {
            Ok(n) => n,
            Err(err) => return Err(EncryptionError::IOError(err)),
        };
        if n != NONCE_LEN {
            return Err(EncryptionError::CouldntGetNonceAndKey);
        }
    }

    let mut key = derive_key(passwd, Vec::from(salt))?;
    let cipher = match XChaCha20Poly1305::new_from_slice(&key) {
        Ok(key) => key,
        Err(_) => return Err(EncryptionError::KeyLengthError),
    };
    let mut stream_decryptor = DecryptorBE32::from_aead(cipher, nonce.as_ref().into());

    // Create temporary file
    let mut dest_path = PathBuf::from(tmp_dir);
    dest_path.push(format!("{}.tmp", OsRng.next_u64().to_string()));
    // Avoid repetitions
    loop {
        if dest_path.exists() {
            dest_path.pop();
            dest_path.push(OsRng.next_u64().to_string());
        } else {
            break;
        }
    }
    let mut dest = match File::create(&dest_path).await {
        Ok(dest) => dest,
        Err(err) => return Err(EncryptionError::IOError(err)),
    };
    if let Err(err) = dest.set_permissions(Permissions::from_mode(0o600)).await {
        return Err(EncryptionError::IOError(err));
    }

    let mut buffer = [0u8; BUF_LEN + AEAD_LEN];
    loop {
        let n = match src.read(&mut buffer).await {
            Ok(n) => n,
            Err(err) => return Err(EncryptionError::IOError(err)),
        };
        if n == BUF_LEN + AEAD_LEN {
            let plaintext = match stream_decryptor.decrypt_next(buffer.as_slice()) {
                Ok(plaintext) => plaintext,
                Err(_) => return Err(EncryptionError::DecryptionFailed),
            };
            dest.write_all(&plaintext).await?
        } else if n == 0 {
            break;
        } else {
            let plaintext = match stream_decryptor.decrypt_last(&buffer[..n]) {
                Ok(plaintext) => plaintext,
                Err(_) => return Err(EncryptionError::DecryptionFailed),
            };
            dest.write_all(&plaintext).await?;
            break;
        }
    }
    if let Err(err) = dest.shutdown().await {
        return Err(EncryptionError::IOError(err));
    }

    buffer.zeroize();
    nonce.zeroize();
    salt.zeroize();
    key.zeroize();
    Ok(match dest_path.to_str() {
        Some(path) => String::from(path),
        None => return Err(EncryptionError::InvalidPath),
    })
}
