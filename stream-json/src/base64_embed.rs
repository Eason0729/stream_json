use base64::Engine;
use bytes::Bytes;

use crate::error::Error;
use crate::serde::{IntoSerializer, Serializer};
use crate::CHUNK_SIZE;

enum Base64EmbedURLState {
    EmitHeader,
    EmitData { encoded: String, chunk_start: usize },
    Done,
}

enum Base64EmbedFileState {
    EmitStart,
    EmitData { encoded: String, chunk_start: usize },
    Done,
}

pub struct Base64EmbedURL<T: AsRef<[u8]>> {
    state: Base64EmbedURLState,
    data: T,
    mime_type: String,
    expected_size: usize,
}

fn size_mismatch(expected: usize, actual: usize) -> Error {
    Error::Serialization(format!(
        "base64 embed size mismatch: expected {}, got {}",
        expected, actual
    ))
}

fn base64_len(size: usize) -> usize {
    size.div_ceil(3) * 4
}

impl<T: AsRef<[u8]>> Base64EmbedURL<T> {
    pub fn new(data: T, expected_size: usize, mime_type: String) -> Result<Self, Error> {
        Ok(Self {
            state: Base64EmbedURLState::EmitHeader,
            data,
            mime_type,
            expected_size,
        })
    }

    pub fn size(&self) -> Option<usize> {
        let header = format!("data:{};base64,", self.mime_type);
        Some(header.len() + base64_len(self.expected_size) + 2)
    }
}

impl<T: AsRef<[u8]>> Serializer for Base64EmbedURL<T> {
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<Bytes, Error>>> {
        use std::task::Poll;
        loop {
            match std::mem::replace(&mut self.state, Base64EmbedURLState::Done) {
                Base64EmbedURLState::EmitHeader => {
                    let header = format!("\"data:{};base64,", self.mime_type);
                    let data = self.data.as_ref();
                    if data.len() < self.expected_size {
                        return Poll::Ready(Some(Err(size_mismatch(
                            self.expected_size,
                            data.len(),
                        ))));
                    }
                    let encode_len = std::cmp::min(data.len(), self.expected_size);
                    let encoded =
                        base64::engine::general_purpose::STANDARD.encode(&data[..encode_len]);
                    self.state = Base64EmbedURLState::EmitData {
                        encoded,
                        chunk_start: 0,
                    };
                    return Poll::Ready(Some(Ok(Bytes::from(header))));
                }
                Base64EmbedURLState::EmitData {
                    encoded,
                    chunk_start,
                } => {
                    if chunk_start >= encoded.len() {
                        self.state = Base64EmbedURLState::Done;
                        return Poll::Ready(Some(Ok(Bytes::from_static(b"\""))));
                    }
                    let end = std::cmp::min(chunk_start + CHUNK_SIZE, encoded.len());
                    let chunk = encoded[chunk_start..end].to_string();
                    self.state = Base64EmbedURLState::EmitData {
                        encoded,
                        chunk_start: end,
                    };
                    return Poll::Ready(Some(Ok(Bytes::from(chunk))));
                }
                Base64EmbedURLState::Done => return Poll::Ready(None),
            }
        }
    }
}

pub struct Base64EmbedFile<T: AsRef<[u8]>> {
    state: Base64EmbedFileState,
    data: T,
    expected_size: usize,
}

impl<T: AsRef<[u8]>> Base64EmbedFile<T> {
    pub fn new(data: T, expected_size: usize) -> Result<Self, Error> {
        Ok(Self {
            state: Base64EmbedFileState::EmitStart,
            data,
            expected_size,
        })
    }

    pub fn size(&self) -> Option<usize> {
        Some(base64_len(self.expected_size) + 2)
    }
}

impl<T: AsRef<[u8]>> Serializer for Base64EmbedFile<T> {
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<Bytes, Error>>> {
        use std::task::Poll;
        loop {
            match std::mem::replace(&mut self.state, Base64EmbedFileState::Done) {
                Base64EmbedFileState::EmitStart => {
                    let data = self.data.as_ref();
                    if data.len() < self.expected_size {
                        return Poll::Ready(Some(Err(size_mismatch(
                            self.expected_size,
                            data.len(),
                        ))));
                    }
                    let encode_len = std::cmp::min(data.len(), self.expected_size);
                    let encoded =
                        base64::engine::general_purpose::STANDARD.encode(&data[..encode_len]);
                    self.state = Base64EmbedFileState::EmitData {
                        encoded,
                        chunk_start: 0,
                    };
                    return Poll::Ready(Some(Ok(Bytes::from_static(b"\""))));
                }
                Base64EmbedFileState::EmitData {
                    encoded,
                    chunk_start,
                } => {
                    if chunk_start >= encoded.len() {
                        self.state = Base64EmbedFileState::Done;
                        return Poll::Ready(Some(Ok(Bytes::from_static(b"\""))));
                    }
                    let end = std::cmp::min(chunk_start + CHUNK_SIZE, encoded.len());
                    let chunk = encoded[chunk_start..end].to_string();
                    self.state = Base64EmbedFileState::EmitData {
                        encoded,
                        chunk_start: end,
                    };
                    return Poll::Ready(Some(Ok(Bytes::from(chunk))));
                }
                Base64EmbedFileState::Done => return Poll::Ready(None),
            }
        }
    }
}

impl<T: AsRef<[u8]> + Unpin> IntoSerializer for Base64EmbedFile<T> {
    type S = Base64EmbedFile<T>;

    fn into_serializer(self) -> Self::S {
        self
    }

    fn size(&self) -> Option<usize> {
        self.size()
    }
}

impl<T: AsRef<[u8]> + Unpin> IntoSerializer for Base64EmbedURL<T> {
    type S = Base64EmbedURL<T>;

    fn into_serializer(self) -> Self::S {
        self
    }

    fn size(&self) -> Option<usize> {
        self.size()
    }
}
