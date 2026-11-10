//! The compact clipboard form of a project: a prefix such as `SPECI1:` followed by
//! base64url(deflate(text)), short enough to send through a chat.

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

#[cfg(test)]
mod tests {
    use super::*;

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
