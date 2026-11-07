//! Bounded HTTP/1.1 framing. One request per connection, loopback only.
use std::io::Read;
use std::net::TcpStream;
use std::time::{Duration, Instant};
const HEADER_LIMIT: usize = 16 * 1024;
const BODY_LIMIT: usize = 8 * 1024 * 1024;
pub(super) struct Request {
    pub method: String,
    pub path: String,
    pub body: String,
}
pub(super) fn read(stream: &mut TcpStream) -> Result<Request, (u16, String)> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut data = Vec::new();
    let mut chunk = [0; 8192];
    let header_end = loop {
        if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        if data.len() >= HEADER_LIMIT {
            return Err((413, "HTTP headers too large".into()));
        }
        read_more(stream, &mut data, &mut chunk, deadline)?;
    };
    if header_end > HEADER_LIMIT {
        return Err((413, "HTTP headers too large".into()));
    }
    let header = std::str::from_utf8(&data[..header_end])
        .map_err(|_| (400, "Invalid HTTP headers".into()))?;
    let mut lines = header.split("\r\n");
    let mut start = lines.next().unwrap_or_default().split_whitespace();
    let method = start
        .next()
        .ok_or((400, "Missing method".into()))?
        .to_owned();
    let path = start.next().ok_or((400, "Missing path".into()))?.to_owned();
    if start.next() != Some("HTTP/1.1") || start.next().is_some() || !path.starts_with('/') {
        return Err((400, "Expected an HTTP/1.1 origin-form request".into()));
    }
    let mut length = None;
    for line in lines.filter(|l| !l.is_empty()) {
        let (name, value) = line
            .split_once(':')
            .ok_or((400, "Malformed header".into()))?;
        if name.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return Err((400, "Duplicate Content-Length".into()));
            }
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| (400, "Invalid Content-Length".into()))?,
            );
        }
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err((
                400,
                "Transfer-Encoding is unsupported; use Content-Length".into(),
            ));
        }
    }
    if method == "POST" && length.is_none() {
        return Err((411, "Content-Length is required".into()));
    }
    let length = length.unwrap_or(0);
    if length > BODY_LIMIT {
        return Err((413, "Request body exceeds 8 MiB".into()));
    }
    while data.len() < header_end + length {
        read_more(stream, &mut data, &mut chunk, deadline)?;
    }
    let body = std::str::from_utf8(&data[header_end..header_end + length])
        .map_err(|_| (400, "Body must be UTF-8".into()))?
        .to_owned();
    Ok(Request { method, path, body })
}
fn read_more(
    stream: &mut TcpStream,
    data: &mut Vec<u8>,
    chunk: &mut [u8],
    deadline: Instant,
) -> Result<(), (u16, String)> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or((408, "Request deadline exceeded".into()))?;
    stream
        .set_read_timeout(Some(remaining))
        .map_err(|e| (500, e.to_string()))?;
    let n = stream.read(chunk).map_err(|e| {
        (
            if matches!(
                e.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) {
                408
            } else {
                400
            },
            e.to_string(),
        )
    })?;
    if n == 0 {
        return Err((400, "Incomplete HTTP request".into()));
    }
    data.extend_from_slice(&chunk[..n]);
    Ok(())
}
pub(super) fn query(path: &str, key: &str) -> Option<String> {
    path.split_once('?')?.1.split('&').find_map(|entry| {
        let (name, value) = entry.split_once('=')?;
        if name != key {
            return None;
        }
        let mut out = Vec::new();
        let bytes = value.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'+' => out.push(b' '),
                b'%' if i + 2 < bytes.len() => {
                    out.push(
                        u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?, 16)
                            .ok()?,
                    );
                    i += 2;
                }
                b'%' => return None,
                b => out.push(b),
            }
            i += 1;
        }
        String::from_utf8(out).ok()
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpListener, thread};
    fn roundtrip(data: Vec<Vec<u8>>) -> Result<Request, (u16, String)> {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            read(&mut s)
        });
        let mut s = TcpStream::connect(addr).unwrap();
        for bytes in data {
            s.write_all(&bytes).unwrap();
            thread::sleep(Duration::from_millis(2));
        }
        s.shutdown(std::net::Shutdown::Write).unwrap();
        worker.join().unwrap()
    }
    #[test]
    fn fragmented_large_utf8_body() {
        let body = "材".repeat(20000);
        let mut chunks = vec![
            format!(
                "POST /scepter/command HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
                body.len()
            )
            .into_bytes(),
        ];
        chunks.extend(body.as_bytes().chunks(701).map(|c| c.to_vec()));
        assert_eq!(roundtrip(chunks).unwrap().body, body);
    }
    #[test]
    fn rejects_ambiguous_oversized_and_truncated_requests() {
        for input in [
            "POST / HTTP/1.1\r\nContent-Length: 1\r\nContent-Length: 1\r\n\r\nx",
            "POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n",
            "POST / HTTP/1.1\r\nContent-Length: 9000000\r\n\r\n",
            "POST / HTTP/1.1\r\nContent-Length: 4\r\n\r\nx",
        ] {
            assert!(roundtrip(vec![input.as_bytes().to_vec()]).is_err());
        }
    }
    #[test]
    fn decodes_query() {
        assert_eq!(
            query("/?name=Harbor+%E6%9D%90", "name").as_deref(),
            Some("Harbor 材")
        );
    }
}
