//! The record layer, the PSOM operations and the parameter types (PO-03, PO-04, PO-05).
//!
//! Sources: [MS-PSOM] 20250520 §2.2.1 (records, pp. 19–21), §2.2.2 (operations, pp. 21–25),
//! §3.1.1.1 (types, pp. 26–27). Everything read here comes from the network and is bounded.

use crate::{decode_generic_int, decode_string, encode_generic_int, encode_string, Error};

/// Record types (§2.2.1.1, pp. 19–21).
pub const REC_CLOSE: u8 = 0x00;
pub const REC_SET_CHANNEL: u8 = 0x04;
pub const REC_BREAK: u8 = 0x06;
pub const REC_RPC: u8 = 0x16;
pub const REC_RPC_OPEN: u8 = 0x37;

/// Operation headers (§2.2.2.1.1–2.2.2.1.2, pp. 23–24).
pub const OP_CONNECT: u8 = 0x84;
pub const OP_CLOSE: u8 = 0x86;

/// Largest record body we accept (design): slide and annotation notices are small; a peer
/// announcing more is treated as hostile.
pub const MAX_RECORD: usize = 4 * 1024 * 1024;
/// Largest array we decode (design): a poll has at most 7 choices (§3.1.4.1.14, p. 48), user
/// and annotation batches stay far below.
pub const MAX_ARRAY: usize = 65_536;

/// One record of the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    Close,
    SetChannel(u32),
    /// The reason is ASCII (§2.2.1.1.3, p. 20).
    Break(String),
    Rpc(Vec<u8>),
    RpcOpen {
        channel: u32,
        body: Vec<u8>,
    },
}

fn u32_at(input: &[u8], at: usize) -> Option<u32> {
    let b: [u8; 4] = input.get(at..at.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_be_bytes(b))
}

fn body_len(input: &[u8], at: usize) -> Result<Option<usize>, Error> {
    match u32_at(input, at) {
        None => Ok(None),
        Some(n) => {
            let n = usize::try_from(n).map_err(|_| Error::BadFrame)?;
            if n > MAX_RECORD {
                return Err(Error::BadFrame);
            }
            Ok(Some(n))
        }
    }
}

impl Record {
    /// Reads one record from the start of `input`: `Ok(None)` while it is incomplete,
    /// otherwise the record and the bytes it took.
    pub fn decode(input: &[u8]) -> Result<Option<(Self, usize)>, Error> {
        let Some(&kind) = input.first() else {
            return Ok(None);
        };
        match kind {
            REC_CLOSE => Ok(Some((Record::Close, 1))),
            REC_SET_CHANNEL => Ok(u32_at(input, 1).map(|c| (Record::SetChannel(c), 5))),
            REC_BREAK | REC_RPC => {
                let Some(n) = body_len(input, 1)? else {
                    return Ok(None);
                };
                let Some(body) = input.get(5..5 + n) else {
                    return Ok(None);
                };
                let r = if kind == REC_BREAK {
                    Record::Break(body.iter().map(|&b| char::from(b)).collect())
                } else {
                    Record::Rpc(body.to_vec())
                };
                Ok(Some((r, 5 + n)))
            }
            REC_RPC_OPEN => {
                let (Some(channel), Some(n)) = (u32_at(input, 1), body_len(input, 5)?) else {
                    return Ok(None);
                };
                let Some(body) = input.get(9..9 + n) else {
                    return Ok(None);
                };
                Ok(Some((
                    Record::RpcOpen {
                        channel,
                        body: body.to_vec(),
                    },
                    9 + n,
                )))
            }
            _ => Err(Error::BadFrame),
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let len = |b: &[u8]| u32::try_from(b.len()).map_err(|_| Error::BadFrame);
        let mut out = Vec::new();
        match self {
            Record::Close => out.push(REC_CLOSE),
            Record::SetChannel(c) => {
                out.push(REC_SET_CHANNEL);
                out.extend_from_slice(&c.to_be_bytes());
            }
            Record::Break(reason) => {
                out.push(REC_BREAK);
                out.extend_from_slice(&len(reason.as_bytes())?.to_be_bytes());
                out.extend_from_slice(reason.as_bytes());
            }
            Record::Rpc(body) => {
                out.push(REC_RPC);
                out.extend_from_slice(&len(body)?.to_be_bytes());
                out.extend_from_slice(body);
            }
            Record::RpcOpen { channel, body } => {
                out.push(REC_RPC_OPEN);
                out.extend_from_slice(&channel.to_be_bytes());
                out.extend_from_slice(&len(body)?.to_be_bytes());
                out.extend_from_slice(body);
            }
        }
        Ok(out)
    }
}

/// A PSOM operation in an RPC record body (§2.2.2, p. 22).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// OP_CONNECT: the parent's proxy id as the sender knows it, the part name and the
    /// sender's interface hash of the child.
    Connect {
        parent: i64,
        part: String,
        hash: i64,
    },
    Close {
        proxy: i64,
    },
    /// A method call: the sender's proxy id, the method index and the encoded parameters.
    Call {
        proxy: i64,
        method: u8,
        params: Vec<u8>,
    },
}

impl Op {
    pub fn decode(body: &[u8]) -> Result<Self, Error> {
        match body.first() {
            Some(&OP_CONNECT) => {
                let mut r = Reader::new(body.get(1..).unwrap_or(&[]));
                let parent = r.int()?;
                let part = r.string()?;
                let hash = r.int()?;
                Ok(Op::Connect { parent, part, hash })
            }
            Some(&OP_CLOSE) => {
                let mut r = Reader::new(body.get(1..).unwrap_or(&[]));
                Ok(Op::Close { proxy: r.int()? })
            }
            Some(_) => {
                let (proxy, used) = decode_generic_int(body)?;
                let method = *body.get(used).ok_or(Error::BadFrame)?;
                let params = body.get(used + 1..).unwrap_or(&[]).to_vec();
                Ok(Op::Call {
                    proxy,
                    method,
                    params,
                })
            }
            None => Err(Error::Truncated),
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let mut out = Vec::new();
        match self {
            Op::Connect { parent, part, hash } => {
                out.push(OP_CONNECT);
                encode_generic_int(*parent, &mut out);
                encode_string(part, &mut out)?;
                encode_generic_int(*hash, &mut out);
            }
            Op::Close { proxy } => {
                out.push(OP_CLOSE);
                encode_generic_int(*proxy, &mut out);
            }
            Op::Call {
                proxy,
                method,
                params,
            } => {
                encode_generic_int(*proxy, &mut out);
                out.push(*method);
                out.extend_from_slice(params);
            }
        }
        Ok(out)
    }
}

/// Reads method parameters in declaration order (§2.2.2.2, p. 25; types §3.1.1.1, p. 26).
pub struct Reader<'a> {
    input: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input }
    }

    fn advance(&mut self, n: usize) {
        self.input = self.input.get(n..).unwrap_or(&[]);
    }

    /// Int32, Int64 and their unsigned forms: a GenericInt (§3.1.1.1.5, p. 27).
    pub fn int(&mut self) -> Result<i64, Error> {
        let (v, n) = decode_generic_int(self.input)?;
        self.advance(n);
        Ok(v)
    }

    pub fn i32(&mut self) -> Result<i32, Error> {
        i32::try_from(self.int()?).map_err(|_| Error::Overflow)
    }

    pub fn string(&mut self) -> Result<String, Error> {
        let (s, n) = decode_string(self.input)?;
        self.advance(n);
        Ok(s)
    }

    /// Boolean: 0x01 true, 0x00 false (§3.1.1.1.2, p. 26); other values are refused.
    pub fn boolean(&mut self) -> Result<bool, Error> {
        let b = self.byte()?;
        match b {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::BadFrame),
        }
    }

    pub fn byte(&mut self) -> Result<u8, Error> {
        let b = *self.input.first().ok_or(Error::Truncated)?;
        self.advance(1);
        Ok(b)
    }

    /// The element count of an array (§3.1.1.1.1, p. 26), bounded by [`MAX_ARRAY`] and by the
    /// bytes left (every element takes at least one byte).
    pub fn count(&mut self) -> Result<usize, Error> {
        let n = usize::try_from(self.int()?).map_err(|_| Error::BadFrame)?;
        if n > MAX_ARRAY || n > self.input.len() {
            return Err(Error::BadFrame);
        }
        Ok(n)
    }

    pub fn ints(&mut self) -> Result<Vec<i64>, Error> {
        let n = self.count()?;
        (0..n).map(|_| self.int()).collect()
    }

    pub fn strings(&mut self) -> Result<Vec<String>, Error> {
        let n = self.count()?;
        (0..n).map(|_| self.string()).collect()
    }

    pub fn bytes(&mut self) -> Result<Vec<u8>, Error> {
        let n = self.count()?;
        (0..n).map(|_| self.byte()).collect()
    }
}
