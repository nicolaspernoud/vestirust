use crate::helpers::{create_apps_file, TestApp};
use std::{
    convert::Infallible,
    io::{self, BufWriter, Seek, Write},
};

use tokio::fs::File;

use anyhow::Result;
use base64ct::{Base64, Encoding};
use futures::StreamExt;
use sha2::{Digest, Sha512};

#[tokio::test]
async fn put_and_retrieve_tests() -> Result<()> {
    let app = TestApp::spawn().await;
    put_and_get_file(&app, app.port, "lorem.txt", "files1", false).await?;
    put_and_get_file(&app, app.port, "lorem.txt", "files2", true).await?;

    let big_file_path = "tests/data/big_file.bin";
    create_big_binary_file(big_file_path);
    put_and_get_file(&app, app.port, "big_file.bin", "files1", false).await?;
    put_and_get_file(&app, app.port, "big_file.bin", "files2", true).await?;

    std::fs::remove_file(&app.config_file).ok();
    std::fs::remove_file(big_file_path).ok();
    Ok(())
}

async fn put_and_get_file(
    app: &TestApp,
    port: u16,
    file_name: &str,
    dav_server: &str,
    encrypted: bool,
) -> Result<()> {
    let mut file = std::fs::File::open(format!("tests/data/{file_name}"))?;

    let mut hasher = Sha512::new();
    io::copy(&mut file, &mut hasher)?;
    let hash_source = hasher.finalize();
    println!("Source file hash: {}", Base64::encode_string(&hash_source));

    let file = File::open(format!("tests/data/{file_name}")).await?;
    // Act : send the file
    let resp = app
        .client
        .put(format!(
            "http://{dav_server}.vestibule.io:{port}/{file_name}"
        ))
        .body(file_to_body(file))
        .send()
        .await?;
    assert_eq!(resp.status(), 201);

    let stored_file_path = if !encrypted {
        format!("data/dir1/{file_name}")
    } else {
        format!("data/dir2/{file_name}")
    };
    let mut stored_file = std::fs::File::open(stored_file_path)?;
    let mut hasher = Sha512::new();
    io::copy(&mut stored_file, &mut hasher)?;
    let hash_stored = hasher.finalize();
    println!("Stored file hash: {}", Base64::encode_string(&hash_stored));
    // Assert that the stored file is the same as the send file... or not if it it encrypted
    if !encrypted {
        assert_eq!(hash_source, hash_stored);
    } else {
        assert!(hash_source != hash_stored);
    }

    // Act : retrieve the file
    let resp = app
        .client
        .get(format!(
            "http://{dav_server}.vestibule.io:{port}/{file_name}"
        ))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let mut stream = resp.bytes_stream();

    let mut hasher = Sha512::new();
    while let Some(item) = stream.next().await {
        let chunk = item?;
        hasher.write_all(&chunk)?;
    }
    let hash_retrieved = hasher.finalize();
    println!(
        "Retrieved file hash: {}",
        Base64::encode_string(&hash_retrieved)
    );
    // Assert that the retrieved file is the same as the original file
    assert_eq!(hash_source, hash_retrieved);
    Ok(())
}

fn file_to_body(file: File) -> reqwest::Body {
    let stream = tokio_util::codec::FramedRead::new(file, tokio_util::codec::BytesCodec::new());
    let body = reqwest::Body::wrap_stream(stream);
    body
}

fn create_big_binary_file(path: &str) {
    let size = 100_000_000;

    std::fs::remove_file(path).ok();
    let f = std::fs::File::create(path).unwrap();
    let mut writer = BufWriter::new(f);

    let mut rng = rand::thread_rng();
    let mut buffer = [0; 1024];
    let mut remaining_size = size;

    while remaining_size > 0 {
        let to_write = std::cmp::min(remaining_size, buffer.len());
        let buffer = &mut buffer[..to_write];
        rand::Rng::fill(&mut rng, buffer);
        io::Write::write(&mut writer, buffer).unwrap();
        remaining_size -= to_write;
    }
}

#[tokio::test]
async fn get_correct_range() -> Result<()> {
    let app = TestApp::spawn().await;

    let cases = vec!["files1", "files2"];

    for case in cases.iter() {
        let file = File::open(format!("tests/data/lorem.txt")).await?;
        // Act : send the file
        let resp = app
            .client
            .put(format!("http://{case}.vestibule.io:{}/{case}", app.port))
            .body(file_to_body(file))
            .send()
            .await?;
        assert_eq!(resp.status(), 201);

        // Act : retrieve the file
        let resp = app
            .client
            .get(format!("http://{case}.vestibule.io:{}/{case}", app.port))
            .header(hyper::header::RANGE, "bytes=20000-20050")
            .send()
            .await?;
        assert_eq!(resp.status(), 206);
        assert_eq!(
            resp.text().await?,
            "estie vitae volutpat eget, aliquet ac ipsum. Quisqu"
        );
    }

    std::fs::remove_file(&app.config_file).ok();
    Ok(())
}
