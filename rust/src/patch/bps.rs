use anyhow::{bail, Result};

use super::suffix_tree::SuffixTree;

#[derive(Debug)]
enum BPSBlock {
    Unchanged {
        dst_start: usize,
        length: usize,
    },
    SourceCopy {
        src_start: usize,
        dst_start: usize,
        length: usize,
    },
    TargetCopy {
        src_start: usize,
        dst_start: usize,
        length: usize,
    },
    Data {
        dst_start: usize,
        data: Vec<u8>
    },
}

#[derive(Debug)]
pub struct BPSPatch {
    blocks: Vec<BPSBlock>,
}

impl BPSPatch {
    pub fn new(data: Vec<u8>) -> Result<Self> {
        let mut decoder = BPSPatchDecoder::new(data);
        decoder.decode()?;
        Ok(BPSPatch {
            blocks: decoder.blocks,
        })
    }

    pub fn apply(&self, source: &[u8], output: &mut [u8]) {
        for block in &self.blocks {
            match block {
                &BPSBlock::Unchanged { .. } => {
                    // These blocks won't be loaded and wouldn't do anything anyway.
                }
                &BPSBlock::SourceCopy { src_start, dst_start, length } => {
                    let src_slice = &source[src_start..(src_start+length)];
                    let dst_slice = &mut output[dst_start..(dst_start+length)];
                    dst_slice.copy_from_slice(src_slice);
                },
                &BPSBlock::TargetCopy { src_start, dst_start, length } => {
                    for i in 0..length {
                        output[dst_start + i] = output[src_start + i];
                    }
                },
                BPSBlock::Data { dst_start, data } => {
                    output[*dst_start..(*dst_start + data.len())].copy_from_slice(data);
                }
            }           
        }
    }

    // This function isn't used now, but could be useful later, so keeping it around.
    pub fn get_modified_ranges(&self) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = vec![];
        for block in &self.blocks {
            match block {
                &BPSBlock::Unchanged { .. } => {}
                &BPSBlock::SourceCopy { src_start, dst_start, length } => {
                    if src_start != dst_start {
                        out.push((dst_start, dst_start + length));
                    }
                }
                &BPSBlock::TargetCopy { dst_start, length, .. } => {
                    out.push((dst_start, dst_start + length));
                }
                BPSBlock::Data { dst_start, data } => {
                    out.push((*dst_start, *dst_start + data.len()));
                }
            }
        }
        out
    }
}

struct BPSPatchDecoder {
    patch_bytes: Vec<u8>,
    patch_pos: usize,
    output_pos: usize,
    src_pos: usize,
    dst_pos: usize,
    blocks: Vec<BPSBlock>,
}

impl BPSPatchDecoder {
    fn new(patch_bytes: Vec<u8>) -> Self {
        BPSPatchDecoder {
            patch_bytes,
            patch_pos: 0,
            output_pos: 0,
            src_pos: 0,
            dst_pos: 0,
            blocks: vec![],
        }
    }

    fn decode(&mut self) -> Result<()> {
        assert!(self.read_n(4)? == "BPS1".as_bytes());
        let _src_size = self.decode_number()?;
        let _dst_size = self.decode_number()?;
        let metadata_size = self.decode_number()?;
        self.patch_pos += metadata_size;
        while self.patch_pos < self.patch_bytes.len() - 12 {
            let block = self.decode_block()?;
            if let BPSBlock::Unchanged { .. } = block {
                // Skip blocks that do not change the output.
            } else {
                self.blocks.push(block);
            }
        }
        assert!(self.patch_pos == self.patch_bytes.len() - 12);
        // Ignore the checksums at the end of the file.
        Ok(())
    }

    fn read(&mut self) -> Result<u8> {
        if self.patch_pos >= self.patch_bytes.len() {
            bail!("BPS read past end of data");
        }
        let x = self.patch_bytes[self.patch_pos];
        self.patch_pos += 1;
        Ok(x)
    }

    fn read_n(&mut self, n: usize) -> Result<Vec<u8>> {
        if self.patch_pos + n > self.patch_bytes.len() {
            bail!("BPS read_n past end of data");
        }
        let out = self.patch_bytes[self.patch_pos..(self.patch_pos + n)].to_vec();
        self.patch_pos += n;
        Ok(out)
    }

    fn decode_block(&mut self) -> Result<BPSBlock> {
        let cmd = self.decode_number()?;
        let action = cmd & 3;
        let length = (cmd >> 2) + 1;
        match action {
            0 => {
                let block = BPSBlock::Unchanged {
                    dst_start: self.output_pos,
                    length, 
                };
                self.output_pos += length;
                return Ok(block);
            },
            1 => {
                let block = BPSBlock::Data {
                    dst_start: self.output_pos,
                    data: self.read_n(length)?,
                };
                self.output_pos += length;
                return Ok(block);
            },
            2 => {
                let raw_offset = self.decode_number()?;
                let offset_neg = raw_offset & 1 == 1;
                let offset_abs = (raw_offset >> 1) as isize;
                let offset = if offset_neg { -offset_abs } else { offset_abs };
                self.src_pos = (self.src_pos as isize + offset) as usize;
                let block = BPSBlock::SourceCopy {
                    src_start: self.src_pos,
                    dst_start: self.output_pos,
                    length,
                };
                self.src_pos += length;
                self.output_pos += length;
                return Ok(block);
            },
            3 => {
                let raw_offset = self.decode_number()?;
                let offset_neg = raw_offset & 1 == 1;
                let offset_abs = (raw_offset >> 1) as isize;
                let offset = if offset_neg { -offset_abs } else { offset_abs };
                self.dst_pos = (self.dst_pos as isize + offset) as usize;
                let block = BPSBlock::TargetCopy {
                    src_start: self.dst_pos,
                    dst_start: self.output_pos,
                    length,
                };
                self.dst_pos += length;
                self.output_pos += length;
                return Ok(block);
            },
            _ => panic!("Unexpected BPS action: {}", action)
        }
    }

    fn decode_number(&mut self) -> Result<usize> {
        let mut out: usize = 0;
        let mut shift: usize = 1;
        for _ in 0..10 {
            let x = self.read()?;
            out += ((x & 0x7f) as usize) * shift;
            if x & 0x80 != 0 {
                return Ok(out);
            }
            shift <<= 7;
            out += shift;
        }
        bail!("invalid BPS");
    }
}


struct BPSPatchEncoder<'a> {
    source_prefix_tree: &'a SuffixTree,
    target: &'a [u8],
    modified_ranges: &'a [(usize, usize)],
    patch_bytes: Vec<u8>,
    src_pos: usize,
    input_pos: usize,
}

fn compute_crc32(data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

// This is a simplified BPS encoder that is not overly concerned with minimizing patch size,
// mainly just with capturing dependencies on the source data (even when relocated) to avoid 
// source data being copied into the patch. The key difference from other encoders (and the reason
// we rolled a custom one here) is that this encoder enforces that only bytes that differ from the source
// will get touched by the patch; this is important in order to ensure that we can correctly 
// and efficiently layer multiple patches on top of each other (assuming they affect disjoint sets of 
// bytes). For a similar reason, this encoder also doesn't create blocks that copy from the target
// (i.e. previously output data).
impl<'a> BPSPatchEncoder<'a> {
    pub fn new(source_prefix_tree: &'a SuffixTree, target: &'a [u8], modified_ranges: &'a [(usize, usize)]) -> Self {
        Self {
            source_prefix_tree,
            target,
            modified_ranges,
            patch_bytes: vec![],
            src_pos: 0,
            input_pos: 0,
        }       
    }

    fn encode(&mut self) {
        self.write_n("BPS1".as_bytes());
        self.encode_number(self.source_prefix_tree.data.len());
        self.encode_number(self.target.len());
        self.encode_number(0); // metadata size
        for r in self.modified_ranges {
            self.encode_range(r.0, r.1);
        }
        self.write_n(&compute_crc32(&self.source_prefix_tree.data).to_le_bytes());
        self.write_n(&compute_crc32(&self.target).to_le_bytes());
        self.write_n(&compute_crc32(&self.patch_bytes).to_le_bytes());
    }

    fn encode_range(&mut self, mut start_addr: usize, end_addr: usize) {
        // Unchanged block:
        if self.input_pos < start_addr {
            let length = start_addr - self.input_pos;
            self.encode_unchanged(length);
            self.input_pos += length;
        }

        // Data and source copy blocks:
        while start_addr < end_addr {
            let (source_start, match_length) = self.source_prefix_tree.lookup(&self.target[start_addr..end_addr]);
            if match_length >= 3 {
                if start_addr > self.input_pos {
                    self.encode_data(&self.source_prefix_tree.data[self.input_pos..start_addr]);
                    self.encode_source_copy(source_start, match_length);
                    self.input_pos = start_addr + match_length;
                    start_addr = start_addr + match_length;
                }
            } else {
                start_addr += 1;
            }    
        }
        if end_addr > self.input_pos {
            self.encode_data(&self.source_prefix_tree.data[self.input_pos..end_addr]);
            self.input_pos = end_addr;
        }
    }

    fn encode_block_header(&mut self, action: usize, length: usize) {
        let x = action | ((length - 1) << 2);
        self.encode_number(x);
    }

    fn encode_unchanged(&mut self, length: usize) {
        self.encode_block_header(0, length);
    }

    fn encode_data(&mut self, data: &[u8]) {
        self.encode_block_header(1, data.len());
        self.write_n(data);
    }

    fn encode_source_copy(&mut self, idx: usize, length: usize) {
        self.encode_block_header(2, length);
        let relative_idx = (idx as isize) - (self.src_pos as isize);
        if relative_idx < 0 {
            self.encode_number(1 | (((-relative_idx) as usize) << 1));
        } else {
            self.encode_number(1 | ((relative_idx as usize) << 1));
        }
        self.src_pos = idx + length;
    }

    fn write(&mut self, b: u8) {
        self.patch_bytes.push(b);
    }

    fn write_n(&mut self, data: &[u8]) {
        self.patch_bytes.extend(data);
    }

    fn encode_number(&mut self, mut x: usize) {
        for _ in 0..10 {
            let b = (x & 0x7f) as u8;
            x >>= 7;
            if x == 0 {
              self.write(0x80 | b);
              break;
            }
            self.write(b);
            x -= 1;
        }
    }
}