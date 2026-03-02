use std::io::{self, BufReader, Read, Write};

pub struct RleCodec;

impl crate::codec::Codec for RleCodec {
    fn encode(&self, reader: &mut dyn Read, writer: &mut dyn Write) -> io::Result<u64> {
        encode(reader, writer)
    }

    fn decode(&self, reader: &mut dyn Read, writer: &mut dyn Write) -> io::Result<usize> {
        decode(reader, writer)
    }
}

/// Encodes data using PackBits-style Run-Length Encoding, reading input as a stream.
///
/// Format uses a control byte to switch between two modes:
/// - `0x00–0x7F`: Literal — the next `n + 1` bytes (1–128) are copied verbatim
/// - `0x80–0xFF`: Run — the next byte is repeated `n - 126` times (2–129)
///
/// Returns the number of bits in the encoded output.
pub fn encode(reader: &mut dyn Read, writer: &mut dyn Write) -> io::Result<u64> {
    let mut reader = BufReader::new(reader);
    let mut total_bytes: u64 = 0;
    let mut literals: Vec<u8> = Vec::with_capacity(128);
    let mut byte_buf = [0u8; 1];

    // Read first byte
    if reader.read(&mut byte_buf)? == 0 {
        return Ok(0);
    }
    let mut prev = byte_buf[0];
    let mut run_count: usize = 1;

    loop {
        let n = reader.read(&mut byte_buf)?;
        if n == 0 {
            // EOF — flush remaining state
            if run_count >= 2 {
                total_bytes += flush_run(writer, prev, run_count)?;
            } else {
                literals.push(prev);
                total_bytes += flush_literals(writer, &mut literals)?;
            }
            break;
        }

        let byte = byte_buf[0];

        if byte == prev {
            if run_count == 1 {
                // prev was uncommitted — flush any accumulated literals before starting the run
                total_bytes += flush_literals(writer, &mut literals)?;
            }
            run_count += 1;
            if run_count == 129 {
                total_bytes += flush_run(writer, prev, run_count)?;
                run_count = 0;
            }
        } else {
            if run_count >= 2 {
                total_bytes += flush_run(writer, prev, run_count)?;
            } else if run_count == 1 {
                literals.push(prev);
                if literals.len() == 128 {
                    total_bytes += flush_literals(writer, &mut literals)?;
                }
            }
            // run_count == 0 means we just flushed a full 129-run
            prev = byte;
            run_count = 1;
        }
    }

    Ok(total_bytes * 8)
}

fn flush_run(writer: &mut dyn Write, value: u8, count: usize) -> io::Result<u64> {
    let control = (count as u8) + 126;
    writer.write_all(&[control, value])?;
    Ok(2)
}

fn flush_literals(writer: &mut dyn Write, literals: &mut Vec<u8>) -> io::Result<u64> {
    if literals.is_empty() {
        return Ok(0);
    }
    let control = (literals.len() as u8) - 1;
    writer.write_all(&[control])?;
    writer.write_all(literals)?;
    let len = 1 + literals.len() as u64;
    literals.clear();
    Ok(len)
}

/// Decodes PackBits-style RLE-encoded data, reading input as a stream.
///
/// Reads a control byte, then either copies literal bytes or expands a run.
/// Returns the number of bytes written to output.
pub fn decode(reader: &mut dyn Read, writer: &mut dyn Write) -> io::Result<usize> {
    let mut reader = BufReader::new(reader);
    let mut bytes_written: usize = 0;
    let mut control_buf = [0u8; 1];

    loop {
        match reader.read_exact(&mut control_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }
        let control = control_buf[0];

        if control <= 0x7F {
            // Literal: next (control + 1) bytes copied verbatim
            let count = (control as usize) + 1;
            let mut buf = vec![0u8; count];
            reader.read_exact(&mut buf).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "RLE decode error: truncated literal data",
                )
            })?;
            writer.write_all(&buf)?;
            bytes_written += count;
        } else {
            // Run: next byte repeated (control - 126) times
            let mut value_buf = [0u8; 1];
            reader.read_exact(&mut value_buf).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "RLE decode error: truncated run data",
                )
            })?;
            let count = (control as usize) - 126;
            let run = vec![value_buf[0]; count];
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

        encode(&mut &data[..], &mut enc_buf).expect("Failed to encode");
        decode(&mut enc_buf.as_slice(), &mut dec_buf).expect("Failed to decode");

        assert_eq!(dec_buf, data);
    }

    #[test]
    fn encodes_empty_input() {
        let mut buf: Vec<u8> = Vec::new();
        let bits = encode(&mut &b""[..], &mut buf).expect("Failed to encode");

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

        let bits = encode(&mut &data[..], &mut enc_buf).expect("Failed to encode");
        assert_eq!(bits, 16); // 2 bytes: [0x00, 'x']
        assert_eq!(enc_buf, vec![0x00, b'x']);

        decode(&mut enc_buf.as_slice(), &mut dec_buf).expect("Failed to decode");
        assert_eq!(dec_buf, data);
    }

    #[test]
    fn encodes_to_correct_run_bytes() {
        let mut buf: Vec<u8> = Vec::new();

        // "aaaaaa" = run of 6, control = 6 + 126 = 132 = 0x84
        let bits = encode(&mut &b"aaaaaa"[..], &mut buf).expect("Failed to encode");
        assert_eq!(buf, vec![0x84, b'a']);
        assert_eq!(bits, 16);
    }

    #[test]
    fn encodes_to_correct_literal_bytes() {
        let mut buf: Vec<u8> = Vec::new();

        // "abcdef" = literal of 6, control = 6 - 1 = 5 = 0x05
        let bits = encode(&mut &b"abcdef"[..], &mut buf).expect("Failed to encode");
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
        encode(&mut &b"aaabbc"[..], &mut buf).expect("Failed to encode");
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

        let bits = encode(&mut &data[..], &mut enc_buf).expect("Failed to encode");
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

        encode(&mut &data[..], &mut enc_buf).expect("Failed to encode");
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

        encode(&mut &data[..], &mut enc_buf).expect("Failed to encode");
        decode(&mut enc_buf.as_slice(), &mut dec_buf).expect("Failed to decode");

        assert_eq!(dec_buf, data);
    }

    #[test]
    fn encodes_and_decodes_highly_repetitive_data() {
        let data = vec![0x00; 10_000];
        let mut enc_buf: Vec<u8> = Vec::new();
        let mut dec_buf: Vec<u8> = Vec::new();

        encode(&mut &data[..], &mut enc_buf).expect("Failed to encode");
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

        let bits = encode(&mut &data[..], &mut buf).expect("Failed to encode");
        assert_eq!(bits, (buf.len() as u64) * 8);
    }
}
