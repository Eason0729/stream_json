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
    },
    EncodeInitial {
        reader: T,
        encoded: String,
        mime_type: String,
        chunk_start: usize,
        header_emitted: bool,
    },
    ReadAndEncode {
        reader: T,
        encoded: String,
        chunk_start: usize,
        eof: bool,
    },
    Done,
}

pub struct Base64EmbedFile<T: AsyncRead + Unpin> {
    state: Base64EmbedState<T>,
}

impl<T: AsyncRead + Unpin> Base64EmbedFile<T> {
    pub fn new(reader: T) -> Self {
        Self {
            state: Base64EmbedState::ReadForMime {
                reader,
                buffer: Vec::with_capacity(INFER_BUFFER_SIZE),
            },
        }
    }
}

fn infer_mime_type(buffer: &[u8]) -> String {
    if let Some(kind) = infer::get(buffer) {
        kind.mime_type().to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

impl<T: AsyncRead + Unpin> Serializer for Base64EmbedFile<T> {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        loop {
            match std::mem::replace(&mut self.state, Base64EmbedState::Done) {
                Base64EmbedState::ReadForMime {
                    mut reader,
                    mut buffer,
                } => {
                    let start_pos = buffer.len();
                    let remaining = INFER_BUFFER_SIZE - start_pos;
                    if remaining == 0 {
                        let mime_type = infer_mime_type(&buffer);
                        let encoded = base64::engine::general_purpose::STANDARD.encode(&buffer);
                        self.state = Base64EmbedState::EncodeInitial {
                            reader,
                            encoded,
                            mime_type,
                            chunk_start: 0,
                            header_emitted: false,
                        };
                        continue;
                    }

                    buffer.resize(start_pos + remaining, 0);
                    let buf: &mut [u8] = &mut buffer[start_pos..];

                    let mut pin_reader = std::pin::Pin::new(&mut reader);
                    match AsyncRead::poll_read(pin_reader.as_mut(), cx, buf) {
                        Poll::Ready(Ok(0)) => {
                            buffer.resize(start_pos, 0);
                            let mime_type = infer_mime_type(&buffer);
                            let encoded = base64::engine::general_purpose::STANDARD.encode(&buffer);
                            self.state = Base64EmbedState::EncodeInitial {
                                reader,
                                encoded,
                                mime_type,
                                chunk_start: 0,
                                header_emitted: false,
                            };
                            continue;
                        }
                        Poll::Ready(Ok(n)) => {
                            let filled = n;
                            buffer.resize(start_pos + filled, 0);

                            let mime_detected = filled < remaining && filled > 0;
                            let has_enough = buffer.len() >= INFER_BUFFER_SIZE;

                            if mime_detected || has_enough {
                                let mime_type = infer_mime_type(&buffer);
                                let encoded =
                                    base64::engine::general_purpose::STANDARD.encode(&buffer);
                                self.state = Base64EmbedState::EncodeInitial {
                                    reader,
                                    encoded,
                                    mime_type,
                                    chunk_start: 0,
                                    header_emitted: false,
                                };
                                continue;
                            }

                            self.state = Base64EmbedState::ReadForMime { reader, buffer };
                            return Poll::Ready(Some(Ok(Bytes::new())));
                        }
                        Poll::Ready(Err(e)) => {
                            self.state = Base64EmbedState::Done;
                            return Poll::Ready(Some(Err(Error::Io(e))));
                        }
                        Poll::Pending => {
                            self.state = Base64EmbedState::ReadForMime { reader, buffer };
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
                } => {
                    if !header_emitted {
                        header_emitted = true;
                        let header = format!("data:{};base64,", mime_type);
                        self.state = Base64EmbedState::EncodeInitial {
                            reader,
                            encoded,
                            mime_type,
                            chunk_start,
                            header_emitted,
                        };
                        return Poll::Ready(Some(Ok(Bytes::from(header))));
                    }

                    if chunk_start >= encoded.len() {
                        self.state = Base64EmbedState::ReadAndEncode {
                            reader,
                            encoded: String::new(),
                            chunk_start: 0,
                            eof: false,
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
                    };
                    return Poll::Ready(Some(Ok(Bytes::from(chunk))));
                }

                Base64EmbedState::ReadAndEncode {
                    mut reader,
                    mut encoded,
                    mut chunk_start,
                    eof,
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
                        };
                        return Poll::Ready(Some(Ok(Bytes::from(chunk))));
                    }

                    if eof {
                        self.state = Base64EmbedState::Done;
                        return Poll::Ready(None);
                    }

                    let mut buffer = vec![0u8; CHUNK_SIZE];
                    let buf: &mut [u8] = &mut buffer;

                    let mut pin_reader = std::pin::Pin::new(&mut reader);
                    match AsyncRead::poll_read(pin_reader.as_mut(), cx, buf) {
                        Poll::Ready(Ok(0)) => {
                            self.state = Base64EmbedState::Done;
                            return Poll::Ready(None);
                        }
                        Poll::Ready(Ok(n)) => {
                            encoded =
                                base64::engine::general_purpose::STANDARD.encode(&buffer[..n]);
                            chunk_start = 0;
                            self.state = Base64EmbedState::ReadAndEncode {
                                reader,
                                encoded,
                                chunk_start,
                                eof: false,
                            };
                            continue;
                        }
                        Poll::Ready(Err(e)) => {
                            self.state = Base64EmbedState::Done;
                            return Poll::Ready(Some(Err(Error::Io(e))));
                        }
                        Poll::Pending => {
                            self.state = Base64EmbedState::ReadAndEncode {
                                reader,
                                encoded,
                                chunk_start,
                                eof,
                            };
                            return Poll::Pending;
                        }
                    }
                }

                Base64EmbedState::Done => {
                    return Poll::Ready(None);
                }
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
}

impl IntoSerializer for Box<dyn AsyncRead + Unpin> {
    type S = Base64EmbedFile<Box<dyn AsyncRead + Unpin>>;

    fn into_serializer(self) -> Self::S {
        Base64EmbedFile::new(self)
    }
}
