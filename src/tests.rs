use std::env::current_dir;

use rand::distributions::{Alphanumeric, DistString};
use tokio::{
    fs::{remove_file, File},
    io::{AsyncReadExt, AsyncWriteExt},
};
use zeroize::Zeroizing;

use crate::encryption::*;

pub async fn test_main() {
    log::info!("Starting encryption testing...");
    test_encryption().await;

    log::info!("Starting compression testing...");
    test_compression().await;
}

pub async fn test_encryption() {
    log::info!("Preparing files...");
    let cur_path = current_dir().expect("Failed to get current dir");
    let cur_path = cur_path.to_str().expect("Invalid dir");
    let passwd = Zeroizing::new(String::from("test"));
    let file_content = "test test test";
    let file1_content = file_content.repeat(2000);
    let file2_content = Alphanumeric.sample_string(&mut rand::thread_rng(), 10000);
    {
        let mut file = File::create("test").await.expect("Failed to create file");
        file.write_all(file_content.as_bytes())
            .await
            .expect("Failed to write to file");
        let mut file1 = File::create("test1").await.expect("Failed to create file1");
        file1
            .write_all(file1_content.as_bytes())
            .await
            .expect("Failed to write to file1");
        let mut file2 = File::create("test2").await.expect("Failed to create file2");
        file2
            .write_all(file2_content.as_bytes())
            .await
            .expect("Failed to write to file2");
    }
    log::info!("Testing encryption...");
    encrypt(&passwd, "test", "test.encr")
        .await
        .expect("Encryption failed");
    encrypt(&passwd, "test1", "test1.encr")
        .await
        .expect("Encryption failed");
    encrypt(&passwd, "test2", "test2.encr")
        .await
        .expect("Encryption failed");
    log::info!("Testing decryption...");
    let path = decrypt(&passwd, "test.encr", cur_path)
        .await
        .expect("Decryption failed");
    let path1 = decrypt(&passwd, "test1.encr", cur_path)
        .await
        .expect("Decryption failed");
    let path2 = decrypt(&passwd, "test2.encr", cur_path)
        .await
        .expect("Decryption failed");
    log::info!("Checking content...");
    {
        let mut file = File::open(&path).await.expect("Failed to open file");
        let mut file1 = File::open(&path1).await.expect("Failed to open file");
        let mut file2 = File::open(&path2).await.expect("Failed to open file");
        let mut content = String::new();
        let mut content1 = String::new();
        let mut content2 = String::new();
        file.read_to_string(&mut content)
            .await
            .expect("Failed to read to string");
        file1
            .read_to_string(&mut content1)
            .await
            .expect("Failed to read to string");
        file2
            .read_to_string(&mut content2)
            .await
            .expect("Failed to read to string");
        assert_eq!(content, file_content.to_string());
        assert_eq!(content1, file1_content);
        assert_eq!(content2, file2_content);
        log::info!("Assertions succeeded");
    }

    log::info!("Cleaning up...");
    remove_file(path).await.expect("Failed to remove file");
    remove_file(path1).await.expect("Failed to remove file");
    remove_file(path2).await.expect("Failed to remove file");
    remove_file("test.encr")
        .await
        .expect("Failed to remove file");
    remove_file("test1.encr")
        .await
        .expect("Failed to remove file");
    remove_file("test2.encr")
        .await
        .expect("Failed to remove file");
}

pub async fn test_compression() {
    log::info!("Preparing files...");
    let file_content = "test test test\n".repeat(2000);
    let file1_content = Alphanumeric.sample_string(&mut rand::thread_rng(), 2500);
    {
        let mut file = File::create("test").await.expect("Failed to create file");
        let mut file1 = File::create("test1").await.expect("Failed to create file1");
        file.write_all(file_content.as_bytes())
            .await
            .expect("Failed to write to file");
        file1
            .write_all(file1_content.as_bytes())
            .await
            .expect("Failed to write to file1");
    }

    log::info!("Testing compression...");
    compress("test", "test.xz")
        .await
        .expect("Compression failed");
    compress("test1", "test1.xz")
        .await
        .expect("Compression failed");

    log::info!("Testing decompression...");
    decompress("test.xz", "test_dec")
        .await
        .expect("Decompression failed");
    decompress("test1.xz", "test1_dec")
        .await
        .expect("Decompression failed");

    log::info!("Checking content...");
    {
        let mut file = File::open("test_dec").await.expect("Failed to open file");
        let mut file1 = File::open("test1_dec").await.expect("Failed to open file");
        let mut content = String::new();
        let mut content1 = String::new();
        file.read_to_string(&mut content)
            .await
            .expect("Failed to read file");
        file1
            .read_to_string(&mut content1)
            .await
            .expect("Failed to read file");
        assert_eq!(content, file_content);
        assert_eq!(content1, file1_content);
        log::info!("Assertions succeeded");
    }

    log::info!("Cleaning up...");
    remove_file("test").await.expect("Failed to remove file");
    remove_file("test1").await.expect("Failed to remove file");
    remove_file("test.xz").await.expect("Failed to remove file");
    remove_file("test1.xz")
        .await
        .expect("Failed to remove file");
    remove_file("test_dec")
        .await
        .expect("Failed to remove file");
    remove_file("test1_dec")
        .await
        .expect("Failed to remove file");
}
