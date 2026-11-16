//! The PSOM client: authentication, versioning, the root channel and the meeting content
//! objects (PO-02, PO-06, PO-07, PO-08). Sans I/O: bytes in, bytes and events out.
//!
//! Sources: [MS-PSOM] 20250520 §3.2.3 (connection, pp. 52–56), §3.2.4.1 (client methods,
//! pp. 56–58), §3.1.4.1 (interfaces, pp. 28–51), §2.2.2 (proxy ids, pp. 22–23), examples §4
//! (pp. 92–100).
//!
//! Proxy ids: the peer that connects an object numbers its connects 1, 2, 3 …; the other side
//! knows the object under the negated number, and every proxy id read from the network is
//! negated before the lookup (§2.2.2, p. 22). The root object of a channel has id 0.

use crate::iface::{self, Kind, Version};
use crate::wire::{Op, Reader, Record};
use crate::{encode_generic_int, encode_string, Error};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Signature of the connection join and of the server's confirmation (§3.2.3.1.1.2, p. 54).
pub const SIGNATURE: [u8; 4] = [0x70, 0x77, 0x32, 0x00];
/// Keep-alive interval of `ping` on ConnMgr (§3.2.2, p. 52).
pub const PING_INTERVAL_MS: u64 = 30_000;
/// Longest sAuthId we send (design; the example token has 32 characters, p. 92).
pub const MAX_AUTH_ID: usize = 1024;
/// Most objects we keep (design): content, extensions and annotation containers of one meeting.
pub const MAX_OBJECTS: usize = 4096;
/// Most annotations we count per container (design).
pub const MAX_ANNOTATIONS: usize = 65_536;

// ConnMgr server methods (§3.3.3.1.3, p. 80).
const SRV_VERSION: u8 = 1;
const SRV_ADD_PROTOCOL: u8 = 2;
const SRV_DONE_PROTOCOLS: u8 = 3;
const SRV_LOOKUP: u8 = 5;
const SRV_PING: u8 = 6;
// PollContent server method sVote (§3.1.4.1.14.1, p. 48).
const POLL_VOTE: u8 = 5;

/// What the meeting content tells us.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Versioning finished, the root channel is open.
    Connected,
    /// cMeetingReady: the meeting state was sent (§3.2.4.1.1.1, p. 56).
    Ready,
    UrlBase(String),
    UsersAdded(Vec<(i64, String, String)>),
    UsersRemoved(Vec<i64>),
    ContentAdded {
        content: i64,
        kind: String,
    },
    ContentRemoved(i64),
    ActiveContent(i64),
    ActivePresenter(i64),
    Title {
        content: i64,
        title: String,
    },
    Presented {
        content: i64,
        presented: bool,
        presenter: i64,
    },
    Owner {
        content: i64,
        user: i64,
    },
    FileInfo {
        content: i64,
        name: String,
        size: i64,
    },
    PollQuestion {
        content: i64,
        question: String,
        choices: Vec<String>,
    },
    PollResults {
        content: i64,
        results: Vec<i64>,
    },
    PollOpen {
        content: i64,
        open: bool,
    },
    PollResultsVisible {
        content: i64,
        visible: bool,
    },
    PollLocalVote {
        content: i64,
        choice: i64,
    },
    /// The number of annotations (ink, shapes, text) on a whiteboard or slide set.
    Annotations {
        content: i64,
        count: usize,
    },
    Slides {
        content: i64,
        count: i64,
    },
    SlideLocation {
        content: i64,
        location: String,
    },
    NotesTitles {
        content: i64,
        notebook: String,
        section: String,
        page: String,
    },
    /// The server closed the connection (Close, or Break with its reason).
    Closed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    WaitAuth,
    Versioning,
    Open,
    Closed,
}

#[derive(Debug, Clone)]
struct Object {
    kind: Kind,
    /// The content this object belongs to (Content, its extension, an annotation container).
    content: Option<i64>,
}

#[derive(Debug, Clone, Default)]
struct ContentEntry {
    kind: String,
    proxy: Option<i64>,
    extension: Option<i64>,
}

pub struct Client {
    state: State,
    buf: Vec<u8>,
    out: Vec<u8>,
    rx_channel: u32,
    tx_channel: u32,
    server_protocols: BTreeMap<String, Vec<(i64, i64)>>,
    server_done: bool,
    negotiated: BTreeMap<Kind, Version>,
    objects: HashMap<i64, Object>,
    server_connects: i64,
    client_connects: i64,
    contents: BTreeMap<i64, ContentEntry>,
    annotations: HashMap<i64, BTreeSet<i64>>,
    last_ping: u64,
}

fn call(proxy: i64, method: u8, params: Vec<u8>) -> Op {
    Op::Call {
        proxy,
        method,
        params,
    }
}

impl Client {
    /// A new client and the connection join to send first: signature, authentication version
    /// 0, the length and the bytes of sAuthId (§3.2.3.1.1.1–3.2.3.1.1.2, pp. 53–54). Only the
    /// `tls` mode is implemented; `fwdtls` would need the proxy header first (open).
    pub fn new(auth_id: &str, now_ms: u64) -> Result<(Self, Vec<u8>), Error> {
        if auth_id.is_empty() || auth_id.len() > MAX_AUTH_ID || !auth_id.is_ascii() {
            return Err(Error::BadFrame);
        }
        let mut first = SIGNATURE.to_vec();
        first.extend_from_slice(&0u32.to_be_bytes());
        let len = u32::try_from(auth_id.len()).map_err(|_| Error::BadFrame)?;
        first.extend_from_slice(&len.to_be_bytes());
        first.extend_from_slice(auth_id.as_bytes());
        let me = Self {
            state: State::WaitAuth,
            buf: Vec::new(),
            out: Vec::new(),
            rx_channel: 0,
            tx_channel: 0,
            server_protocols: BTreeMap::new(),
            server_done: false,
            negotiated: BTreeMap::new(),
            objects: HashMap::new(),
            server_connects: 0,
            client_connects: 0,
            contents: BTreeMap::new(),
            annotations: HashMap::new(),
            last_ping: now_ms,
        };
        Ok((me, first))
    }

    /// Bytes to send since the last call.
    pub fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    pub fn is_open(&self) -> bool {
        self.state == State::Open
    }

    /// The negotiated version of an interface.
    pub fn negotiated(&self, kind: Kind) -> Option<i32> {
        self.negotiated.get(&kind).map(|v| v.version)
    }

    fn record(&mut self, r: &Record) -> Result<(), Error> {
        let bytes = r.encode()?;
        self.out.extend_from_slice(&bytes);
        Ok(())
    }

    fn send_on(&mut self, channel: u32, op: &Op) -> Result<(), Error> {
        if self.tx_channel != channel {
            self.record(&Record::SetChannel(channel))?;
            self.tx_channel = channel;
        }
        let body = op.encode()?;
        self.record(&Record::Rpc(body))
    }

    /// The keep-alive: `ping` on ConnMgr on channel zero every 30 s (§3.1.6, p. 51).
    pub fn tick(&mut self, now_ms: u64) -> Result<(), Error> {
        if self.state != State::Open || now_ms.saturating_sub(self.last_ping) < PING_INTERVAL_MS {
            return Ok(());
        }
        self.last_ping = now_ms;
        let back = self.tx_channel;
        self.send_on(0, &call(0, SRV_PING, Vec::new()))?;
        if back != 0 {
            self.record(&Record::SetChannel(back))?;
            self.tx_channel = back;
        }
        Ok(())
    }

    /// Votes in a poll: sVote with the choice 0…6, or −1 for no vote (§3.1.4.1.14, p. 48).
    pub fn vote(&mut self, content: i64, choice: i64) -> Result<(), Error> {
        if !(-1..=6).contains(&choice) {
            return Err(Error::BadFrame);
        }
        let ext = self
            .contents
            .get(&content)
            .and_then(|c| c.extension)
            .filter(|p| self.objects.get(p).is_some_and(|o| o.kind == Kind::Poll))
            .ok_or(Error::BadFrame)?;
        let mut params = Vec::new();
        encode_generic_int(choice, &mut params);
        self.send_on(2, &call(ext, POLL_VOTE, params))
    }

    /// Closes the connection: Close records on the root channel, then on channel zero
    /// (§2.2.1.1.1, p. 20).
    pub fn close(&mut self) -> Result<(), Error> {
        if self.state == State::Open {
            self.record(&Record::SetChannel(2))?;
            self.record(&Record::Close)?;
        }
        self.record(&Record::SetChannel(0))?;
        self.record(&Record::Close)?;
        self.state = State::Closed;
        Ok(())
    }

    /// Bytes from the server. An error means the connection must end (§3.1.5, p. 51).
    pub fn on_bytes(&mut self, data: &[u8], now_ms: u64) -> Result<Vec<Event>, Error> {
        if self.state == State::Closed {
            return Ok(Vec::new());
        }
        if self.buf.len().saturating_add(data.len()) > crate::wire::MAX_RECORD + 16 {
            return Err(Error::BadFrame);
        }
        self.buf.extend_from_slice(data);
        let mut events = Vec::new();
        if self.state == State::WaitAuth {
            let Some(head) = self.buf.get(..4) else {
                return Ok(events);
            };
            if head != SIGNATURE {
                return Err(Error::BadFrame);
            }
            self.buf.drain(..4);
            self.state = State::Versioning;
            self.last_ping = now_ms;
            self.start_versioning()?;
        }
        while let Some((record, used)) = Record::decode(&self.buf)? {
            self.buf.drain(..used);
            match record {
                Record::SetChannel(c) => self.rx_channel = c,
                Record::Close => {
                    self.state = State::Closed;
                    events.push(Event::Closed(String::new()));
                    break;
                }
                Record::Break(reason) => {
                    self.state = State::Closed;
                    events.push(Event::Closed(reason));
                    break;
                }
                Record::Rpc(body) => self.on_op(&Op::decode(&body)?, &mut events)?,
                // Only the client opens a channel (§2.2.1.1.5, p. 21).
                Record::RpcOpen { .. } => return Err(Error::BadFrame),
            }
        }
        Ok(events)
    }

    /// SetChannel 0, then version, an addProtocol per interface and doneProtocols
    /// (§3.2.3.1.2, pp. 54–55; example §4.1.3–4.1.4, pp. 93–95).
    fn start_versioning(&mut self) -> Result<(), Error> {
        self.record(&Record::SetChannel(0))?;
        self.tx_channel = 0;
        let mut p = Vec::new();
        encode_generic_int(iface::CONNMGR_CLIENT_HASH, &mut p);
        self.send_on(0, &call(0, SRV_VERSION, p))?;
        for i in iface::INTERFACES {
            let mut p = Vec::new();
            encode_string(i.name, &mut p)?;
            let mut versions: Vec<(i64, i64)> = i
                .versions
                .iter()
                .map(|v| (i64::from(v.version), v.sum()))
                .collect();
            if i.kind == Kind::Meeting {
                versions.insert(0, (1, iface::MEETING_V1_SUM));
            }
            let count = i64::try_from(versions.len()).map_err(|_| Error::Overflow)?;
            encode_generic_int(count, &mut p);
            for (v, _) in &versions {
                encode_generic_int(*v, &mut p);
            }
            encode_generic_int(count, &mut p);
            for (_, h) in &versions {
                encode_generic_int(*h, &mut p);
            }
            self.send_on(0, &call(0, SRV_ADD_PROTOCOL, p))?;
        }
        self.send_on(0, &call(0, SRV_DONE_PROTOCOLS, Vec::new()))
    }

    /// After both doneProtocols: per interface the highest version both sides announce with
    /// the same hash; the same version with another hash ends the connection (§3.2.3.1.2,
    /// p. 54). Then RPCOpen of channel 2 with `lookup`, and SetChannel 2 (§3.2.3.1.4, p. 55;
    /// example §4.2, p. 96 – the ignored lookup arguments are taken from it).
    fn finish_versioning(&mut self, events: &mut Vec<Event>) -> Result<(), Error> {
        for i in iface::INTERFACES {
            let theirs = self.server_protocols.get(i.name);
            let mut best: Option<Version> = None;
            for v in i.versions {
                let Some(theirs) = theirs else { break };
                for &(version, hash) in theirs {
                    if version == i64::from(v.version) {
                        if hash != v.sum() {
                            return Err(Error::HashMismatch);
                        }
                        if best.is_none_or(|b| b.version < v.version) {
                            best = Some(*v);
                        }
                    }
                }
            }
            if let Some(b) = best {
                self.negotiated.insert(i.kind, b);
            }
        }
        for required in [
            Kind::Meeting,
            Kind::ContentManager,
            Kind::ContentUserManager,
        ] {
            if !self.negotiated.contains_key(&required) {
                return Err(Error::HashMismatch);
            }
        }
        let mut p = Vec::new();
        encode_string("TODO-ryanfa-remove", &mut p)?;
        encode_string("NotUsed", &mut p)?;
        encode_generic_int(-7_932_100_958_924_279_543, &mut p);
        let body = call(0, SRV_LOOKUP, p).encode()?;
        self.record(&Record::RpcOpen { channel: 2, body })?;
        self.record(&Record::SetChannel(2))?;
        self.tx_channel = 2;
        self.objects.insert(
            0,
            Object {
                kind: Kind::Meeting,
                content: None,
            },
        );
        self.state = State::Open;
        events.push(Event::Connected);
        Ok(())
    }

    fn on_op(&mut self, op: &Op, events: &mut Vec<Event>) -> Result<(), Error> {
        if self.rx_channel == 0 {
            return match op {
                Op::Call {
                    proxy: 0,
                    method,
                    params,
                } => self.on_connmgr(*method, params, events),
                _ => Err(Error::BadFrame),
            };
        }
        if self.rx_channel != 2 || self.state != State::Open {
            return Err(Error::BadFrame);
        }
        match op {
            Op::Connect { parent, part, hash } => self.on_connect(-*parent, part, *hash),
            Op::Close { proxy } => {
                let id = -*proxy;
                if let Some(o) = self.objects.remove(&id) {
                    if o.kind == Kind::Content {
                        if let Some(c) = o.content.and_then(|c| self.contents.get_mut(&c)) {
                            c.proxy = None;
                        }
                    }
                    self.annotations.remove(&id);
                }
                Ok(())
            }
            Op::Call {
                proxy,
                method,
                params,
            } => self.on_call(-*proxy, *method, params, events),
        }
    }

    fn on_connmgr(
        &mut self,
        method: u8,
        params: &[u8],
        events: &mut Vec<Event>,
    ) -> Result<(), Error> {
        let mut r = Reader::new(params);
        match (self.state, method) {
            // version(stubHash): the server's ConnMgr hash (§3.2.3.1.3.1.1, p. 55).
            (State::Versioning, 1) => {
                if r.int()? != iface::CONNMGR_SERVER_HASH {
                    return Err(Error::HashMismatch);
                }
                Ok(())
            }
            (State::Versioning, 2) => {
                let name = r.string()?;
                let versions = r.ints()?;
                let hashes = r.ints()?;
                if versions.len() != hashes.len() || self.server_protocols.len() >= 256 {
                    return Err(Error::BadFrame);
                }
                self.server_protocols
                    .insert(name, versions.into_iter().zip(hashes).collect());
                Ok(())
            }
            (State::Versioning, 3) if !self.server_done => {
                self.server_done = true;
                self.finish_versioning(events)
            }
            // ping (§3.2.3.1.3.1.4, p. 55): a no-op, in any state after authentication.
            (_, 4) => Ok(()),
            _ => Err(Error::BadFrame),
        }
    }

    fn version_of(&self, kind: Kind) -> Option<Version> {
        self.negotiated.get(&kind).copied()
    }

    fn add_object(&mut self, id: i64, kind: Kind, content: Option<i64>) -> Result<(), Error> {
        if self.objects.len() >= MAX_OBJECTS || self.objects.contains_key(&id) {
            return Err(Error::BadFrame);
        }
        self.objects.insert(id, Object { kind, content });
        Ok(())
    }

    /// A child the server connects (§2.2.2.1.1, p. 23): the parts of Meeting (§3.1.4.1.6.2),
    /// ContentManager (§3.1.4.1.4.2), UploadManager (§3.1.4.1.8.2) and the annotation container
    /// of a whiteboard or slide set (§3.1.4.1.13.2, §3.1.4.1.11.2). The hash must be the server
    /// interface hash of the negotiated version.
    ///
    /// Part names are compared without regard to case: the Children lists write
    /// "ContentUserManager", the example connects "contentUserManager" (§4.3, p. 97).
    fn on_connect(&mut self, parent: i64, part: &str, hash: i64) -> Result<(), Error> {
        let parent_obj = self
            .objects
            .get(&parent)
            .cloned()
            .ok_or(Error::UnknownObject)?;
        let part = part.to_ascii_lowercase();
        let kind = match (parent_obj.kind, part.as_str()) {
            (Kind::Meeting, "contentmanager") => Kind::ContentManager,
            (Kind::Meeting, "contentusermanager") => Kind::ContentUserManager,
            (Kind::ContentManager, "uploadmanager") => Kind::UploadManager,
            (Kind::UploadManager, "uploadstreams") => Kind::UploadStream,
            (Kind::Whiteboard | Kind::Ppt, "annotationcontainer") => Kind::AnnotationContainer,
            _ => return Err(Error::UnknownObject),
        };
        let v = self.version_of(kind).ok_or(Error::UnknownObject)?;
        if v.server_hash != hash {
            return Err(Error::HashMismatch);
        }
        self.server_connects = self.server_connects.checked_add(1).ok_or(Error::Overflow)?;
        let id = -self.server_connects;
        self.add_object(id, kind, parent_obj.content)
    }

    /// A child we connect: our next number, the parent under our id, our client interface
    /// hash (§2.2.2.1.1, p. 23).
    fn connect(
        &mut self,
        parent: i64,
        part: &str,
        kind: Kind,
        content: i64,
    ) -> Result<Option<i64>, Error> {
        let Some(v) = self.version_of(kind) else {
            return Ok(None);
        };
        self.client_connects = self.client_connects.checked_add(1).ok_or(Error::Overflow)?;
        let id = self.client_connects;
        self.add_object(id, kind, Some(content))?;
        self.send_on(
            2,
            &Op::Connect {
                parent,
                part: part.to_string(),
                hash: v.client_hash,
            },
        )?;
        Ok(Some(id))
    }

    fn on_call(
        &mut self,
        id: i64,
        method: u8,
        params: &[u8],
        events: &mut Vec<Event>,
    ) -> Result<(), Error> {
        let obj = self.objects.get(&id).cloned().ok_or(Error::UnknownObject)?;
        let v = self.version_of(obj.kind).ok_or(Error::UnknownObject)?;
        if method == 0 || method > v.client_methods {
            return Err(Error::UnknownMethod);
        }
        let mut r = Reader::new(params);
        let content = obj.content.unwrap_or(0);
        match (obj.kind, method) {
            // Meeting: cMeetingReady 1, cSetInfo 2 (ignored), cSetServerTime 3, cSetUrlBase 4
            // (§3.2.4.1.1.1, p. 56; index 4 confirmed by the example §4.3, p. 97).
            (Kind::Meeting, 1) => events.push(Event::Ready),
            (Kind::Meeting, 4) => events.push(Event::UrlBase(r.string()?)),
            // ContentUserManager: cUsersAdded 1, cUsersRemoved 2 (§3.2.4.1.2.1, p. 57).
            (Kind::ContentUserManager, 1) => {
                let ids = r.ints()?;
                let uris = r.strings()?;
                let names = r.strings()?;
                if ids.len() != uris.len() || ids.len() != names.len() {
                    return Err(Error::BadFrame);
                }
                let users = ids
                    .into_iter()
                    .zip(uris)
                    .zip(names)
                    .map(|((i, u), n)| (i, u, n))
                    .collect();
                events.push(Event::UsersAdded(users));
            }
            (Kind::ContentUserManager, 2) => events.push(Event::UsersRemoved(r.ints()?)),
            // ContentManager: cContentAdded 1, cContentRemoved 4, cSetActiveContent 6,
            // cSetActivePresenter 7 (§3.2.4.1.3.1, pp. 57–58). Content is connected by us as
            // "content.<id>" (§3.1.4.1.4.2, p. 33).
            (Kind::ContentManager, 1) => {
                let cid = r.int()?;
                let kind = r.string()?;
                if self.contents.len() >= MAX_OBJECTS {
                    return Err(Error::BadFrame);
                }
                let proxy = self.connect(id, &format!("content.{cid}"), Kind::Content, cid)?;
                self.contents.insert(
                    cid,
                    ContentEntry {
                        kind: kind.clone(),
                        proxy,
                        extension: None,
                    },
                );
                events.push(Event::ContentAdded { content: cid, kind });
            }
            (Kind::ContentManager, 4) => {
                let cid = r.int()?;
                if let Some(c) = self.contents.remove(&cid) {
                    for p in c.proxy.into_iter().chain(c.extension) {
                        self.objects.remove(&p);
                        self.annotations.remove(&p);
                    }
                }
                events.push(Event::ContentRemoved(cid));
            }
            (Kind::ContentManager, 6) => events.push(Event::ActiveContent(r.int()?)),
            (Kind::ContentManager, 7) => events.push(Event::ActivePresenter(r.int()?)),
            // Content: cConnectCompleted 1 – then we connect "extendedContent" (§3.1.4.1.5.2,
            // p. 36) –, cSetNativeFileInfo 5, cSetOwnerId 6, cSetPresentInfo 7, cSetTitle 9
            // (§3.1.4.1.5.1, pp. 33–35).
            (Kind::Content, 1) => {
                let ext_kind = self
                    .contents
                    .get(&content)
                    .and_then(|c| iface::extended_kind(&c.kind));
                let already = self.contents.get(&content).and_then(|c| c.extension);
                if let (Some(k), None) = (ext_kind, already) {
                    let ext = self.connect(id, "extendedContent", k, content)?;
                    if let Some(c) = self.contents.get_mut(&content) {
                        c.extension = ext;
                    }
                }
            }
            (Kind::Content, 5) => {
                let name = r.string()?;
                let _key = r.bytes()?;
                let _iv = r.bytes()?;
                let _hash = r.bytes()?;
                let size = r.int()?;
                events.push(Event::FileInfo {
                    content,
                    name,
                    size,
                });
            }
            (Kind::Content, 6) => events.push(Event::Owner {
                content,
                user: r.int()?,
            }),
            (Kind::Content, 7) => {
                let presented = r.boolean()?;
                let presenter = r.int()?;
                events.push(Event::Presented {
                    content,
                    presented,
                    presenter,
                });
            }
            (Kind::Content, 9) => events.push(Event::Title {
                content,
                title: r.string()?,
            }),
            // PollContent: cSetLocalVote 2, cSetOpenState 3, cSetQuestion 4, cSetResults 5,
            // cSetResultsState 6 (§3.1.4.1.14.1, p. 48).
            (Kind::Poll, 2) => events.push(Event::PollLocalVote {
                content,
                choice: r.int()?,
            }),
            (Kind::Poll, 3) => events.push(Event::PollOpen {
                content,
                open: r.boolean()?,
            }),
            (Kind::Poll, 4) => {
                let question = r.string()?;
                let choices = r.strings()?;
                events.push(Event::PollQuestion {
                    content,
                    question,
                    choices,
                });
            }
            (Kind::Poll, 5) => events.push(Event::PollResults {
                content,
                results: r.ints()?,
            }),
            (Kind::Poll, 6) => events.push(Event::PollResultsVisible {
                content,
                visible: r.boolean()?,
            }),
            // PptContent: cSetLocation 4, cSetSlideCount 9 (§3.1.4.1.11.1, pp. 41–44).
            (Kind::Ppt, 4) => events.push(Event::SlideLocation {
                content,
                location: r.string()?,
            }),
            (Kind::Ppt, 9) => events.push(Event::Slides {
                content,
                count: r.int()?,
            }),
            // SharedNotesContent: cSetNotesTitles 3 (§3.1.4.1.15.1, p. 49).
            (Kind::SharedNotes, 3) => {
                let notebook = r.string()?;
                let section = r.string()?;
                let page = r.string()?;
                events.push(Event::NotesTitles {
                    content,
                    notebook,
                    section,
                    page,
                });
            }
            // AnnotationContainer: cAddAnnotationBatch 1 (ids first), cClearAnnotations 4,
            // cRemoveAnnotation 13, cRemoveAnnotations 14 (§3.1.4.1.12.1, pp. 46–47).
            (Kind::AnnotationContainer, 1 | 4 | 13 | 14) => {
                let set = self.annotations.entry(id).or_default();
                match method {
                    1 => {
                        for a in r.ints()? {
                            if set.len() < MAX_ANNOTATIONS {
                                set.insert(a);
                            }
                        }
                    }
                    4 => set.clear(),
                    13 => {
                        set.remove(&r.int()?);
                    }
                    _ => {
                        for a in r.ints()? {
                            set.remove(&a);
                        }
                    }
                }
                let count = set.len();
                events.push(Event::Annotations { content, count });
            }
            // Every other declared method is valid and carries nothing we show.
            _ => {}
        }
        Ok(())
    }
}
