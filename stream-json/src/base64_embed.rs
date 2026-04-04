use base64::Engine;
use bytes::Bytes;
use futures_core::task::Poll;
use futures_io::AsyncRead;
use std::task::Context;

use crate::error::Error;
use crate::serde::{IntoSerializer, Serializer};
use crate::CHUNK_SIZE;

enum Base64EmbedURLState<T: AsyncRead + Unpin> {
    EmitHeader {
        reader: T,
        mime_type: String,
        expected_size: usize,
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

enum Base64EmbedFileState<T: AsyncRead + Unpin> {
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

pub struct Base64EmbedURL<T: AsyncRead + Unpin> {
    state: Base64EmbedURLState<T>,
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

impl<T: AsyncRead + Unpin> Base64EmbedURL<T> {
    pub fn new(reader: T, expected_size: usize, mime_type: String) -> Result<Self, Error> {
        Ok(Self {
            state: Base64EmbedURLState::EmitHeader {
                reader,
                mime_type: mime_type.clone(),
                expected_size,
            },
            mime_type,
            expected_size,
        })
    }

    pub fn size(&self) -> Option<usize> {
        let header = format!("data:{};base64,", self.mime_type);
        Some(header.len() + base64_len(self.expected_size) + 2)
    }
}

impl<T: AsyncRead + Unpin> Serializer for Base64EmbedURL<T> {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        loop {
            match std::mem::replace(&mut self.state, Base64EmbedURLState::Done) {
                Base64EmbedURLState::EmitHeader {
                    reader,
                    mime_type,
                    expected_size,
                } => {
                    let header = format!("\"data:{};base64,", mime_type);
                    self.state = Base64EmbedURLState::ReadAndEncode {
                        reader,
                        encoded: String::new(),
                        chunk_start: 0,
                        eof: false,
                        expected_size,
                        bytes_read: 0,
                    };
                    return Poll::Ready(Some(Ok(Bytes::from(header))));
                }
                Base64EmbedURLState::ReadAndEncode {
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
                        self.state = Base64EmbedURLState::ReadAndEncode {
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
                        self.state = Base64EmbedURLState::Done;
                        return Poll::Ready(Some(Ok(Bytes::from_static(b"\""))));
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
                            self.state = Base64EmbedURLState::Done;
                            return Poll::Ready(Some(Ok(Bytes::from_static(b"\""))));
                        }
                        Poll::Ready(Ok(n)) => {
                            let encode_len = if bytes_read + n > expected_size {
                                expected_size - bytes_read
                            } else {
                                n
                            };
                            bytes_read += n;
                            encoded = base64::engine::general_purpose::STANDARD
                                .encode(&buffer[..encode_len]);
                            chunk_start = 0;
                            let at_expected = bytes_read >= expected_size;
                            self.state = Base64EmbedURLState::ReadAndEncode {
                                reader,
                                encoded,
                                chunk_start,
                                eof: at_expected,
                                expected_size,
                                bytes_read,
                            };
                            if at_expected {
                                continue;
                            }
                        }
                        Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(Error::Io(e)))),
                        Poll::Pending => {
                            self.state = Base64EmbedURLState::ReadAndEncode {
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
                Base64EmbedURLState::Done => return Poll::Ready(None),
            }
        }
    }
}

impl<T: AsyncRead + Unpin> Unpin for Base64EmbedURL<T> {}

pub struct Base64EmbedFile<T: AsyncRead + Unpin> {
    state: Base64EmbedFileState<T>,
    expected_size: usize,
}

impl<T: AsyncRead + Unpin> Base64EmbedFile<T> {
    pub fn new(reader: T, expected_size: usize) -> Result<Self, Error> {
        Ok(Self {
            state: Base64EmbedFileState::ReadAndEncode {
                reader,
                encoded: String::new(),
                chunk_start: 0,
                eof: false,
                expected_size,
                bytes_read: 0,
            },
            expected_size,
        })
    }

    pub fn size(&self) -> Option<usize> {
        Some(base64_len(self.expected_size))
    }
}

impl<T: AsyncRead + Unpin> Serializer for Base64EmbedFile<T> {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        loop {
            match std::mem::replace(&mut self.state, Base64EmbedFileState::Done) {
                Base64EmbedFileState::ReadAndEncode {
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
                        self.state = Base64EmbedFileState::ReadAndEncode {
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
                            let encode_len = if bytes_read + n > expected_size {
                                expected_size - bytes_read
                            } else {
                                n
                            };
                            bytes_read += n;
                            encoded = base64::engine::general_purpose::STANDARD
                                .encode(&buffer[..encode_len]);
                            chunk_start = 0;
                            let at_expected = bytes_read >= expected_size;
                            self.state = Base64EmbedFileState::ReadAndEncode {
                                reader,
                                encoded,
                                chunk_start,
                                eof: at_expected,
                                expected_size,
                                bytes_read,
                            };
                            if at_expected {
                                continue;
                            }
                        }
                        Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(Error::Io(e)))),
                        Poll::Pending => {
                            self.state = Base64EmbedFileState::ReadAndEncode {
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
                Base64EmbedFileState::Done => return Poll::Ready(None),
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

impl<T: AsyncRead + Unpin> IntoSerializer for Base64EmbedURL<T> {
    type S = Base64EmbedURL<T>;

    fn into_serializer(self) -> Self::S {
        self
    }

    fn size(&self) -> Option<usize> {
        self.size()
    }
}
