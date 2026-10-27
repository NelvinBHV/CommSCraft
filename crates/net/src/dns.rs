//! A minimal DNS client for SRV records (RFC 1035, RFC 2782), used for the MS-CONMGMT fallback when
//! Autodiscover gives no SIP access point.
//!
//! Security by design: DNS without DNSSEC can be spoofed, so the results are only hints – the
//! discovery layer keeps only targets inside the user's SIP domain (CON-16) and TLS verifies the
//! host name. Here: a random 16-bit ID, the answer must come from the queried server, carry the same
//! ID and the same question; parsing is bounded (packet size, record count, compression pointer
//! jumps); truncated UDP answers are retried over TCP.

use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream, UdpSocket};
use std::time::Duration;

use crate::NetError;

const TYPE_SRV: u16 = 33;
const CLASS_IN: u16 = 1;
const MAX_ANSWERS: usize = 64;

/// One SRV answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Srv {
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: String,
}

fn bad(what: &str) -> NetError {
    NetError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("DNS: {what}"),
    ))
}

/// Name servers from `/etc/resolv.conf` content (at most three, like the C library).
pub fn nameservers(resolv_conf: &str) -> Vec<IpAddr> {
    resolv_conf
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            (it.next() == Some("nameserver"))
                .then(|| it.next())
                .flatten()
        })
        .filter_map(|a| a.parse().ok())
        .take(3)
        .collect()
}

/// Encodes the query for `name` (absolute, without trailing dot needed).
pub fn build_query(id: u16, name: &str) -> Result<Vec<u8>, NetError> {
    let mut q = Vec::with_capacity(32 + name.len());
    q.extend_from_slice(&id.to_be_bytes());
    q.extend_from_slice(&[0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]); // RD; QDCOUNT 1
    let name = name.strip_suffix('.').unwrap_or(name);
    if name.len() > 253 {
        return Err(bad("name too long"));
    }
    for label in name.split('.') {
        let len = u8::try_from(label.len())
            .ok()
            .filter(|l| (1..=63).contains(l))
            .ok_or_else(|| bad("label"))?;
        if !label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(bad("label characters"));
        }
        q.push(len);
        q.extend_from_slice(label.as_bytes());
    }
    q.push(0);
    q.extend_from_slice(&TYPE_SRV.to_be_bytes());
    q.extend_from_slice(&CLASS_IN.to_be_bytes());
    Ok(q)
}

fn u16_at(p: &[u8], at: usize) -> Result<u16, NetError> {
    match p.get(at..at + 2) {
        Some(&[a, b]) => Ok(u16::from_be_bytes([a, b])),
        _ => Err(bad("truncated")),
    }
}

/// Reads a (possibly compressed) name; returns it and the offset after it in the original
/// position.
fn read_name(p: &[u8], mut at: usize) -> Result<(String, usize), NetError> {
    let mut labels: Vec<String> = Vec::new();
    let mut end = None;
    let mut jumps = 0;
    let mut total = 0;
    loop {
        let len = *p.get(at).ok_or_else(|| bad("truncated name"))?;
        match len {
            0 => {
                return Ok((labels.join("."), end.unwrap_or(at + 1)));
            }
            l if l & 0xc0 == 0xc0 => {
                let target = usize::from(u16_at(p, at)? & 0x3fff);
                jumps += 1;
                if jumps > 16 || target >= at {
                    // Only backward pointers: no loops.
                    return Err(bad("compression pointer"));
                }
                end.get_or_insert(at + 2);
                at = target;
            }
            l if l & 0xc0 == 0 => {
                let l = usize::from(l);
                let label = p
                    .get(at + 1..at + 1 + l)
                    .ok_or_else(|| bad("truncated label"))?;
                total += l + 1;
                if total > 255 {
                    return Err(bad("name too long"));
                }
                labels.push(String::from_utf8_lossy(label).to_ascii_lowercase());
                at += 1 + l;
            }
            _ => return Err(bad("label type")),
        }
    }
}

/// Parses an answer to `query`; `Ok(None)` if the answer is truncated (retry over TCP).
pub fn parse_answer(query: &[u8], p: &[u8]) -> Result<Option<Vec<Srv>>, NetError> {
    if p.len() < 12 || p.len() > 65535 {
        return Err(bad("size"));
    }
    if p.get(..2) != query.get(..2) {
        return Err(bad("ID mismatch"));
    }
    let flags = u16_at(p, 2)?;
    if flags & 0x8000 == 0 {
        return Err(bad("not a response"));
    }
    if flags & 0x0200 != 0 {
        return Ok(None);
    }
    let rcode = flags & 0x000f;
    if rcode == 3 {
        return Ok(Some(Vec::new())); // NXDOMAIN
    }
    if rcode != 0 {
        return Err(bad("server error"));
    }
    if u16_at(p, 4)? != 1 {
        return Err(bad("question count"));
    }
    let question = query.get(12..).ok_or_else(|| bad("query"))?;
    if p.get(12..12 + question.len())
        .map(|q| q.to_ascii_lowercase())
        != Some(question.to_ascii_lowercase())
    {
        return Err(bad("question mismatch"));
    }
    let ancount = usize::from(u16_at(p, 6)?);
    if ancount > MAX_ANSWERS {
        return Err(bad("too many answers"));
    }
    let mut at = 12 + question.len();
    let mut out = Vec::new();
    for _ in 0..ancount {
        let (_, after) = read_name(p, at)?;
        let rtype = u16_at(p, after)?;
        let class = u16_at(p, after + 2)?;
        let rdlen = usize::from(u16_at(p, after + 8)?);
        let rdata = after + 10;
        if p.len() < rdata + rdlen {
            return Err(bad("truncated record"));
        }
        if rtype == TYPE_SRV && class == CLASS_IN {
            if rdlen < 7 {
                return Err(bad("SRV record"));
            }
            let (target, _) = read_name(p, rdata + 6)?;
            out.push(Srv {
                priority: u16_at(p, rdata)?,
                weight: u16_at(p, rdata + 2)?,
                port: u16_at(p, rdata + 4)?,
                target,
            });
        }
        at = rdata + rdlen;
    }
    Ok(Some(out))
}

/// Looks up SRV records of `name` at the given name servers. `id` comes from the CSPRNG.
pub fn lookup_srv(
    servers: &[IpAddr],
    name: &str,
    id: u16,
    timeout: Duration,
) -> Result<Vec<Srv>, NetError> {
    let query = build_query(id, name)?;
    let mut last = bad("no name server");
    for ip in servers {
        let addr = SocketAddr::new(*ip, 53);
        match udp(addr, &query, timeout).and_then(|a| parse_answer(&query, &a)) {
            Ok(Some(r)) => return Ok(r),
            Ok(None) => match tcp(addr, &query, timeout).and_then(|a| parse_answer(&query, &a)) {
                Ok(Some(r)) => return Ok(r),
                Ok(None) => last = bad("truncated over TCP"),
                Err(e) => last = e,
            },
            Err(e) => last = e,
        }
    }
    Err(last)
}

fn udp(addr: SocketAddr, query: &[u8], timeout: Duration) -> Result<Vec<u8>, NetError> {
    let bind: SocketAddr = if addr.is_ipv4() {
        ([0, 0, 0, 0], 0).into()
    } else {
        ([0u16; 8], 0).into()
    };
    let sock = UdpSocket::bind(bind).map_err(NetError::Io)?;
    sock.connect(addr).map_err(NetError::Io)?; // only datagrams from this server are accepted
    sock.set_read_timeout(Some(timeout)).map_err(NetError::Io)?;
    sock.send(query).map_err(NetError::Io)?;
    let mut buf = vec![0u8; 4096];
    let n = sock.recv(&mut buf).map_err(NetError::Io)?;
    buf.truncate(n);
    Ok(buf)
}

fn tcp(addr: SocketAddr, query: &[u8], timeout: Duration) -> Result<Vec<u8>, NetError> {
    let mut s = TcpStream::connect_timeout(&addr, timeout).map_err(NetError::Connect)?;
    s.set_read_timeout(Some(timeout)).map_err(NetError::Io)?;
    let len = u16::try_from(query.len()).map_err(|_| bad("query too long"))?;
    s.write_all(&len.to_be_bytes()).map_err(NetError::Io)?;
    s.write_all(query).map_err(NetError::Io)?;
    let mut l = [0u8; 2];
    s.read_exact(&mut l).map_err(NetError::Io)?;
    let mut buf = vec![0u8; usize::from(u16::from_be_bytes(l))];
    s.read_exact(&mut buf).map_err(NetError::Io)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An answer as a server would build it: the question, then SRV records whose owner name is a
    /// compression pointer to the question (0xc00c).
    fn answer(query: &[u8], records: &[(u16, u16, u16, &str)], flags: u16) -> Vec<u8> {
        let mut p = query.to_vec();
        p[2..4].copy_from_slice(&flags.to_be_bytes());
        p[6..8].copy_from_slice(&(records.len() as u16).to_be_bytes());
        for (prio, weight, port, target) in records {
            let mut rdata = Vec::new();
            rdata.extend(prio.to_be_bytes());
            rdata.extend(weight.to_be_bytes());
            rdata.extend(port.to_be_bytes());
            for l in target.split('.') {
                rdata.push(l.len() as u8);
                rdata.extend(l.as_bytes());
            }
            rdata.push(0);
            p.extend([0xc0, 0x0c]);
            p.extend(TYPE_SRV.to_be_bytes());
            p.extend(CLASS_IN.to_be_bytes());
            p.extend(300u32.to_be_bytes());
            p.extend((rdata.len() as u16).to_be_bytes());
            p.extend(rdata);
        }
        p
    }

    #[test]
    fn query_and_answer() {
        let q = build_query(0x1234, "_sipinternaltls._tcp.contoso.com").unwrap();
        assert_eq!(&q[..2], &[0x12, 0x34]);
        assert_eq!(q[12], 15, "first label length");
        let a = answer(
            &q,
            &[
                (0, 0, 5061, "pool1.contoso.com"),
                (10, 5, 5061, "pool2.contoso.com"),
            ],
            0x8180,
        );
        let r = parse_answer(&q, &a).unwrap().unwrap();
        assert_eq!(
            r,
            vec![
                Srv {
                    priority: 0,
                    weight: 0,
                    port: 5061,
                    target: "pool1.contoso.com".into()
                },
                Srv {
                    priority: 10,
                    weight: 5,
                    port: 5061,
                    target: "pool2.contoso.com".into()
                },
            ]
        );
        assert_eq!(
            nameservers("# x\nnameserver 127.0.0.53\noptions edns0\nnameserver ::1\n"),
            vec![
                "127.0.0.53".parse::<IpAddr>().unwrap(),
                "::1".parse::<IpAddr>().unwrap(),
            ]
        );
    }

    #[test]
    fn spoofed_or_broken_answers_are_refused() {
        let q = build_query(0x1234, "_sip._tls.contoso.com").unwrap();
        let good = answer(&q, &[(0, 0, 443, "sip.contoso.com")], 0x8180);
        let mut wrong_id = good.clone();
        wrong_id[1] ^= 1;
        assert!(parse_answer(&q, &wrong_id).is_err());
        let mut other_question = good.clone();
        other_question[13] = b'x';
        assert!(parse_answer(&q, &other_question).is_err());
        let mut not_response = good.clone();
        not_response[2] = 0x01;
        assert!(parse_answer(&q, &not_response).is_err());
        let mut servfail = good.clone();
        servfail[3] = 0x82;
        assert!(parse_answer(&q, &servfail).is_err());
        assert_eq!(
            parse_answer(&q, &answer(&q, &[], 0x8183)).unwrap(),
            Some(vec![]),
            "NXDOMAIN"
        );
        assert_eq!(
            parse_answer(&q, &answer(&q, &[], 0x8380)).unwrap(),
            None,
            "truncated"
        );
        let mut loop_ptr = good.clone();
        let n = q.len();
        loop_ptr[n] = 0xc0;
        loop_ptr[n + 1] = n as u8; // points to itself
        assert!(parse_answer(&q, &loop_ptr).is_err());
        assert!(parse_answer(&q, &good[..good.len() - 3]).is_err());
        assert!(build_query(1, "a..b").is_err());
        assert!(build_query(1, "a b.c").is_err());
    }

    #[test]
    fn mutated_answers_never_panic() {
        let q = build_query(7, "_sip._tls.contoso.com").unwrap();
        let good = answer(
            &q,
            &[
                (0, 0, 443, "sip.contoso.com"),
                (1, 1, 5061, "pool.contoso.com"),
            ],
            0x8180,
        );
        let mut s: u64 = 0xdead_beef_cafe_f00d;
        for _ in 0..20_000 {
            let mut p = good.clone();
            for _ in 0..4 {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                let i = (s as usize) % p.len();
                p[i] = (s >> 32) as u8;
            }
            let _ = parse_answer(&q, &p);
        }
    }
}
