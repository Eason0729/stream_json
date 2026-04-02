use base64::Engine;
use bytes::Bytes;
use futures_core::task::Poll;
use futures_io::AsyncRead;
use std::task::Context;

use crate::error::Error;
use crate::serde::{IntoSerializer, Serializer};
use crate::CHUNK_SIZE;

pub const INFER_BUFFER_SIZE: usize = 512;

enum Base64EmbedState<T: AsyncRead + Unpin> {
    ReadForMime {
        reader: T,
        buffer: Vec<u8>,
        expected_size: usize,
        bytes_read: usize,
    },
    EncodeInitial {
        reader: T,
        encoded: String,
        mime_type: String,
        chunk_start: usize,
        header_emitted: bool,
        expected_size: usize,
        bytes_read: usize,
    },
    ReadAndEncode {
        reader: T,
        encoded: String,
        chunk_start: usize,
        eof: bool,
        expected_size: usize,
        bytes_read: usize,
    },
    Done,
}

pub struct Base64EmbedFile<T: AsyncRead + Unpin> {
    state: Base64EmbedState<T>,
    mime_type: Option<String>,
    expected_size: usize,
}

fn infer_mime_type(buffer: &[u8]) -> String {
    infer::get(buffer)
        .map(|kind| kind.mime_type().to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string())
}

fn base64_len(size: usize) -> usize {
    size.div_ceil(3) * 4
}

fn size_mismatch(expected: usize, actual: usize) -> Error {
    Error::Serialization(format!(
        "base64 embed size mismatch: expected {}, got {}",
        expected, actual
    ))
}

impl<T: AsyncRead + Unpin> Base64EmbedFile<T> {
    pub async fn new(mut reader: T, expected_size: usize) -> Result<Self, Error> {
        let mut buffer = Vec::with_capacity(INFER_BUFFER_SIZE);
        let mut tmp = vec![0u8; INFER_BUFFER_SIZE];
        let mut bytes_read = 0usize;

        while buffer.len() < INFER_BUFFER_SIZE && bytes_read < expected_size {
            let remaining = INFER_BUFFER_SIZE - buffer.len();
            let n = std::future::poll_fn(|cx| {
                let mut pinned = std::pin::Pin::new(&mut reader);
                AsyncRead::poll_read(pinned.as_mut(), cx, &mut tmp[..remaining])
            })
            .await?;
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&tmp[..n]);
            bytes_read += n;
        }

        if bytes_read > expected_size {
            return Err(size_mismatch(expected_size, bytes_read));
        }

        let mime_type = infer_mime_type(&buffer);
        let encoded = base64::engine::general_purpose::STANDARD.encode(&buffer);

        Ok(Self {
            state: Base64EmbedState::EncodeInitial {
                reader,
                encoded,
                mime_type: mime_type.clone(),
                chunk_start: 0,
                header_emitted: false,
                expected_size,
                bytes_read,
            },
            mime_type: Some(mime_type),
            expected_size,
        })
    }

    pub fn size(&self) -> Option<usize> {
        self.mime_type.as_ref().map(|mime| {
            let header = format!("data:{};base64,", mime);
            header.len() + base64_len(self.expected_size)
        })
    }
}

impl<T: AsyncRead + Unpin> Serializer for Base64EmbedFile<T> {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        loop {
            match std::mem::replace(&mut self.state, Base64EmbedState::Done) {
                Base64EmbedState::ReadForMime {
                    mut reader,
                    mut buffer,
                    expected_size,
                    mut bytes_read,
                } => {
                    let remaining = INFER_BUFFER_SIZE - buffer.len();
                    if remaining == 0 {
                        let mime_type = infer_mime_type(&buffer);
                        let encoded = base64::engine::general_purpose::STANDARD.encode(&buffer);
                        self.mime_type = Some(mime_type.clone());
                        self.state = Base64EmbedState::EncodeInitial {
                            reader,
                            encoded,
                            mime_type,
                            chunk_start: 0,
                            header_emitted: false,
                            expected_size,
                            bytes_read,
                        };
                        continue;
                    }

                    buffer.resize(buffer.len() + remaining, 0);
                    let buf = &mut buffer[bytes_read..];
                    match std::pin::Pin::new(&mut reader).poll_read(cx, buf) {
                        Poll::Ready(Ok(0)) => {
                            buffer.truncate(bytes_read);
                            let mime_type = infer_mime_type(&buffer);
                            let encoded = base64::engine::general_purpose::STANDARD.encode(&buffer);
                            if bytes_read != expected_size {
                                return Poll::Ready(Some(Err(size_mismatch(
                                    expected_size,
                                    bytes_read,
                                ))));
                            }
                            self.mime_type = Some(mime_type.clone());
                            self.state = Base64EmbedState::EncodeInitial {
                                reader,
                                encoded,
                                mime_type,
                                chunk_start: 0,
                                header_emitted: false,
                                expected_size,
                                bytes_read,
                            };
                            continue;
                        }
                        Poll::Ready(Ok(n)) => {
                            bytes_read += n;
                            buffer.truncate(bytes_read);
                            if bytes_read > expected_size {
                                return Poll::Ready(Some(Err(size_mismatch(
                                    expected_size,
                                    bytes_read,
                                ))));
                            }
                            if bytes_read >= INFER_BUFFER_SIZE {
                                let mime_type = infer_mime_type(&buffer);
                                let encoded =
                                    base64::engine::general_purpose::STANDARD.encode(&buffer);
                                self.mime_type = Some(mime_type.clone());
                                self.state = Base64EmbedState::EncodeInitial {
                                    reader,
                                    encoded,
                                    mime_type,
                                    chunk_start: 0,
                                    header_emitted: false,
                                    expected_size,
                                    bytes_read,
                                };
                                continue;
                            }
                            self.state = Base64EmbedState::ReadForMime {
                                reader,
                                buffer,
                                expected_size,
                                bytes_read,
                            };
                            return Poll::Ready(Some(Ok(Bytes::new())));
                        }
                        Poll::Ready(Err(e)) => {
                            return Poll::Ready(Some(Err(Error::Io(e))));
                        }
                        Poll::Pending => {
                            self.state = Base64EmbedState::ReadForMime {
                                reader,
                                buffer,
                                expected_size,
                                bytes_read,
                            };
                            return Poll::Pending;
                        }
                    }
                }
                Base64EmbedState::EncodeInitial {
                    reader,
                    encoded,
                    mime_type,
                    mut chunk_start,
                    mut header_emitted,
                    expected_size,
                    bytes_read,
                } => {
                    if !header_emitted {
                        header_emitted = true;
                        let header = format!("data:{};base64,", mime_type);
                        self.mime_type = Some(mime_type.clone());
                        self.state = Base64EmbedState::EncodeInitial {
                            reader,
                            encoded,
                            mime_type,
                            chunk_start,
                            header_emitted,
                            expected_size,
                            bytes_read,
                        };
                        return Poll::Ready(Some(Ok(Bytes::from(header))));
                    }

                    if chunk_start >= encoded.len() {
                        self.state = Base64EmbedState::ReadAndEncode {
                            reader,
                            encoded: String::new(),
                            chunk_start: 0,
                            eof: false,
                            expected_size,
                            bytes_read,
                        };
                        continue;
                    }

                    let end = std::cmp::min(chunk_start + CHUNK_SIZE, encoded.len());
                    let chunk = encoded[chunk_start..end].to_string();
                    chunk_start = end;
                    self.state = Base64EmbedState::EncodeInitial {
                        reader,
                        encoded,
                        mime_type,
                        chunk_start,
                        header_emitted,
                        expected_size,
                        bytes_read,
                    };
                    return Poll::Ready(Some(Ok(Bytes::from(chunk))));
                }
                Base64EmbedState::ReadAndEncode {
                    mut reader,
                    mut encoded,
                    mut chunk_start,
                    eof,
                    expected_size,
                    mut bytes_read,
                } => {
                    if chunk_start < encoded.len() {
                        let end = std::cmp::min(chunk_start + CHUNK_SIZE, encoded.len());
                        let chunk = encoded[chunk_start..end].to_string();
                        chunk_start = end;
                        self.state = Base64EmbedState::ReadAndEncode {
                            reader,
                            encoded,
                            chunk_start,
                            eof,
                            expected_size,
                            bytes_read,
                        };
                        return Poll::Ready(Some(Ok(Bytes::from(chunk))));
                    }

                    if eof {
                        if bytes_read != expected_size {
                            return Poll::Ready(Some(Err(size_mismatch(
                                expected_size,
                                bytes_read,
                            ))));
                        }
                        return Poll::Ready(None);
                    }

                    let mut buffer = vec![0u8; CHUNK_SIZE];
                    match std::pin::Pin::new(&mut reader).poll_read(cx, &mut buffer) {
                        Poll::Ready(Ok(0)) => {
                            if bytes_read != expected_size {
                                return Poll::Ready(Some(Err(size_mismatch(
                                    expected_size,
                                    bytes_read,
                                ))));
                            }
                            return Poll::Ready(None);
                        }
                        Poll::Ready(Ok(n)) => {
                            bytes_read += n;
                            if bytes_read > expected_size {
                                return Poll::Ready(Some(Err(size_mismatch(
                                    expected_size,
                                    bytes_read,
                                ))));
                            }
                            encoded =
                                base64::engine::general_purpose::STANDARD.encode(&buffer[..n]);
                            chunk_start = 0;
                            self.state = Base64EmbedState::ReadAndEncode {
                                reader,
                                encoded,
                                chunk_start,
                                eof: false,
                                expected_size,
                                bytes_read,
                            };
                            continue;
                        }
                        Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(Error::Io(e)))),
                        Poll::Pending => {
                            self.state = Base64EmbedState::ReadAndEncode {
                                reader,
                                encoded,
                                chunk_start,
                                eof,
                                expected_size,
                                bytes_read,
                            };
                            return Poll::Pending;
                        }
                    }
                }
                Base64EmbedState::Done => return Poll::Ready(None),
            }
        }
    }
}

impl<T: AsyncRead + Unpin> Unpin for Base64EmbedFile<T> {}

impl<T: AsyncRead + Unpin> IntoSerializer for Base64EmbedFile<T> {
    type S = Base64EmbedFile<T>;

    fn into_serializer(self) -> Self::S {
        self
    }

    fn size(&self) -> Option<usize> {
        self.size()
    }
}
