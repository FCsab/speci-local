//! The compact clipboard form of a project: a prefix such as `SPECI1:` followed by
//! base64url(deflate(text)), short enough to send through a chat. Also the links that open a
//! project in the ELTE web editors.

const MAX_UNPACKED: usize = 16 << 20;

pub fn pack(prefix: &str, text: &str) -> String {
    let packed = miniz_oxide::deflate::compress_to_vec(text.as_bytes(), 9);
    format!("{prefix}{}", b64_encode(&packed))
}

pub fn unpack(prefix: &str, packed: &str, what: &str) -> Result<String, String> {
    // chat apps and mail clients like to wrap long lines
    let packed: String = packed.chars().filter(|c| !c.is_whitespace()).collect();
    let deflated = b64_decode(&packed).ok_or_else(|| {
        format!("The text after {prefix} is damaged (not valid base64). Was it copied completely?")
    })?;
    let raw = inflate(&deflated).ok_or_else(|| {
        format!("The copied {what} is damaged or incomplete. Copy it again from the source.")
    })?;
    String::from_utf8(raw).map_err(|_| format!("The copied {what} is not valid text."))
}

pub fn inflate(deflated: &[u8]) -> Option<Vec<u8>> {
    miniz_oxide::inflate::decompress_to_vec_with_limit(deflated, MAX_UNPACKED).ok()
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// base64url without padding (URL- and chat-safe).
pub fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, b)| acc | (*b as u32) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    out
}

pub fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim_end_matches('=');
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.bytes() {
        // also accept the standard alphabet (+ /)
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            _ => return None,
        };
        acc = acc << 6 | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

pub const SPEC_EDITOR: &str = "https://progalap.elte.hu/specifikacio/v1/";
pub const STUKI_EDITOR: &str = "https://progalap.elte.hu/stuki/v1/";

/// A link that opens `json` in one of the ELTE web editors, as their own Share button makes it:
/// `?data=` + standard base64 of the gzipped JSON, URL-encoded.
pub fn web_link(editor: &str, json: &str) -> String {
    let b64 = b64_encode(&gzip(json.as_bytes()));
    let pad = "%3D".repeat((4 - b64.len() % 4) % 4);
    // base64url to standard base64 (+ and /), URL-encoded
    let data = b64.replace('-', "%2B").replace('_', "%2F");
    format!("{editor}?data={data}{pad}")
}

/// The browser's DecompressionStream checks the trailer, so the CRC and length must be right.
fn gzip(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff];
    out.extend(miniz_oxide::deflate::compress_to_vec(data, 9));
    out.extend(crc32(data).to_le_bytes());
    out.extend((data.len() as u32).to_le_bytes());
    out
}

fn crc32(data: &[u8]) -> u32 {
    !data.iter().fold(!0u32, |crc, &b| {
        (0..8).fold(crc ^ u32::from(b), |c, _| {
            if c & 1 != 0 {
                c >> 1 ^ 0xedb8_8320
            } else {
                c >> 1
            }
        })
    })
}

/// The JSON in a web editor link (`…?data=…`), or `None` if the text has no `data=`.
pub fn web_link_json(text: &str) -> Option<Result<String, String>> {
    let raw = &text[text.find("data=")? + "data=".len()..];
    let raw = &raw[..raw.find(['&', '#']).unwrap_or(raw.len())];
    Some(decode_web_data(raw))
}

fn decode_web_data(raw: &str) -> Result<String, String> {
    let b64: String = percent_decode(raw)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let gz = b64_decode(&b64).ok_or("The link's data is damaged (not valid base64).")?;
    let json = gunzip(&gz).ok_or("The link's data is damaged (not valid gzip).")?;
    String::from_utf8(json).map_err(|_| "The link's data is not valid text.".to_string())
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) =
                u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16)
        {
            out.push(v);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// gzip (what the browser's CompressionStream makes) → raw deflate → bytes.
fn gunzip(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() < 18 || d[0] != 0x1f || d[1] != 0x8b || d[2] != 8 {
        return None;
    }
    let flags = d[3];
    let mut i = 10;
    if flags & 4 != 0 {
        let n = u16::from_le_bytes([*d.get(i)?, *d.get(i + 1)?]) as usize;
        i += 2 + n;
    }
    for bit in [8u8, 16] {
        if flags & bit != 0 {
            i += d.get(i..)?.iter().position(|&c| c == 0)? + 1;
        }
    }
    if flags & 2 != 0 {
        i += 2;
    }
    inflate(d.get(i..)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decoding() {
        assert_eq!(percent_decode("a%2Bb%2fc%3D"), "a+b/c=");
        assert_eq!(percent_decode("%C5%91"), "ő");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz%4"), "%zz%4");
    }

    #[test]
    fn gunzip_skips_the_optional_header_fields() {
        let text = b"structogram".repeat(20);
        let deflated = miniz_oxide::deflate::compress_to_vec(&text, 6);
        let gzip = |flags: u8, extra: &[u8]| {
            let mut g = vec![0x1f, 0x8b, 8, flags, 0, 0, 0, 0, 0, 3];
            g.extend_from_slice(extra);
            g.extend_from_slice(&deflated);
            g.extend([0u8; 8]);
            g
        };
        assert_eq!(gunzip(&gzip(0, &[])).unwrap(), text);
        assert_eq!(gunzip(&gzip(4, &[3, 0, 1, 2, 3])).unwrap(), text);
        assert_eq!(gunzip(&gzip(8, b"name.st\0")).unwrap(), text);
        assert_eq!(gunzip(&gzip(8 | 16 | 2, b"n\0c\0\x12\x34")).unwrap(), text);
        assert!(gunzip(b"plain text, long enough to look at").is_none());
        assert!(gunzip(&gzip(0, &[])[..12]).is_none());
        let mut other = gzip(0, &[]);
        other[2] = 7;
        assert!(gunzip(&other).is_none());
        assert!(gunzip(&gzip(8, b"never ends")).is_none());
    }

    #[test]
    fn web_links_hold_checked_gzip() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        let text = "Be: n∈N\n".repeat(20);
        let gz = gzip(text.as_bytes());
        let n = gz.len();
        assert_eq!(inflate(&gz[10..n - 8]).unwrap(), text.as_bytes());
        assert_eq!(gz[n - 8..n - 4], crc32(text.as_bytes()).to_le_bytes());
        assert_eq!(gz[n - 4..], (text.len() as u32).to_le_bytes());
        let link = web_link(SPEC_EDITOR, &text);
        let data = link.strip_prefix(SPEC_EDITOR).unwrap();
        let data = data.strip_prefix("?data=").unwrap();
        assert!(data.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'%'));
        let std = data
            .replace("%2B", "+")
            .replace("%2F", "/")
            .replace("%3D", "=");
        assert_eq!(std.len() % 4, 0);
        assert_eq!(b64_decode(&std).unwrap(), gz);
    }

    #[test]
    fn base64_round_trip() {
        for len in 0..40 {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            assert_eq!(b64_decode(&b64_encode(&data)).unwrap(), data, "len {len}");
        }
        assert_eq!(b64_encode(b"Man"), "TWFu");
        assert_eq!(b64_encode(b"Ma"), "TWE");
        assert!(b64_decode("TW!u").is_none());
        assert_eq!(b64_encode(&[0xfb, 0xff, 0xbf]), "-_-_");
        assert_eq!(b64_decode("-_-_"), b64_decode("+/+/"));
        assert_eq!(b64_decode("TWE=").unwrap(), b"Ma");
    }

    #[test]
    fn pack_round_trip_and_damage() {
        let text = "Be: n∈N\nKi: x∈N\n".repeat(50);
        let line = pack("X1:", &text);
        assert!(line.starts_with("X1:") && line.len() < text.len() / 4);
        let body = &line[3..];
        assert_eq!(unpack("X1:", body, "project").unwrap(), text);
        let wrapped = format!(" {}\n{} ", &body[..10], &body[10..]);
        assert_eq!(unpack("X1:", &wrapped, "project").unwrap(), text);
        let e = unpack("X1:", "@@", "project").unwrap_err();
        assert!(e.contains("after X1:") && e.contains("base64"), "{e}");
        let e = unpack("X1:", &body[..body.len() - 8], "structogram").unwrap_err();
        assert!(e.contains("structogram is damaged"), "{e}");
        let bin = b64_encode(&miniz_oxide::deflate::compress_to_vec(&[0xff, 0xfe], 9));
        assert!(
            unpack("X1:", &bin, "project")
                .unwrap_err()
                .contains("not valid text")
        );
    }
}
