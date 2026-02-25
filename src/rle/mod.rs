use std::io::{self, Read, Write};

pub struct RleCodec;

impl crate::codec::Codec for RleCodec {
    fn encode(&self, data: &[u8], writer: &mut dyn Write) -> io::Result<u64> {
        encode(data, writer)
    }

    fn decode(&self, reader: &mut dyn Read, writer: &mut dyn Write) -> io::Result<usize> {
        decode(reader, writer)
    }
}

/// Encodes data using PackBits-style Run-Length Encoding.
///
/// Format uses a control byte to switch between two modes:
/// - `0x00–0x7F`: Literal — the next `n + 1` bytes (1–128) are copied verbatim
/// - `0x80–0xFF`: Run — the next byte is repeated `n - 126` times (2–129)
///
/// Returns the number of bits in the encoded output.
pub fn encode(data: &[u8], writer: &mut dyn Write) -> io::Result<u64> {
    if data.is_empty() {
        return Ok(0);
    }

    let mut total_bytes: u64 = 0;
    let mut i = 0;

    while i < data.len() {
        let value = data[i];
        let mut run_len = 1;
        while i + run_len < data.len() && data[i + run_len] == value && run_len < 129 {
            run_len += 1;
        }

        if run_len >= 2 {
            // Emit a run chunk: control byte + value byte
            let control = (run_len as u8) + 126;
            writer.write_all(&[control, value])?;
            total_bytes += 2;
            i += run_len;
        } else {
            // Collect literal bytes until we hit a run of 2+ or reach 128
            let start = i;
            i += 1;

            while i < data.len() && (i - start) < 128 {
                if i + 1 < data.len() && data[i] == data[i + 1] {
                    break;
                }
                i += 1;
            }

            let lit_len = i - start;
            let control = (lit_len as u8) - 1;
            writer.write_all(&[control])?;
            writer.write_all(&data[start..i])?;
            total_bytes += 1 + lit_len as u64;
        }
    }

    Ok(total_bytes * 8)
}

/// Decodes PackBits-style RLE-encoded data.
///
/// Reads a control byte, then either copies literal bytes or expands a run.
/// Returns the number of bytes written to output.
pub fn decode(reader: &mut dyn Read, writer: &mut dyn Write) -> io::Result<usize> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;

    let mut bytes_written: usize = 0;
    let mut i = 0;

    while i < buf.len() {
        let control = buf[i];
        i += 1;

        if control <= 0x7F {
            // Literal: next (control + 1) bytes copied verbatim
            let count = (control as usize) + 1;
            if i + count > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "RLE decode error: truncated literal data",
                ));
            }
            writer.write_all(&buf[i..i + count])?;
            bytes_written += count;
            i += count;
        } else {
            // Run: next byte repeated (control - 126) times
            if i >= buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "RLE decode error: truncated run data",
                ));
            }
            let count = (control as usize) - 126;
            let value = buf[i];
            i += 1;
            let run = vec![value; count];
            writer.write_all(&run)?;
            bytes_written += count;
        }
    }

    Ok(bytes_written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_and_decodes_simple_data() {
        let data = b"aaabbcdddd";
        let mut enc_buf: Vec<u8> = Vec::new();
        let mut dec_buf: Vec<u8> = Vec::new();

        encode(data, &mut enc_buf).expect("Failed to encode");
        decode(&mut enc_buf.as_slice(), &mut dec_buf).expect("Failed to decode");

        assert_eq!(dec_buf, data);
    }

    #[test]
    fn encodes_empty_input() {
        let mut buf: Vec<u8> = Vec::new();
        let bits = encode(b"", &mut buf).expect("Failed to encode");

        assert_eq!(bits, 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn decodes_empty_input() {
        let mut buf: Vec<u8> = Vec::new();
        let bytes = decode(&mut [].as_slice(), &mut buf).expect("Failed to decode");

        assert_eq!(bytes, 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn encodes_and_decodes_single_byte() {
        let data = b"x";
        let mut enc_buf: Vec<u8> = Vec::new();
        let mut dec_buf: Vec<u8> = Vec::new();

        let bits = encode(data, &mut enc_buf).expect("Failed to encode");
        assert_eq!(bits, 16); // 2 bytes: [0x00, 'x']
        assert_eq!(enc_buf, vec![0x00, b'x']);

        decode(&mut enc_buf.as_slice(), &mut dec_buf).expect("Failed to decode");
        assert_eq!(dec_buf, data);
    }

    #[test]
    fn encodes_to_correct_run_bytes() {
        let mut buf: Vec<u8> = Vec::new();

        // "aaaaaa" = run of 6, control = 6 + 126 = 132 = 0x84
        let bits = encode(b"aaaaaa", &mut buf).expect("Failed to encode");
        assert_eq!(buf, vec![0x84, b'a']);
        assert_eq!(bits, 16);
    }

    #[test]
    fn encodes_to_correct_literal_bytes() {
        let mut buf: Vec<u8> = Vec::new();

        // "abcdef" = literal of 6, control = 6 - 1 = 5 = 0x05
        let bits = encode(b"abcdef", &mut buf).expect("Failed to encode");
        assert_eq!(buf, vec![0x05, b'a', b'b', b'c', b'd', b'e', b'f']);
        assert_eq!(bits, 56); // 7 bytes * 8
    }

    #[test]
    fn encodes_to_correct_mixed_bytes() {
        let mut buf: Vec<u8> = Vec::new();

        // "aaabbc" = run(3,'a') + run(2,'b') + literal(1,'c')
        // run(3): control = 3 + 126 = 129 = 0x81
        // run(2): control = 2 + 126 = 128 = 0x80
        // literal(1): control = 1 - 1 = 0 = 0x00
        encode(b"aaabbc", &mut buf).expect("Failed to encode");
        assert_eq!(
            buf,
            vec![0x81, b'a', 0x80, b'b', 0x00, b'c']
        );
    }

    #[test]
    fn encodes_and_decodes_non_repeating_data() {
        let data = b"abcdefghijklmnop"; // 16 non-repeating bytes
        let mut enc_buf: Vec<u8> = Vec::new();
        let mut dec_buf: Vec<u8> = Vec::new();

        let bits = encode(data, &mut enc_buf).expect("Failed to encode");
        // 1 control byte + 16 literal bytes = 17 bytes
        assert_eq!(enc_buf.len(), 17);
        assert_eq!(bits, 136); // 17 * 8

        decode(&mut enc_buf.as_slice(), &mut dec_buf).expect("Failed to decode");
        assert_eq!(dec_buf, data);
    }

    #[test]
    fn encodes_and_decodes_run_exceeding_129() {
        let data = vec![0xAA; 300];
        let mut enc_buf: Vec<u8> = Vec::new();
        let mut dec_buf: Vec<u8> = Vec::new();

        encode(&data, &mut enc_buf).expect("Failed to encode");
        // 300 = 129 + 129 + 42
        // [0xFF, 0xAA, 0xFF, 0xAA, 0xA8, 0xAA] = 6 bytes
        assert_eq!(enc_buf, vec![0xFF, 0xAA, 0xFF, 0xAA, 0xA8, 0xAA]);

        decode(&mut enc_buf.as_slice(), &mut dec_buf).expect("Failed to decode");
        assert_eq!(dec_buf, data);
    }

    #[test]
    fn encodes_and_decodes_binary_data() {
        let data: Vec<u8> = (0..=255).cycle().take(1000).collect();
        let mut enc_buf: Vec<u8> = Vec::new();
        let mut dec_buf: Vec<u8> = Vec::new();

        encode(&data, &mut enc_buf).expect("Failed to encode");
        decode(&mut enc_buf.as_slice(), &mut dec_buf).expect("Failed to decode");

        assert_eq!(dec_buf, data);
    }

    #[test]
    fn encodes_and_decodes_highly_repetitive_data() {
        let data = vec![0x00; 10_000];
        let mut enc_buf: Vec<u8> = Vec::new();
        let mut dec_buf: Vec<u8> = Vec::new();

        encode(&data, &mut enc_buf).expect("Failed to encode");
        // 10000 / 129 = 77 full runs (77 * 129 = 9933), remainder = 67
        // 77 runs * 2 bytes + 1 run * 2 bytes = 156 bytes
        assert_eq!(enc_buf.len(), 156);

        decode(&mut enc_buf.as_slice(), &mut dec_buf).expect("Failed to decode");
        assert_eq!(dec_buf, data);
    }

    #[test]
    fn decode_rejects_truncated_run() {
        // Control byte 0x80 means run, but no value byte follows
        let bad_data = vec![0x80u8];
        let mut dec_buf: Vec<u8> = Vec::new();

        let result = decode(&mut bad_data.as_slice(), &mut dec_buf);
        assert!(result.is_err());
    }

    #[test]
    fn decode_rejects_truncated_literal() {
        // Control byte 0x05 means 6 literal bytes, but only 3 follow
        let bad_data = vec![0x05, b'a', b'b', b'c'];
        let mut dec_buf: Vec<u8> = Vec::new();

        let result = decode(&mut bad_data.as_slice(), &mut dec_buf);
        assert!(result.is_err());
    }

    #[test]
    fn encode_returns_correct_bit_count() {
        let data = b"aabbcc";
        let mut buf: Vec<u8> = Vec::new();

        let bits = encode(data, &mut buf).expect("Failed to encode");
        assert_eq!(bits, (buf.len() as u64) * 8);
    }
}
