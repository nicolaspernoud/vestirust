use async_stream::stream;

use chacha20poly1305::aead::stream::NewStream;
use chacha20poly1305::aead::stream::StreamPrimitive;
use chacha20poly1305::aead::{stream, NewAead};
use chacha20poly1305::XChaCha20Poly1305;
use futures::{Stream, StreamExt};
use rand::rngs::OsRng;
use rand::RngCore;
use std::io::{Error, ErrorKind};
use std::pin::Pin;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};

pub const PLAIN_CHUNK_SIZE: usize = 10000;
pub const ENCRYPTION_OVERHEAD: usize = 16;
pub const ENCRYPTED_CHUNK_SIZE: usize = PLAIN_CHUNK_SIZE + ENCRYPTION_OVERHEAD;
pub const NONCE_SIZE: usize = 19;

pub struct EncryptedStreamer<I>
where
    I: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    inner: I,
    key: [u8; 32],
}

impl<I> EncryptedStreamer<I>
where
    I: AsyncRead + AsyncWrite + AsyncSeekExt + Unpin + Send + 'static,
{
    #[inline]
    pub fn new(inner: I, key: [u8; 32]) -> Self {
        Self { inner, key }
    }

    pub async fn copy_from<'a, R>(&'a mut self, mut reader: &'a mut R) -> Result<u64, Error>
    where
        R: AsyncRead + Unpin + ?Sized,
    {
        let mut nonce = [0; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce);
        let aead = XChaCha20Poly1305::new(self.key.as_ref().into());
        let mut stream_encryptor = stream::EncryptorBE32::from_aead(aead, nonce.as_ref().into());

        // Write the nonce as stream header
        self.inner.write(&nonce).await?;
        let mut total_count = 0;

        loop {
            let mut buffer = Vec::with_capacity(PLAIN_CHUNK_SIZE);
            let mut chunked_reader = reader.take(PLAIN_CHUNK_SIZE.try_into().unwrap());

            let read_count = chunked_reader.read_to_end(&mut buffer).await?;
            total_count += read_count;

            reader = chunked_reader.into_inner();
            buffer.truncate(read_count);

            if read_count == PLAIN_CHUNK_SIZE {
                let ciphertext = stream_encryptor.encrypt_next(buffer.as_slice()).unwrap();
                self.inner.write(&ciphertext).await?;
            } else {
                let ciphertext = stream_encryptor
                    .encrypt_last(&buffer[..read_count])
                    .unwrap();
                self.inner.write(&ciphertext).await?;
                break;
            }
        }
        self.inner.flush().await?;
        Ok(total_count as u64)
    }

    pub async fn copy_to<'a, W>(mut self, writer: &'a mut W) -> Result<u64, Error>
    where
        W: AsyncWrite + Unpin + ?Sized,
    {
        let nonce = self.retrieve_nonce().await?;
        let aead = XChaCha20Poly1305::new(self.key.as_ref().into());

        let mut stream_decryptor = stream::DecryptorBE32::from_aead(aead, nonce.as_ref().into());

        let mut total_count = 0;

        loop {
            let mut buffer = Vec::with_capacity(ENCRYPTED_CHUNK_SIZE);
            let mut reader = self.inner.take(ENCRYPTED_CHUNK_SIZE.try_into().unwrap());

            let read_count = reader.read_to_end(&mut buffer).await?;
            total_count += read_count;

            self.inner = reader.into_inner();
            buffer.truncate(read_count);

            if read_count == ENCRYPTED_CHUNK_SIZE {
                let plaintext = match stream_decryptor.decrypt_next(buffer.as_slice()) {
                    Ok(plaintext) => plaintext,
                    Err(_e) => {
                        panic!("decrypt in zip failed");
                    }
                };
                writer.write(&plaintext).await?;
            } else if read_count == 0 {
                break;
            } else {
                let plaintext = match stream_decryptor.decrypt_last(&buffer[..read_count]) {
                    Ok(plaintext) => plaintext,
                    Err(_e) => {
                        panic!("decrypt in zip failed");
                    }
                };
                writer.write(&plaintext).await?;
                break;
            }
        }
        Ok(total_count as u64)
    }

    pub fn into_stream(
        mut self,
    ) -> Pin<Box<impl ?Sized + Stream<Item = Result<Vec<u8>, Error>> + 'static>> {
        let stream = stream! {
            let aead = XChaCha20Poly1305::new(self.key.as_ref().into());
            let nonce = self.retrieve_nonce().await?;
            let mut stream_decryptor = stream::DecryptorBE32::from_aead(aead, nonce.as_ref().into());

             loop {
                let mut buffer = Vec::with_capacity(ENCRYPTED_CHUNK_SIZE);
                let mut reader = self.inner.take(ENCRYPTED_CHUNK_SIZE.try_into().unwrap());

                let read_count = reader.read_to_end(&mut buffer).await?;

                self.inner = reader.into_inner();
                buffer.truncate(read_count);

                if read_count == ENCRYPTED_CHUNK_SIZE {
                    let plaintext = match stream_decryptor
                        .decrypt_next(buffer.as_slice()) {
                            Ok(plaintext) => plaintext,
                            Err(e) => {yield Err(Error::new(ErrorKind::Other, format!("error decrypting plaintext: {}", e)));break;}
                        };
                    yield Ok(plaintext);
                } else if read_count == 0 {
                    break;
                } else {
                    let plaintext = match stream_decryptor
                    .decrypt_last(&buffer[..read_count]){
                            Ok(plaintext) => plaintext,
                            Err(e) => {yield Err(Error::new(ErrorKind::Other, format!("error decrypting plaintext: {}", e)));break;}
                        };
                        yield Ok(plaintext);
                     break;
                }
            }

        };
        stream.boxed()
    }

    // allow truncation as truncated remaining is always less than buf_size: usize
    pub fn into_stream_sized(
        mut self,
        start: u64,
        max_length: u64,
    ) -> Pin<Box<impl ?Sized + Stream<Item = Result<Vec<u8>, Error>> + 'static>> {
        let stream = stream! {
            let aead = XChaCha20Poly1305::new(self.key.as_ref().into());
            let nonce = self.retrieve_nonce().await?;
            let stream_decryptor = stream::StreamBE32::from_aead(aead, nonce.as_ref().into());
            let mut chunked_position = ChunkedPosition::new(start);
            self.inner.seek(std::io::SeekFrom::Start(chunked_position.beginning_of_active_chunk)).await?;


        let mut remaining = max_length;
            loop {
                if remaining == 0 {
                    break;
                }
                let mut buffer = Vec::with_capacity(ENCRYPTED_CHUNK_SIZE);
                let mut reader = self.inner.take(ENCRYPTED_CHUNK_SIZE.try_into().unwrap());
                let read_count = reader.read_to_end(&mut buffer).await?;
                self.inner = reader.into_inner();
                buffer.truncate(read_count);

                if read_count == ENCRYPTED_CHUNK_SIZE {
                    let mut plaintext = match stream_decryptor
                        .decrypt(chunked_position.active_chunk_counter as u32, false, buffer.as_slice()) {
                            Ok(plaintext) => plaintext,
                            Err(e) => {
                                println!("Error : {}", e);
                                yield Err(Error::new(ErrorKind::Other, format!("error decrypting plaintext: {}", e)));
                                break;
                            }
                        };

                        chunked_position.active_chunk_counter+= 1;

                        if start != 0 {
                            plaintext.drain(0..chunked_position.offset_in_active_chunk as usize);
                            chunked_position.offset_in_active_chunk = 0;
                        }
                        if (remaining as usize) < plaintext.len()   {
                            plaintext.truncate(remaining as usize);
                             yield Ok(plaintext);
                            break;
                        } else {
                            remaining -= plaintext.len() as u64;
                        }

                    yield Ok(plaintext);

                } else if read_count == 0 {
                    break;
                } else {
                    let mut plaintext = match stream_decryptor
                    .decrypt(chunked_position.active_chunk_counter as u32, true,&buffer[..read_count]){
                            Ok(plaintext) => plaintext,
                            Err(e) => {yield Err(Error::new(ErrorKind::Other, format!("error decrypting plaintext: {}", e)));break;}
                        };

                        if start != 0 {
                            plaintext.drain(0..chunked_position.offset_in_active_chunk as usize);
                        }
                        if (remaining as usize) < plaintext.len()   {

                            plaintext.truncate(remaining as usize);
                        }
                        yield Ok(plaintext);
                     break;
                }

            }
        };
        stream.boxed()
    }

    pub async fn retrieve_nonce(&mut self) -> Result<[u8; NONCE_SIZE], std::io::Error> {
        let mut nonce = [0u8; NONCE_SIZE];
        self.inner.read_exact(&mut nonce).await?;
        Ok(nonce)
    }
}

pub fn decrypted_size(enc_size: u64) -> u64 {
    let number_of_chunks = {
        let rhs = ENCRYPTED_CHUNK_SIZE as u64;
        let d = enc_size / rhs;
        let r = enc_size % rhs;
        if r > 0 && rhs > 0 {
            d + 1
        } else {
            d
        }
    };
    enc_size - ENCRYPTION_OVERHEAD as u64 * number_of_chunks - NONCE_SIZE as u64
}

pub fn encrypted_offset(dec_offset: u64) -> u64 {
    let number_of_chunks = dec_offset / PLAIN_CHUNK_SIZE as u64 + 1;
    dec_offset + ENCRYPTION_OVERHEAD as u64 * number_of_chunks + NONCE_SIZE as u64
}

pub struct ChunkedPosition {
    pub beginning_of_active_chunk: u64,
    pub offset_in_active_chunk: u64,
    pub active_chunk_counter: u64,
}

impl ChunkedPosition {
    pub fn new(plain_offset: u64) -> Self {
        let active_chunk_counter = plain_offset / PLAIN_CHUNK_SIZE as u64;
        let beginning_of_active_chunk =
            active_chunk_counter * ENCRYPTED_CHUNK_SIZE as u64 + NONCE_SIZE as u64;
        let start = encrypted_offset(plain_offset);
        let offset_in_active_chunk =
            start - (beginning_of_active_chunk + ENCRYPTION_OVERHEAD as u64);
        Self {
            beginning_of_active_chunk,
            offset_in_active_chunk,
            active_chunk_counter,
        }
    }
}