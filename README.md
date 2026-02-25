# Comprust

A silly little file compression tool built purely for educational purposes.

## Usage

```
comprust <command> [-a algorithm] <input-file> <output-file>
```

### Commands

- `encode` — Compress a file
- `decode` — Decompress a file

### Options

- `-a, --algorithm <name>` — Compression algorithm to use (default: `huffman`)

### Example

```bash
# Compress a file
comprust encode myfile.txt myfile.compressed

# Decompress it back
comprust decode myfile.compressed myfile.restored.txt
```

Output includes compression ratio and time taken:

```
=> Raw: 1024 bytes
=> Compressed: 743 bytes
=> Compressed: 5940 bits
=> Ratio: 72.56%
=> Time: 1.234ms
=> Written to: myfile.compressed
```

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test
```

## How it works

Currently implements Huffman coding — a classic lossless compression algorithm that assigns shorter bit sequences to more frequent bytes. The compressed file format is:

1. Serialized Huffman tree (byte frequencies)
2. Padding count (1 byte)
3. Compressed bit data

The `Codec` trait makes it straightforward to add new algorithms alongside Huffman.

## Features

- [x] Huffman coding
- [x] Well documented command line interface
- [x] Support generic data
- [x] Verbose mode with instructive output explaining result

## Ideas for future work

- [ ] Run-length encoding (RLE) — a good simple one to start with
- [ ] LZ77 / LZ78 — dictionary-based compression, the basis for gzip and friends
- [ ] Arithmetic coding — more optimal than Huffman but trickier to implement
- [ ] Combine algorithms (e.g. LZ77 + Huffman, like DEFLATE does)
- [ ] Streaming support for large files
- [ ] Benchmarks comparing the different algorithms
