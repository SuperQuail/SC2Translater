//! 归档的读取（Archive）与编辑重建（Document）。
//!
//! 写侧最容易犯的错都在这里集中处理：
//!
//! * 头部 0x6C 的 dwRawChunkSize 一律写 **0**，然后重算 0xC0 自哈希；
//! * 头部 0x70/0x80/0x90/0xA0/0xB0 六个摘要里，前五个先算、0xC0 最后算；
//! * HET/BET 必须与经典表同源，否则引擎（走 HET）与别的读取器（走经典表）会看到不同结果。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::compress::{best_compress, decompress_chunk};
use crate::crypto::{
    HASH_FILE_KEY, HASH_TABLE_OFFSET, bits_get, decrypt, encrypt, file_key, hash_string,
};
use crate::tables::{
    Attributes, BetTable, BlockEntry, HashEntry, HetTable, build_attributes, build_bet, build_hash_table,
    build_het, md5_of,
};
use crate::{
    ATTR_CRC32, ATTR_MD5, DEFAULT_BLOCK_SHIFT, FLAG_COMPRESS, FLAG_ENCRYPTED, FLAG_EXISTS, FLAG_IMPLODE,
    FLAG_SINGLE_UNIT, HEADER_SIZE_V3, MAGIC, MpqError, Result, is_bookkeeping,
};

fn u16_at(buf: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([buf[at], buf[at + 1]])
}

fn u32_at(buf: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]])
}

fn u64_at(buf: &[u8], at: usize) -> u64 {
    u64::from_le_bytes([
        buf[at],
        buf[at + 1],
        buf[at + 2],
        buf[at + 3],
        buf[at + 4],
        buf[at + 5],
        buf[at + 6],
        buf[at + 7],
    ])
}

fn put_u32(buf: &mut [u8], at: usize, value: u32) {
    buf[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(buf: &mut [u8], at: usize, value: u64) {
    buf[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

// =========================================================================== //
// 只读视图
// =========================================================================== //

/// 只读的归档视图：解析表结构，读到某个组件时才解压它。
pub struct Archive {
    pub path: PathBuf,
    raw: Vec<u8>,
    pub header_size: u32,
    pub archive_size: u32,
    pub format_version: u16,
    pub block_shift: u16,
    pub hash_pos: u32,
    pub block_pos: u32,
    pub hash_count: u32,
    pub block_count: u32,
    pub bet_pos: u64,
    pub het_pos: u64,
    pub het_size: u64,
    pub bet_size: u64,
    /// 原件在这里写的是 16384；**写回时一律用 0**。
    pub raw_chunk_size: u32,
    pub block_size: u32,
    pub hash_table: Vec<HashEntry>,
    pub block_table: Vec<BlockEntry>,
    pub het: Option<HetTable>,
    pub bet: Option<BetTable>,
    pub attributes: Option<Attributes>,
    names: Vec<String>,
    name_by_block: HashMap<u32, String>,
}

impl Archive {
    /// 打开文件并解析（只做表解析与 listfile 读取，不解压业务组件）。
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let raw = std::fs::read(&path)?;
        Self::parse(raw, path)
    }

    pub fn parse(raw: Vec<u8>, path: PathBuf) -> Result<Self> {
        if raw.len() < 32 || &raw[0..4] != MAGIC {
            return Err(MpqError::NotAnArchive(path.display().to_string()));
        }
        let header_size = u32_at(&raw, 4);
        let archive_size = u32_at(&raw, 8);
        let format_version = u16_at(&raw, 12);
        let block_shift = u16_at(&raw, 14);
        let hash_pos = u32_at(&raw, 16);
        let block_pos = u32_at(&raw, 20);
        let hash_count = u32_at(&raw, 24);
        let block_count = u32_at(&raw, 28);

        // 按 headerSize 判断字段存在，而不是按 formatVersion
        let (bet_pos, het_pos) = if header_size >= 68 {
            (u64_at(&raw, 52), u64_at(&raw, 60))
        } else {
            (0, 0)
        };
        let (het_size, bet_size) = if header_size >= 208 {
            (u64_at(&raw, 92), u64_at(&raw, 100))
        } else {
            (0, 0)
        };
        let raw_chunk_size = if header_size >= 208 { u32_at(&raw, 108) } else { 0 };

        let block_size = 512u32 << block_shift;

        let hash_table = Self::parse_hash_table(&raw, hash_pos, hash_count, block_count)?;
        let block_table = Self::parse_block_table(&raw, block_pos, block_count)?;

        let ext = |pos: u64, size: u64| -> Option<Vec<u8>> {
            let pos = pos as usize;
            if pos == 0 || pos >= raw.len() {
                return None;
            }
            if size != 0 {
                let end = (pos + size as usize).min(raw.len());
                return Some(raw[pos..end].to_vec());
            }
            if pos + 12 > raw.len() {
                return None;
            }
            let data_size = u32_at(&raw, pos + 8) as usize;
            let end = (pos + 12 + data_size).min(raw.len());
            Some(raw[pos..end].to_vec())
        };

        let het = ext(het_pos, het_size).and_then(|b| HetTable::parse(&b).ok());
        let bet = ext(bet_pos, bet_size).and_then(|b| BetTable::parse(&b).ok());

        let mut arc = Self {
            path,
            raw,
            header_size,
            archive_size,
            format_version,
            block_shift,
            hash_pos,
            block_pos,
            hash_count,
            block_count,
            bet_pos,
            het_pos,
            het_size,
            bet_size,
            raw_chunk_size,
            block_size,
            hash_table,
            block_table,
            het,
            bet,
            attributes: None,
            names: Vec::new(),
            name_by_block: HashMap::new(),
        };

        if let Some(attr_raw) = arc.try_read("(attributes)") {
            arc.attributes = Attributes::parse(&attr_raw, arc.block_count as usize).ok();
        }
        arc.load_names();
        Ok(arc)
    }

    fn parse_hash_table(raw: &[u8], pos: u32, count: u32, block_count: u32) -> Result<Vec<HashEntry>> {
        let start = pos as usize;
        let end = start + count as usize * 16;
        if end > raw.len() {
            return Err(MpqError::TooShort { need: end, got: raw.len() });
        }
        let blob = &raw[start..end];
        let key = hash_string("(hash table)", HASH_FILE_KEY);
        let plain = decrypt(blob, key);
        let mut entries: Vec<HashEntry> = plain.chunks_exact(16).map(HashEntry::unpack).collect();
        // 自洽校验：必须**至少有一个空闲槽**，且非空闲项索引不越界。
        // 不通过就退回「未解密」的解释（有些归档这两张表本来就没加密）。
        let ok = entries.iter().any(|e| e.is_free())
            && entries.iter().all(|e| e.is_free() || e.block_index < block_count);
        if !ok {
            entries = blob.chunks_exact(16).map(HashEntry::unpack).collect();
        }
        Ok(entries)
    }

    fn parse_block_table(raw: &[u8], pos: u32, count: u32) -> Result<Vec<BlockEntry>> {
        let start = pos as usize;
        let end = start + count as usize * 16;
        if end > raw.len() {
            return Err(MpqError::TooShort { need: end, got: raw.len() });
        }
        let blob = &raw[start..end];
        let key = hash_string("(block table)", HASH_FILE_KEY);
        let plain = decrypt(blob, key);
        let mut entries: Vec<BlockEntry> = plain.chunks_exact(16).map(BlockEntry::unpack).collect();
        let ok = entries
            .iter()
            .all(|e| !e.exists() || (e.file_pos as usize + e.cmp_size as usize) <= raw.len());
        if !ok {
            entries = blob.chunks_exact(16).map(BlockEntry::unpack).collect();
        }
        Ok(entries)
    }

    fn load_names(&mut self) {
        let listing = self.try_read("(listfile)").unwrap_or_default();
        let text = String::from_utf8_lossy(&listing);
        self.names = text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();
        let mut map = HashMap::new();
        let names = self.names.clone();
        for name in &names {
            if let Some(i) = self.find_block(name) {
                map.entry(i).or_insert_with(|| name.clone());
            }
        }
        self.name_by_block = map;
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// 业务组件名（不含簿记项）。
    pub fn component_names(&self) -> Vec<String> {
        self.names.iter().filter(|n| !is_bookkeeping(n)).cloned().collect()
    }

    pub fn block_name(&self, index: u32) -> Option<String> {
        self.name_by_block.get(&index).cloned()
    }

    /// 经典哈希表查找。
    pub fn find_block(&self, name: &str) -> Option<u32> {
        if self.hash_count == 0 {
            return None;
        }
        let mask = self.hash_count - 1;
        let mut idx = (hash_string(name, HASH_TABLE_OFFSET) & mask) as usize;
        let want_a = hash_string(name, crate::HASH_NAME_A);
        let want_b = hash_string(name, crate::HASH_NAME_B);
        for _ in 0..self.hash_count {
            let Some(e) = self.hash_table.get(idx) else { return None };
            if e.is_free() {
                return None;
            }
            if e.hash_a == want_a && e.hash_b == want_b && e.block_index < self.block_count {
                return Some(e.block_index);
            }
            idx = (idx + 1) & mask as usize;
        }
        None
    }

    /// 权威查找：有 HET 就走 HET，并用 BET 的 64 位名字哈希复核。
    ///
    /// **HET 条目只存名字哈希的高 8 位，命中只是候选**；复核不匹配时必须**继续探测**
    /// 而不是直接返回失败（实测 HTXL 有 3 个、UmojanCT 有 1 个这样的名字）。
    pub fn lookup_index(&self, name: &str) -> Option<u32> {
        let Some(het) = self.het.as_ref() else {
            return self.find_block(name);
        };
        if het.entry_count == 0 || het.total_count == 0 {
            return None;
        }
        let want = crate::jenkins_name_hash(name, het.name_hash_bit_size);
        let name_hash1 = ((want >> (het.name_hash_bit_size - 8)) & 0xFF) as u8;
        let start = (want % het.total_count as u64) as usize;
        let mut index = start;
        loop {
            if index >= het.name_hashes.len() {
                return None;
            }
            if het.name_hashes[index] == crate::HET_ENTRY_FREE {
                return None;
            }
            if het.name_hashes[index] == name_hash1 {
                let bitpos = het.index_size_total as usize * index;
                let cand = bits_get(&het.index_bits, bitpos, het.index_size) as u32;
                if cand < self.block_count {
                    match self.bet.as_ref() {
                        None => return Some(cand),
                        Some(bet) => {
                            if cand < bet.entry_count {
                                let mask = (1u64 << bet.bit_count_name_hash2) - 1;
                                if (want & mask) == bet.name_hash2(cand) {
                                    return Some(cand);
                                }
                            }
                        }
                    }
                }
            }
            index = (index + 1) % het.total_count as usize;
            if index == start {
                return None;
            }
        }
    }

    pub fn has(&self, name: &str) -> bool {
        self.find_block(name).is_some()
    }

    pub fn try_read(&self, name: &str) -> Option<Vec<u8>> {
        let index = self.find_block(name)?;
        self.read_block(index, name).ok()
    }

    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        let index = self
            .find_block(name)
            .ok_or_else(|| MpqError::MissingComponent(name.to_string()))?;
        self.read_block(index, name)
    }

    /// 读一个块。
    pub fn read_block(&self, index: u32, name: &str) -> Result<Vec<u8>> {
        let Some(b) = self.block_table.get(index as usize).copied() else {
            return Err(MpqError::MissingComponent(format!("块索引越界 {index}")));
        };
        if !b.exists() {
            return Err(MpqError::MissingComponent(name.to_string()));
        }
        if b.file_size == 0 {
            return Ok(Vec::new());
        }
        let start = b.file_pos as usize;
        let end = start + b.cmp_size as usize;
        if end > self.raw.len() {
            return Err(MpqError::TooShort { need: end, got: self.raw.len() });
        }
        let stored = &self.raw[start..end];
        let data = if b.flags & FLAG_ENCRYPTED != 0 {
            decrypt(stored, file_key(name, b.file_pos, b.flags))
        } else {
            stored.to_vec()
        };

        if b.flags & FLAG_IMPLODE != 0 {
            return Err(MpqError::UnsupportedCompression { mask: 0x08 });
        }
        if b.flags & FLAG_SINGLE_UNIT != 0 {
            return decompress_chunk(&data, b.file_size as usize);
        }

        // 分扇区。偏移表是 n+1 个 dword；SECTOR_CRC 的块在磁盘上多一个尾部 dword
        // （offsets[0] 已经把这段表长算进去了），所以 offsets[0..=n] 两种情况下都对。
        let file_size = b.file_size as usize;
        let bs = self.block_size as usize;
        if bs == 0 {
            return Err(MpqError::BadTable("block_size 为 0".into()));
        }
        let sectors = file_size.div_ceil(bs);
        if (sectors + 1) * 4 > data.len() {
            return Err(MpqError::TooShort { need: (sectors + 1) * 4, got: data.len() });
        }
        let mut offsets = Vec::with_capacity(sectors + 1);
        for i in 0..=sectors {
            offsets.push(u32_at(&data, i * 4) as usize);
        }
        let mut out = Vec::with_capacity(file_size);
        for i in 0..sectors {
            let expected = bs.min(file_size - out.len());
            let (s, e) = (offsets[i], offsets[i + 1]);
            if e > data.len() || s > e {
                return Err(MpqError::BadTable(format!("扇区 {i} 偏移越界: {s}..{e}")));
            }
            out.extend_from_slice(&decompress_chunk(&data[s..e], expected)?);
        }
        Ok(out)
    }

    /// 全量解包到目录。
    pub fn extract_all(&self, dest: &Path) -> Result<Vec<String>> {
        let mut written = Vec::new();
        for (i, b) in self.block_table.iter().enumerate() {
            if !b.exists() {
                continue;
            }
            let Some(name) = self.block_name(i as u32) else { continue };
            let data = self.read_block(i as u32, &name)?;
            let target = dest.join(name.replace('\\', "/"));
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, data)?;
            written.push(name);
        }
        Ok(written)
    }

    /// 容器自检。deep=true 时会逐块校验 attributes。
    pub fn verify(&self, deep: bool) -> VerifyReport {
        let mut r = VerifyReport {
            path: self.path.display().to_string(),
            format_version: self.format_version,
            header_size: self.header_size,
            block_size: self.block_size,
            blocks: self.block_count,
            hash_entries: self.hash_count,
            names: self.names.len(),
            has_het: self.het.is_some(),
            has_bet: self.bet.is_some(),
            has_attributes: self.attributes.is_some(),
            raw_chunk_size: self.raw_chunk_size,
            ..Default::default()
        };

        if self.header_size as usize >= HEADER_SIZE_V3 && self.raw.len() >= 0xD0 {
            let h: Vec<u8> = self.raw[..0xD0].to_vec();
            r.header_md5_valid = md5_of(&h[..0xC0]) == h[0xC0..0xD0];
            let bt_start = self.block_pos as usize;
            let bt_end = bt_start + self.block_count as usize * 16;
            let ht_start = self.hash_pos as usize;
            let ht_end = ht_start + self.hash_count as usize * 16;
            if bt_end <= self.raw.len() {
                r.block_table_md5_valid = md5_of(&self.raw[bt_start..bt_end]) == h[0x70..0x80];
            }
            if ht_end <= self.raw.len() {
                r.hash_table_md5_valid = md5_of(&self.raw[ht_start..ht_end]) == h[0x80..0x90];
            }
            if let Some(het) = self.het.as_ref() {
                let len = het.to_bytes().len();
                let start = self.het_pos as usize;
                if start + len <= self.raw.len() {
                    r.het_md5_valid = md5_of(&self.raw[start..start + len]) == h[0xB0..0xC0];
                }
            }
            if let Some(bet) = self.bet.as_ref() {
                let len = bet.to_bytes().len();
                let start = self.bet_pos as usize;
                if start + len <= self.raw.len() {
                    r.bet_md5_valid = md5_of(&self.raw[start..start + len]) == h[0xA0..0xB0];
                }
            }
        }

        // 从表模型重新序列化，必须与磁盘上那段字节完全相同 —— 最强的正确性证据。
        if let Some(het) = self.het.as_ref() {
            let start = self.het_pos as usize;
            let blob = het.to_bytes();
            if start + blob.len() <= self.raw.len() {
                r.het_roundtrip_exact = blob == self.raw[start..start + blob.len()];
            }
        }
        if let Some(bet) = self.bet.as_ref() {
            let start = self.bet_pos as usize;
            let blob = bet.to_bytes();
            if start + blob.len() <= self.raw.len() {
                r.bet_roundtrip_exact = blob == self.raw[start..start + blob.len()];
            }
        }

        if deep {
            for name in self.names.iter().cloned() {
                if self.find_block(&name) != self.lookup_index(&name) {
                    r.het_vs_classic_mismatch.push(name);
                }
            }
            for name in ["(listfile)", "(attributes)"] {
                if self.find_block(name) != self.lookup_index(name) {
                    r.het_vs_classic_mismatch.push(name.to_string());
                }
            }
            if let Some(bet) = self.bet.as_ref() {
                for i in 0..self.block_count.min(bet.entry_count) {
                    let (pos, size, cmp, flag) = bet.entry(i);
                    let b = self.block_table[i as usize];
                    if (pos, size, cmp) != (b.file_pos as u64, b.file_size as u64, b.cmp_size as u64) {
                        r.bet_vs_blocktable_mismatch.push(i);
                    }
                    if (flag as usize) < bet.flags.len() && bet.flags[flag as usize] != b.flags {
                        r.bet_flag_mismatch.push(i);
                    }
                }
            }
            if let Some(attrs) = self.attributes.as_ref() {
                for (i, b) in self.block_table.iter().enumerate() {
                    if !b.exists() {
                        continue;
                    }
                    let name = self.block_name(i as u32).unwrap_or_default();
                    let Ok(data) = self.read_block(i as u32, &name) else { continue };
                    if let Some(md5s) = attrs.md5s.as_ref() {
                        if md5s.get(i).is_some_and(|m| *m == md5_of(&data)) {
                            r.md5_ok += 1;
                        } else {
                            r.md5_bad += 1;
                            r.md5_exceptions.push(name.clone());
                        }
                    }
                    if let Some(crcs) = attrs.crcs.as_ref() {
                        if crcs.get(i).is_some_and(|c| *c == crc32fast::hash(&data)) {
                            r.crc_ok += 1;
                        } else {
                            r.crc_bad += 1;
                        }
                    }
                }
            }
        }
        r
    }
}

/// 容器自检报告。
#[derive(Clone, Debug, Default)]
pub struct VerifyReport {
    pub path: String,
    pub format_version: u16,
    pub header_size: u32,
    pub block_size: u32,
    pub blocks: u32,
    pub hash_entries: u32,
    pub names: usize,
    pub has_het: bool,
    pub has_bet: bool,
    pub has_attributes: bool,
    /// **必须为 0**，否则编辑器会对任何内容改动报 e_fileCorrupt。
    pub raw_chunk_size: u32,
    pub header_md5_valid: bool,
    pub block_table_md5_valid: bool,
    pub hash_table_md5_valid: bool,
    pub het_md5_valid: bool,
    pub bet_md5_valid: bool,
    pub het_roundtrip_exact: bool,
    pub bet_roundtrip_exact: bool,
    pub het_vs_classic_mismatch: Vec<String>,
    pub bet_vs_blocktable_mismatch: Vec<u32>,
    pub bet_flag_mismatch: Vec<u32>,
    pub md5_ok: usize,
    pub md5_bad: usize,
    pub crc_ok: usize,
    pub crc_bad: usize,
    pub md5_exceptions: Vec<String>,
}

impl VerifyReport {
    /// 除簿记项约定外的「全部通过」判定。
    ///
    /// (listfile) 与 (attributes) 的 MD5 约定是全零、CRC 约定是 0，
    /// 所以 md5_bad 恒为 2、crc_bad 恒为 1 才是**正确**的。
    pub fn healthy(&self) -> bool {
        self.header_md5_valid
            && self.het_roundtrip_exact
            && self.bet_roundtrip_exact
            && self.het_vs_classic_mismatch.is_empty()
            && self.bet_vs_blocktable_mismatch.is_empty()
            && self.bet_flag_mismatch.is_empty()
            && self.raw_chunk_size == 0
    }
}

// =========================================================================== //
// 写侧
// =========================================================================== //

/// 从源归档继承的布局快照 —— 「重建件能被编辑器接受」的关键之一。
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub hash_count: Option<u32>,
    pub block_shift: u16,
    /// BET 的 unknown08。实测 5 个真实样本恒为 7888，**沿用原件值**。
    pub unknown08: u32,
    pub attr_flags: u32,
    /// 组件集合没变时沿用原件 (listfile)，可做到逐字节复现。
    pub listfile: Option<Vec<u8>>,
    pub per_block_flags: HashMap<String, u32>,
}

/// 构建参数。
#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub hash_count: Option<u32>,
    pub block_shift: u16,
    pub unknown08: u32,
    pub attr_flags: u32,
    pub listfile: Option<Vec<u8>>,
    pub hints: HashMap<String, u32>,
    /// 一律写 0。写 16384 会让编辑器拒收任何改动过的归档。
    pub raw_chunk_size: u32,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            hash_count: None,
            block_shift: DEFAULT_BLOCK_SHIFT,
            unknown08: 0x10,
            attr_flags: ATTR_CRC32 | ATTR_MD5,
            listfile: None,
            hints: HashMap::new(),
            raw_chunk_size: 0,
        }
    }
}

/// 构建结果。
#[derive(Clone, Debug, Default)]
pub struct BuildInfo {
    pub path: String,
    pub bytes: usize,
    pub entries: usize,
    pub blocks: usize,
    pub hash_entries: u32,
    pub het_bytes: usize,
    pub bet_bytes: usize,
    pub block_size: u32,
}

/// 把一组组件打包成块（含分块策略）。
fn pack_entries(
    entries: &[(String, Vec<u8>)],
    block_size: u32,
    hints: &HashMap<String, u32>,
) -> Result<(Vec<u8>, Vec<BlockEntry>)> {
    let mut body: Vec<u8> = Vec::new();
    let mut blocks: Vec<BlockEntry> = Vec::with_capacity(entries.len());
    for (name, data) in entries {
        let current = HEADER_SIZE_V3 as u32 + body.len() as u32;
        let hint = hints.get(name).copied();
        let sectorised =
            hint.is_some_and(|h| h & FLAG_SINGLE_UNIT == 0) && data.len() > block_size as usize;
        if data.is_empty() {
            blocks.push(BlockEntry {
                file_pos: current,
                cmp_size: 0,
                file_size: 0,
                flags: FLAG_EXISTS | FLAG_SINGLE_UNIT,
            });
            continue;
        }
        if !sectorised {
            let (payload, compressed) = best_compress(data)?;
            let flags =
                FLAG_EXISTS | FLAG_SINGLE_UNIT | if compressed { FLAG_COMPRESS } else { 0 };
            blocks.push(BlockEntry {
                file_pos: current,
                cmp_size: payload.len() as u32,
                file_size: data.len() as u32,
                flags,
            });
            body.extend_from_slice(&payload);
        } else {
            let bs = block_size as usize;
            let sectors: Vec<&[u8]> = data.chunks(bs).collect();
            let mut pieces: Vec<Vec<u8>> = Vec::with_capacity(sectors.len());
            let mut offsets: Vec<u32> = Vec::with_capacity(sectors.len() + 1);
            let mut running = 4 * (sectors.len() as u32 + 1);
            offsets.push(running);
            let mut any_compressed = false;
            for chunk in &sectors {
                let (stored, compressed) = best_compress(chunk)?;
                any_compressed |= compressed;
                running += stored.len() as u32;
                offsets.push(running);
                pieces.push(stored);
            }
            let mut payload = Vec::with_capacity(running as usize);
            for o in &offsets {
                payload.extend_from_slice(&o.to_le_bytes());
            }
            for p in &pieces {
                payload.extend_from_slice(p);
            }
            blocks.push(BlockEntry {
                file_pos: current,
                cmp_size: payload.len() as u32,
                file_size: data.len() as u32,
                flags: FLAG_EXISTS | if any_compressed { FLAG_COMPRESS } else { 0 },
            });
            body.extend_from_slice(&payload);
        }
    }
    Ok((body, blocks))
}

/// 从一组组件从零构建一个 SC2 文档。
pub fn build_document(
    entries_in: &[(String, Vec<u8>)],
    dest: &Path,
    opts: &BuildOptions,
) -> Result<BuildInfo> {
    let block_size = 512u32 << opts.block_shift;
    let user_entries: Vec<(String, Vec<u8>)> = entries_in
        .iter()
        .filter(|(n, _)| !is_bookkeeping(n))
        .cloned()
        .collect();

    // 定 (listfile)：调用方给了就用给的；否则按当前组件集合生成。
    let listfile = opts.listfile.clone().unwrap_or_else(|| {
        let mut s = String::new();
        for (n, _) in &user_entries {
            s.push_str(n);
            s.push_str("\r\n");
        }
        s.into_bytes()
    });
    let mut working: Vec<(String, Vec<u8>)> = user_entries.clone();
    working.push(("(listfile)".to_string(), listfile));

    // pass 1：用占位 (attributes) 打一次包，只为拿到块几何
    let placeholder_len = 8 + 4 * (working.len() + 1) + 16 * (working.len() + 1);
    let mut probe = working.clone();
    probe.push(("(attributes)".to_string(), vec![0u8; placeholder_len]));
    let (_, blocks0) = pack_entries(&probe, block_size, &opts.hints)?;
    let names0: Vec<String> = probe.iter().map(|(n, _)| n.clone()).collect();
    let datas0: Vec<Vec<u8>> = probe.iter().map(|(_, d)| d.clone()).collect();
    let attrs0 = build_attributes(&blocks0, &names0, &datas0, 100, opts.attr_flags);

    // pass 2：真打包，再按真几何重算 (attributes)，迭代到不动点
    let mut entries_final: Vec<(String, Vec<u8>)> = working.clone();
    entries_final.push(("(attributes)".to_string(), attrs0.to_bytes()));
    let mut body: Vec<u8> = Vec::new();
    let mut blocks: Vec<BlockEntry> = Vec::new();
    for _ in 0..4 {
        let (b, bl) = pack_entries(&entries_final, block_size, &opts.hints)?;
        body = b;
        blocks = bl;
        let names: Vec<String> = entries_final.iter().map(|(n, _)| n.clone()).collect();
        let datas: Vec<Vec<u8>> = entries_final.iter().map(|(_, d)| d.clone()).collect();
        let attrs = build_attributes(&blocks, &names, &datas, 100, opts.attr_flags);
        let blob = attrs.to_bytes();
        if entries_final.last().map(|(_, d)| d.as_slice()) == Some(blob.as_slice()) {
            break;
        }
        if let Some(last) = entries_final.last_mut() {
            last.1 = blob;
        }
    }

    let names: Vec<String> = entries_final.iter().map(|(n, _)| n.clone()).collect();
    let hash_count = opts.hash_count.unwrap_or_else(|| {
        let mut c = 4u32;
        // 必须**大于**条目数：至少留一个空槽，读取器才认得出这是有效的哈希表
        while c < names.len() as u32 + 1 {
            c <<= 1;
        }
        c
    });

    let het = build_het(&names, 64)?;
    let bet = build_bet(&blocks, &names, 64, opts.unknown08);
    let het_blob = het.to_bytes();
    let bet_blob = bet.to_bytes();

    let het_pos = HEADER_SIZE_V3 + body.len();
    let bet_pos = het_pos + het_blob.len();
    let hash_pos = bet_pos + bet_blob.len();
    let hash_entries = build_hash_table(&names, hash_count)?;
    let mut hash_plain = Vec::with_capacity(hash_entries.len() * 16);
    for e in &hash_entries {
        hash_plain.extend_from_slice(&e.pack());
    }
    let hash_blob = encrypt(&hash_plain, hash_string("(hash table)", HASH_FILE_KEY));
    let block_pos = hash_pos + hash_blob.len();
    let mut block_plain = Vec::with_capacity(blocks.len() * 16);
    for b in &blocks {
        block_plain.extend_from_slice(&b.pack());
    }
    let block_blob = encrypt(&block_plain, hash_string("(block table)", HASH_FILE_KEY));

    let archive_size = block_pos + block_blob.len();
    let mut header = vec![0u8; HEADER_SIZE_V3];
    header[0..4].copy_from_slice(MAGIC);
    put_u32(&mut header, 4, HEADER_SIZE_V3 as u32);
    put_u32(&mut header, 8, archive_size as u32);
    header[12..14].copy_from_slice(&3u16.to_le_bytes());
    header[14..16].copy_from_slice(&opts.block_shift.to_le_bytes());
    put_u32(&mut header, 16, hash_pos as u32);
    put_u32(&mut header, 20, block_pos as u32);
    put_u32(&mut header, 24, hash_count);
    put_u32(&mut header, 28, blocks.len() as u32);
    put_u64(&mut header, 44, archive_size as u64);
    put_u64(&mut header, 52, bet_pos as u64);
    put_u64(&mut header, 60, het_pos as u64);
    put_u64(&mut header, 68, hash_blob.len() as u64);
    put_u64(&mut header, 76, block_blob.len() as u64);
    put_u64(&mut header, 84, 0);
    put_u64(&mut header, 92, het_blob.len() as u64);
    put_u64(&mut header, 100, bet_blob.len() as u64);
    // dwRawChunkSize 保持 0。暴雪写入器在这里写 16384（尽管头自称 formatVersion=3），
    // 那会打开 Blizzard System_Mopaq 的 raw-chunk MD5 校验路径，导致任何内容改动
    // 都被判成 e_fileCorrupt —— 实测 A/B：16384 崩溃、0 正常加载。
    put_u32(&mut header, 108, opts.raw_chunk_size);

    header[0x70..0x80].copy_from_slice(&md5_of(&block_blob));
    header[0x80..0x90].copy_from_slice(&md5_of(&hash_blob));
    header[0x90..0xA0].copy_from_slice(&[0u8; 16]);
    header[0xA0..0xB0].copy_from_slice(&md5_of(&bet_blob));
    header[0xB0..0xC0].copy_from_slice(&md5_of(&het_blob));
    // 0xC0 是自哈希：必须最后算，覆盖前 0xC0 字节（含上面 5 个摘要与 rawChunkSize）
    let header_digest = md5_of(&header[..0xC0]);
    header[0xC0..0xD0].copy_from_slice(&header_digest);

    let mut payload = Vec::with_capacity(archive_size);
    payload.extend_from_slice(&header);
    payload.extend_from_slice(&body);
    payload.extend_from_slice(&het_blob);
    payload.extend_from_slice(&bet_blob);
    payload.extend_from_slice(&hash_blob);
    payload.extend_from_slice(&block_blob);

    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(dest, &payload)?;

    Ok(BuildInfo {
        path: dest.display().to_string(),
        bytes: payload.len(),
        entries: user_entries.len(),
        blocks: blocks.len(),
        hash_entries: hash_count,
        het_bytes: het_blob.len(),
        bet_bytes: bet_blob.len(),
        block_size,
    })
}

/// 可写视图：把归档读成「组件名 -> 字节」的保序列表，改完整体写回。
pub struct Document {
    pub path: PathBuf,
    entries: Vec<(String, Vec<u8>)>,
    pub layout: Layout,
    original_names: Vec<String>,
}

impl Document {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let archive = Archive::open(&path)?;
        let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
        for name in archive.component_names() {
            if let Ok(data) = archive.read(&name) {
                entries.push((name, data));
            }
        }
        for name in ["(listfile)", "(attributes)", "(signature)", "(user data)"] {
            if let Some(data) = archive.try_read(name) {
                entries.push((name.to_string(), data));
            }
        }
        let original_names: Vec<String> = entries
            .iter()
            .filter(|(n, _)| !is_bookkeeping(n))
            .map(|(n, _)| n.clone())
            .collect();
        let mut per_block_flags = HashMap::new();
        for (i, b) in archive.block_table.iter().enumerate() {
            if b.exists() {
                if let Some(n) = archive.block_name(i as u32) {
                    per_block_flags.insert(n, b.flags);
                }
            }
        }
        let listfile = archive.try_read("(listfile)");
        let layout = Layout {
            hash_count: Some(archive.hash_count),
            block_shift: archive.block_shift,
            unknown08: archive.bet.as_ref().map(|b| b.unknown08).unwrap_or(0x10),
            attr_flags: archive.attributes.as_ref().map(|a| a.flags).unwrap_or(ATTR_CRC32 | ATTR_MD5),
            listfile,
            per_block_flags,
        };
        Ok(Self { path, entries, layout, original_names })
    }

    pub fn component_names(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|(n, _)| !is_bookkeeping(n))
            .map(|(n, _)| n.clone())
            .collect()
    }

    pub fn entries(&self) -> &[(String, Vec<u8>)] {
        &self.entries
    }

    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, d)| d.as_slice())
    }

    /// 写入/替换一个组件（新组件插在簿记项之前）。
    pub fn set(&mut self, name: &str, data: Vec<u8>) {
        if let Some(slot) = self.entries.iter_mut().find(|(n, _)| n == name) {
            slot.1 = data;
            return;
        }
        let insert_at = self
            .entries
            .iter()
            .position(|(n, _)| is_bookkeeping(n))
            .unwrap_or(self.entries.len());
        self.entries.insert(insert_at, (name.to_string(), data));
    }

    pub fn remove(&mut self, name: &str) -> bool {
        if let Some(pos) = self.entries.iter().position(|(n, _)| n == name) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    /// 写回。自动沿用原归档的 hash_count / block_shift / unknown08 / attr_flags / 分块方式；
    /// 组件集合没变时沿用原件 (listfile)。
    pub fn save(&self, dest: &Path) -> Result<BuildInfo> {
        let names_now: Vec<String> = self.component_names();
        let keep_listfile =
            if names_now == self.original_names { self.layout.listfile.clone() } else { None };
        let mut hints = self.layout.per_block_flags.clone();
        for name in &names_now {
            hints.entry(name.clone()).or_insert(FLAG_EXISTS | FLAG_SINGLE_UNIT);
        }

        // 经典 hash 表必须**大于**条目数（至少留一个空槽），否则读取器会
        // 判定「这张表没解密成功」而退回原始字节解析。组件变多时要放大。
        let need = (names_now.len() + 2) as u32;
        let mut hash_count = self.layout.hash_count.unwrap_or(4).max(4);
        while hash_count < need + 1 || (hash_count as u64) * 7 < (need as u64) * 8 {
            hash_count <<= 1;
        }

        let opts = BuildOptions {
            hash_count: Some(hash_count),
            block_shift: self.layout.block_shift,
            unknown08: self.layout.unknown08,
            attr_flags: self.layout.attr_flags,
            listfile: keep_listfile,
            hints,
            raw_chunk_size: 0,
        };
        build_document(&self.entries, dest, &opts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{decrypt as crypto_decrypt, encrypt as crypto_encrypt};

    #[test]
    fn block_cipher_roundtrips_including_tail() {
        // 尾部 1~3 字节原样搬运：长度 199 不能被 4 整除，正是 HET 表的真实情况
        let data: Vec<u8> = (0..199u32).map(|i| (i % 251) as u8).collect();
        let key = hash_string("(hash table)", HASH_FILE_KEY);
        let enc = crypto_encrypt(&data, key);
        assert_ne!(enc, data);
        assert_eq!(enc.len(), data.len());
        assert_eq!(crypto_decrypt(&enc, key), data);
    }

    #[test]
    fn hash_string_matches_python_reference() {
        // 由 Python 原型（SC2Diff tools/sc2mpq.py）算出，作为两边一致性的锚点
        assert_eq!(hash_string("(hash table)", HASH_FILE_KEY), 0xC3AF_3770);
        assert_eq!(hash_string("(block table)", HASH_FILE_KEY), 0xEC83_B3A3);
        assert_eq!(hash_string("(listfile)", HASH_FILE_KEY), 0x2D2F_0A94);
        assert_eq!(hash_string("(attributes)", HASH_FILE_KEY), 0x50E3_14AF);
    }

    #[test]
    fn jenkins_name_hash_matches_python_reference() {
        assert_eq!(
            crate::jenkins_name_hash("New/Component.txt", 64),
            0x8F4C_0833_8E99_001A
        );
    }

    #[test]
    fn build_and_read_back_small_document() {
        let dir = std::env::temp_dir().join("sc2mpq_unit_small");
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("small.SC2Map");
        let entries = vec![
            ("Base.SC2Data\\GameData\\UnitData.xml".to_string(), b"<Catalog/>".to_vec()),
            (
                "enUS.SC2Data\\LocalizedData\\GameStrings.txt".to_string(),
                b"DocInfo/Name=X\r\n".to_vec(),
            ),
        ];
        let info = build_document(&entries, &dest, &BuildOptions::default()).unwrap();
        assert!(info.bytes > 0);
        let arc = Archive::open(&dest).unwrap();
        assert_eq!(arc.raw_chunk_size, 0);
        let report = arc.verify(true);
        assert!(report.header_md5_valid);
        assert!(report.het_roundtrip_exact);
        assert!(report.bet_roundtrip_exact);
        assert!(report.healthy());
        for (name, data) in &entries {
            assert_eq!(&arc.read(name).unwrap(), data, "组件内容应逐字节相同: {name}");
        }
        let _ = std::fs::remove_file(&dest);
    }

    #[test]
    fn adding_a_component_grows_the_hash_table() {
        let dir = std::env::temp_dir().join("sc2mpq_unit_grow");
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.SC2Map");
        // 6 个组件 -> 连簿记项共 8 个条目，正好是 2 的幂：不处理会被填满
        let entries: Vec<(String, Vec<u8>)> =
            (0..6).map(|i| (format!("C{i}.bin"), vec![i as u8; 32 + i])).collect();
        build_document(&entries, &a, &BuildOptions::default()).unwrap();
        let mut doc = Document::open(&a).unwrap();
        doc.set("New/Component.txt", b"hello".to_vec());
        let b = dir.join("b.SC2Map");
        doc.save(&b).unwrap();
        let arc = Archive::open(&b).unwrap();
        // 关键断言：加完组件后 (listfile) 仍然读得出来
        assert!(arc.has("(listfile)"));
        assert!(arc.has("New/Component.txt"));
        assert!(arc.verify(true).header_md5_valid);
        let _ = std::fs::remove_file(&a);
        let _ = std::fs::remove_file(&b);
    }
}
