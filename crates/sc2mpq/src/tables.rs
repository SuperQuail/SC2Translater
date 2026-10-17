//! 五张表的解析与**对称**序列化。
//!
//! 对称性是本层的验收核心：把真实归档里的 HET/BET 解析出来再序列化回去，
//! 必须与磁盘上的字节**逐字节相同**（SC2Diff 已对 5 个真实样本验证过）。
//! 只要这条成立，模型就没有信息损失，改内容再写回也就是安全的。

use md5::{Digest, Md5};

use crate::compress::best_compress;
use crate::crypto::{
    HASH_FILE_KEY, HASH_NAME_A, HASH_NAME_B, HASH_TABLE_OFFSET, bad_table, bits_get, bits_set, encrypt,
    hash_string, jenkins_name_hash, necessary_bit_count,
};
use crate::{
    ATTR_CRC32, ATTR_FILETIME, ATTR_MD5, BET_SIG, HASH_ENTRY_FREE, HET_ENTRY_FREE, HET_SIG, MD5_ZERO,
    MpqError, Result,
};

// --------------------------------------------------------------------------- //
// 块表 / 哈希表条目
// --------------------------------------------------------------------------- //

/// 经典块表的一条：描述一个块在磁盘上的位置与长度。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockEntry {
    pub file_pos: u32,
    pub cmp_size: u32,
    pub file_size: u32,
    pub flags: u32,
}

impl BlockEntry {
    pub fn exists(&self) -> bool {
        self.flags & crate::FLAG_EXISTS != 0
    }

    pub fn pack(&self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&self.file_pos.to_le_bytes());
        out[4..8].copy_from_slice(&self.cmp_size.to_le_bytes());
        out[8..12].copy_from_slice(&self.file_size.to_le_bytes());
        out[12..16].copy_from_slice(&self.flags.to_le_bytes());
        out
    }

    pub fn unpack(raw: &[u8]) -> Self {
        Self {
            file_pos: u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]),
            cmp_size: u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]),
            file_size: u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]]),
            flags: u32::from_le_bytes([raw[12], raw[13], raw[14], raw[15]]),
        }
    }
}

/// 经典哈希表的一条。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HashEntry {
    pub hash_a: u32,
    pub hash_b: u32,
    pub locale: u16,
    pub platform: u16,
    pub block_index: u32,
}

impl HashEntry {
    pub fn free() -> Self {
        Self {
            hash_a: 0,
            hash_b: 0,
            locale: 0xFFFF,
            platform: 0xFFFF,
            block_index: HASH_ENTRY_FREE,
        }
    }

    pub fn is_free(&self) -> bool {
        self.block_index == HASH_ENTRY_FREE
    }

    pub fn pack(&self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&self.hash_a.to_le_bytes());
        out[4..8].copy_from_slice(&self.hash_b.to_le_bytes());
        out[8..10].copy_from_slice(&self.locale.to_le_bytes());
        out[10..12].copy_from_slice(&self.platform.to_le_bytes());
        out[12..16].copy_from_slice(&self.block_index.to_le_bytes());
        out
    }

    pub fn unpack(raw: &[u8]) -> Self {
        Self {
            hash_a: u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]),
            hash_b: u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]),
            locale: u16::from_le_bytes([raw[8], raw[9]]),
            platform: u16::from_le_bytes([raw[10], raw[11]]),
            block_index: u32::from_le_bytes([raw[12], raw[13], raw[14], raw[15]]),
        }
    }
}

// --------------------------------------------------------------------------- //
// HET
// --------------------------------------------------------------------------- //

/// HET 表：名字哈希高 8 位 -> 块索引。它是**候选**索引，必须用 BET 复核。
#[derive(Clone, Debug, Default)]
pub struct HetTable {
    pub table_size: u32,
    pub entry_count: u32,
    pub total_count: u32,
    pub name_hash_bit_size: u32,
    pub index_size_total: u32,
    pub index_size_extra: u32,
    pub index_size: u32,
    pub index_table_size: u32,
    pub name_hashes: Vec<u8>,
    pub index_bits: Vec<u8>,
}

impl HetTable {
    fn body(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + self.name_hashes.len() + self.index_bits.len());
        for v in [
            self.table_size,
            self.entry_count,
            self.total_count,
            self.name_hash_bit_size,
            self.index_size_total,
            self.index_size_extra,
            self.index_size,
            self.index_table_size,
        ] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&self.name_hashes);
        out.extend_from_slice(&self.index_bits);
        out
    }

    /// 序列化：12 字节明文（签名 + version + bodySize）+ 加密体。
    pub fn to_bytes(&self) -> Vec<u8> {
        let body = self.body();
        let key = hash_string("(hash table)", HASH_FILE_KEY);
        let enc = encrypt(&body, key);
        let mut out = Vec::with_capacity(12 + enc.len());
        out.extend_from_slice(HET_SIG);
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&enc);
        out
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 12 || &data[0..4] != HET_SIG {
            return Err(bad_table("不是 HET 表（签名不匹配或长度不足 12）"));
        }
        let data_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        let end = (12 + data_size).min(data.len());
        let key = hash_string("(hash table)", HASH_FILE_KEY);
        let body = crate::crypto::decrypt(&data[12..end], key);
        if body.len() < 32 {
            return Err(bad_table("HET 表体不足 32 字节"));
        }
        let f = |i: usize| u32::from_le_bytes([body[i], body[i + 1], body[i + 2], body[i + 3]]);
        let mut t = Self {
            table_size: f(0),
            entry_count: f(4),
            total_count: f(8),
            name_hash_bit_size: f(12),
            index_size_total: f(16),
            index_size_extra: f(20),
            index_size: f(24),
            index_table_size: f(28),
            name_hashes: Vec::new(),
            index_bits: Vec::new(),
        };
        let mut off = 32usize;
        let nh_end = (off + t.total_count as usize).min(body.len());
        t.name_hashes = body[off..nh_end].to_vec();
        off = nh_end;
        let ib_end = (off + t.index_table_size as usize).min(body.len());
        t.index_bits = body[off..ib_end].to_vec();
        Ok(t)
    }

    /// 只用 HET 解析出的**候选**块索引（不保证正确，需 BET 复核）。
    pub fn candidate_index(&self, name: &str) -> Option<u32> {
        if self.entry_count == 0 || self.total_count == 0 || self.name_hashes.is_empty() {
            return None;
        }
        let want = jenkins_name_hash(name, self.name_hash_bit_size);
        let name_hash1 = ((want >> (self.name_hash_bit_size - 8)) & 0xFF) as u8;
        let start = (want % self.total_count as u64) as usize;
        let mut index = start;
        loop {
            if index >= self.name_hashes.len() {
                return None;
            }
            if self.name_hashes[index] == HET_ENTRY_FREE {
                return None;
            }
            if self.name_hashes[index] == name_hash1 {
                let bitpos = self.index_size_total as usize * index;
                let cand = bits_get(&self.index_bits, bitpos, self.index_size) as u32;
                if cand != u32::MAX {
                    return Some(cand);
                }
            }
            index = (index + 1) % self.total_count as usize;
            if index == start {
                return None;
            }
        }
    }
}

// --------------------------------------------------------------------------- //
// BET
// --------------------------------------------------------------------------- //

/// BET 表：块表字段的位打包副本 + 名字哈希的低 56 位。
#[derive(Clone, Debug, Default)]
pub struct BetTable {
    pub table_size: u32,
    pub entry_count: u32,
    pub unknown08: u32,
    pub table_entry_size: u32,
    pub bit_index_file_pos: u32,
    pub bit_index_file_size: u32,
    pub bit_index_cmp_size: u32,
    pub bit_index_flag_index: u32,
    pub bit_index_unknown: u32,
    pub bit_count_file_pos: u32,
    pub bit_count_file_size: u32,
    pub bit_count_cmp_size: u32,
    pub bit_count_flag_index: u32,
    pub bit_count_unknown: u32,
    pub bit_total_name_hash2: u32,
    pub bit_extra_name_hash2: u32,
    pub bit_count_name_hash2: u32,
    pub name_hash_array_size: u32,
    pub flag_count: u32,
    pub flags: Vec<u32>,
    pub file_bits: Vec<u8>,
    pub name_hash_bits: Vec<u8>,
}

impl BetTable {
    fn body(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for v in [
            self.table_size,
            self.entry_count,
            self.unknown08,
            self.table_entry_size,
            self.bit_index_file_pos,
            self.bit_index_file_size,
            self.bit_index_cmp_size,
            self.bit_index_flag_index,
            self.bit_index_unknown,
            self.bit_count_file_pos,
            self.bit_count_file_size,
            self.bit_count_cmp_size,
            self.bit_count_flag_index,
            self.bit_count_unknown,
            self.bit_total_name_hash2,
            self.bit_extra_name_hash2,
            self.bit_count_name_hash2,
            self.name_hash_array_size,
            self.flag_count,
        ] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        // 实测：SC2 在磁盘上把 flag 数组存成小端。照抄 StormLib 的 BSWAP 会得到废值。
        for f in &self.flags {
            out.extend_from_slice(&f.to_le_bytes());
        }
        out.extend_from_slice(&self.file_bits);
        out.extend_from_slice(&self.name_hash_bits);
        out
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let body = self.body();
        // 注意密钥来源：BET 用 (block table)，HET 用 (hash table)。
        let key = hash_string("(block table)", HASH_FILE_KEY);
        let enc = encrypt(&body, key);
        let mut out = Vec::with_capacity(12 + enc.len());
        out.extend_from_slice(BET_SIG);
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&enc);
        out
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 12 || &data[0..4] != BET_SIG {
            return Err(bad_table("不是 BET 表（签名不匹配或长度不足 12）"));
        }
        let data_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        let end = (12 + data_size).min(data.len());
        let key = hash_string("(block table)", HASH_FILE_KEY);
        let body = crate::crypto::decrypt(&data[12..end], key);
        if body.len() < 76 {
            return Err(bad_table("BET 表体不足 76 字节"));
        }
        let f = |i: usize| u32::from_le_bytes([body[i], body[i + 1], body[i + 2], body[i + 3]]);
        let mut t = Self {
            table_size: f(0),
            entry_count: f(4),
            unknown08: f(8),
            table_entry_size: f(12),
            bit_index_file_pos: f(16),
            bit_index_file_size: f(20),
            bit_index_cmp_size: f(24),
            bit_index_flag_index: f(28),
            bit_index_unknown: f(32),
            bit_count_file_pos: f(36),
            bit_count_file_size: f(40),
            bit_count_cmp_size: f(44),
            bit_count_flag_index: f(48),
            bit_count_unknown: f(52),
            bit_total_name_hash2: f(56),
            bit_extra_name_hash2: f(60),
            bit_count_name_hash2: f(64),
            name_hash_array_size: f(68),
            flag_count: f(72),
            ..Default::default()
        };
        let mut off = 76usize;
        for i in 0..t.flag_count as usize {
            let p = off + i * 4;
            if p + 4 > body.len() {
                return Err(bad_table("BET flag 数组越界"));
            }
            t.flags.push(f(p));
        }
        off += 4 * t.flag_count as usize;
        let file_bytes = (t.table_entry_size as usize * t.entry_count as usize).div_ceil(8);
        let fend = (off + file_bytes).min(body.len());
        t.file_bits = body[off..fend].to_vec();
        off = fend;
        let hend = (off + t.name_hash_array_size as usize).min(body.len());
        t.name_hash_bits = body[off..hend].to_vec();
        Ok(t)
    }

    /// 返回第 i 条目的 (file_pos, file_size, cmp_size, flag_index)。
    pub fn entry(&self, i: u32) -> (u64, u64, u64, u64) {
        let base = self.table_entry_size as usize * i as usize;
        let pos = bits_get(
            &self.file_bits,
            base + self.bit_index_file_pos as usize,
            self.bit_count_file_pos,
        );
        let size = bits_get(
            &self.file_bits,
            base + self.bit_index_file_size as usize,
            self.bit_count_file_size,
        );
        let cmp = bits_get(
            &self.file_bits,
            base + self.bit_index_cmp_size as usize,
            self.bit_count_cmp_size,
        );
        let flag = bits_get(
            &self.file_bits,
            base + self.bit_index_flag_index as usize,
            self.bit_count_flag_index,
        );
        (pos, size, cmp, flag)
    }

    /// 第 i 条目里存的**名字哈希低 56 位**。
    pub fn name_hash2(&self, i: u32) -> u64 {
        bits_get(
            &self.name_hash_bits,
            self.bit_total_name_hash2 as usize * i as usize,
            self.bit_count_name_hash2,
        )
    }
}

// --------------------------------------------------------------------------- //
// (attributes)
// --------------------------------------------------------------------------- //

/// (attributes) 表：逐块的 CRC32 / MD5。
///
/// 实测约定：(listfile) 的 MD5 全零但 CRC32 是真实值；(attributes) 自身的
/// CRC32 = 0 且 MD5 全零。**MD5 全零代表「跳过校验」，不是「内容为空」**。
#[derive(Clone, Debug, Default)]
pub struct Attributes {
    pub version: u32,
    pub flags: u32,
    pub crcs: Option<Vec<u32>>,
    pub filetimes: Option<Vec<u64>>,
    pub md5s: Option<Vec<[u8; 16]>>,
}

impl Attributes {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&self.flags.to_le_bytes());
        if self.flags & ATTR_CRC32 != 0 {
            for v in self.crcs.as_deref().unwrap_or_default() {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        if self.flags & ATTR_FILETIME != 0 {
            for v in self.filetimes.as_deref().unwrap_or_default() {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        if self.flags & ATTR_MD5 != 0 {
            for v in self.md5s.as_deref().unwrap_or_default() {
                out.extend_from_slice(v);
            }
        }
        out
    }

    pub fn parse(data: &[u8], block_count: usize) -> Result<Self> {
        if data.len() < 8 {
            return Err(bad_table("(attributes) 不足 8 字节"));
        }
        let version = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let flags = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let mut out = Self {
            version,
            flags,
            ..Default::default()
        };
        let mut p = 8usize;
        if flags & ATTR_CRC32 != 0 {
            let mut v = Vec::with_capacity(block_count);
            for i in 0..block_count {
                let q = p + i * 4;
                if q + 4 > data.len() {
                    return Err(bad_table("(attributes) CRC 数组越界"));
                }
                v.push(u32::from_le_bytes([
                    data[q],
                    data[q + 1],
                    data[q + 2],
                    data[q + 3],
                ]));
            }
            p += 4 * block_count;
            out.crcs = Some(v);
        }
        if flags & ATTR_FILETIME != 0 {
            let mut v = Vec::with_capacity(block_count);
            for i in 0..block_count {
                let q = p + i * 8;
                if q + 8 > data.len() {
                    return Err(bad_table("(attributes) FILETIME 数组越界"));
                }
                v.push(u64::from_le_bytes([
                    data[q],
                    data[q + 1],
                    data[q + 2],
                    data[q + 3],
                    data[q + 4],
                    data[q + 5],
                    data[q + 6],
                    data[q + 7],
                ]));
            }
            p += 8 * block_count;
            out.filetimes = Some(v);
        }
        if flags & ATTR_MD5 != 0 {
            let mut v = Vec::with_capacity(block_count);
            for i in 0..block_count {
                let q = p + i * 16;
                if q + 16 > data.len() {
                    return Err(bad_table("(attributes) MD5 数组越界"));
                }
                let mut m = [0u8; 16];
                m.copy_from_slice(&data[q..q + 16]);
                v.push(m);
            }
            out.md5s = Some(v);
        }
        Ok(out)
    }
}

// --------------------------------------------------------------------------- //
// 构造器（写侧）
// --------------------------------------------------------------------------- //

/// 计算一段数据的 MD5。
pub fn md5_of(data: &[u8]) -> [u8; 16] {
    let mut hasher = Md5::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest);
    out
}

/// 构造 HET 表。槽位按**名字列表顺序**插入 —— 顺序变了 HET 字节就全变。
pub fn build_het(names: &[String], name_hash_bit_size: u32) -> Result<HetTable> {
    let entry_count = names.len() as u32;
    let mut total_count = (entry_count * 4) / 3;
    if total_count < entry_count {
        total_count = entry_count;
    }
    let index_size_total = necessary_bit_count(entry_count as u64).max(1);
    let index_table_size = (total_count as u64 * index_size_total as u64).div_ceil(8);
    let mut name_hashes = vec![0u8; total_count as usize];
    let mut index_bits = vec![0xFFu8; index_table_size as usize];

    for (i, name) in names.iter().enumerate() {
        let fh = jenkins_name_hash(name, name_hash_bit_size);
        let nh1 = ((fh >> (name_hash_bit_size - 8)) & 0xFF) as u8;
        let mut idx = (fh % total_count as u64) as usize;
        let start = idx;
        while name_hashes[idx] != HET_ENTRY_FREE {
            idx = (idx + 1) % total_count as usize;
            if idx == start {
                return Err(MpqError::HetTableFull);
            }
        }
        name_hashes[idx] = nh1;
        bits_set(
            &mut index_bits,
            index_size_total as usize * idx,
            index_size_total,
            i as u64,
        );
    }

    Ok(HetTable {
        table_size: 32 + total_count + index_table_size as u32,
        entry_count,
        total_count,
        name_hash_bit_size,
        index_size_total,
        index_size_extra: 0,
        index_size: index_size_total,
        index_table_size: index_table_size as u32,
        name_hashes,
        index_bits,
    })
}

/// 构造 BET 表。flag 数组按块索引下 flags 的**首次出现顺序**去重收集。
pub fn build_bet(
    blocks: &[BlockEntry],
    names: &[String],
    name_hash_bit_size: u32,
    unknown08: u32,
) -> BetTable {
    let entry_count = blocks.len() as u32;
    let mut flag_array: Vec<u32> = Vec::new();
    let mut flag_indexes: Vec<u64> = Vec::with_capacity(blocks.len());
    for b in blocks {
        match flag_array.iter().position(|f| *f == b.flags) {
            Some(p) => flag_indexes.push(p as u64),
            None => {
                flag_array.push(b.flags);
                flag_indexes.push((flag_array.len() - 1) as u64);
            }
        }
    }

    let max_pos = blocks.iter().map(|b| b.file_pos as u64).max().unwrap_or(0);
    let max_fsize = blocks.iter().map(|b| b.file_size as u64).max().unwrap_or(0);
    let max_csize = blocks.iter().map(|b| b.cmp_size as u64).max().unwrap_or(0);
    let bc_pos = necessary_bit_count(max_pos);
    let bc_fsize = necessary_bit_count(max_fsize);
    let bc_csize = necessary_bit_count(max_csize);
    let bc_flag = necessary_bit_count(flag_indexes.iter().copied().max().map_or(0, |m| m + 1));

    let bi_pos = 0u32;
    let bi_fsize = bi_pos + bc_pos;
    let bi_csize = bi_fsize + bc_fsize;
    let bi_flag = bi_csize + bc_csize;
    let bi_unknown = bi_flag + bc_flag;
    let table_entry_size = bi_unknown;

    let bit_total_nh2 = name_hash_bit_size - 8;
    let name_hash_array_size = (bit_total_nh2 as u64 * entry_count as u64).div_ceil(8);
    let entry_bytes = (table_entry_size as u64 * entry_count as u64).div_ceil(8);
    let table_size = 76 + flag_array.len() as u32 * 4 + entry_bytes as u32 + name_hash_array_size as u32;

    let mut file_bits = vec![0u8; entry_bytes as usize];
    let mut hash_bits = vec![0u8; name_hash_array_size as usize];

    for (i, b) in blocks.iter().enumerate() {
        let base = table_entry_size as usize * i;
        bits_set(&mut file_bits, base + bi_pos as usize, bc_pos, b.file_pos as u64);
        bits_set(
            &mut file_bits,
            base + bi_fsize as usize,
            bc_fsize,
            b.file_size as u64,
        );
        bits_set(
            &mut file_bits,
            base + bi_csize as usize,
            bc_csize,
            b.cmp_size as u64,
        );
        bits_set(&mut file_bits, base + bi_flag as usize, bc_flag, flag_indexes[i]);
        let fh = jenkins_name_hash(&names[i], name_hash_bit_size);
        bits_set(
            &mut hash_bits,
            bit_total_nh2 as usize * i,
            bit_total_nh2,
            fh & ((1u64 << bit_total_nh2) - 1),
        );
    }

    BetTable {
        table_size,
        entry_count,
        unknown08,
        table_entry_size,
        bit_index_file_pos: bi_pos,
        bit_index_file_size: bi_fsize,
        bit_index_cmp_size: bi_csize,
        bit_index_flag_index: bi_flag,
        bit_index_unknown: bi_unknown,
        bit_count_file_pos: bc_pos,
        bit_count_file_size: bc_fsize,
        bit_count_cmp_size: bc_csize,
        bit_count_flag_index: bc_flag,
        bit_count_unknown: 0,
        bit_total_name_hash2: bit_total_nh2,
        bit_extra_name_hash2: 0,
        bit_count_name_hash2: bit_total_nh2,
        name_hash_array_size: name_hash_array_size as u32,
        flag_count: flag_array.len() as u32,
        flags: flag_array,
        file_bits,
        name_hash_bits: hash_bits,
    }
}

/// 构造经典哈希表。count 必须是 2 的幂，且**必须大于条目数**（至少留一个空槽）。
pub fn build_hash_table(names: &[String], count: u32) -> Result<Vec<HashEntry>> {
    let mut entries = vec![HashEntry::free(); count as usize];
    for (i, name) in names.iter().enumerate() {
        let mut slot = (hash_string(name, HASH_TABLE_OFFSET) & (count - 1)) as usize;
        let start = slot;
        while !entries[slot].is_free() {
            slot = (slot + 1) & (count as usize - 1);
            if slot == start {
                return Err(MpqError::HashTableFull);
            }
        }
        entries[slot] = HashEntry {
            hash_a: hash_string(name, HASH_NAME_A),
            hash_b: hash_string(name, HASH_NAME_B),
            locale: 0,
            platform: 0,
            block_index: i as u32,
        };
    }
    Ok(entries)
}

/// 构造 (attributes)。
pub fn build_attributes(
    blocks: &[BlockEntry],
    names: &[String],
    datas: &[Vec<u8>],
    version: u32,
    flags: u32,
) -> Attributes {
    let n = blocks.len();
    let mut a = Attributes {
        version,
        flags,
        crcs: if flags & ATTR_CRC32 != 0 {
            Some(vec![0u32; n])
        } else {
            None
        },
        filetimes: if flags & ATTR_FILETIME != 0 {
            Some(vec![0u64; n])
        } else {
            None
        },
        md5s: if flags & ATTR_MD5 != 0 {
            Some(vec![MD5_ZERO; n])
        } else {
            None
        },
    };
    for i in 0..n {
        let Some(name) = names.get(i) else { continue };
        if name == "(attributes)" {
            continue;
        }
        let Some(data) = datas.get(i) else { continue };
        if flags & ATTR_CRC32 != 0 {
            if let Some(crcs) = a.crcs.as_mut() {
                crcs[i] = crc32fast::hash(data);
            }
        }
        if flags & ATTR_MD5 != 0 && name != "(listfile)" {
            if let Some(md5s) = a.md5s.as_mut() {
                md5s[i] = md5_of(data);
            }
        }
    }
    a
}

/// 打包一个组件为 SINGLE_UNIT 块：返回 (载荷, 块条目)。空组件不推进偏移。
pub fn pack_single(data: &[u8], file_pos: u32) -> Result<(Vec<u8>, BlockEntry)> {
    if data.is_empty() {
        return Ok((
            Vec::new(),
            BlockEntry {
                file_pos,
                cmp_size: 0,
                file_size: 0,
                flags: crate::FLAG_EXISTS | crate::FLAG_SINGLE_UNIT,
            },
        ));
    }
    let (payload, compressed) = best_compress(data)?;
    let flags =
        crate::FLAG_EXISTS | crate::FLAG_SINGLE_UNIT | if compressed { crate::FLAG_COMPRESS } else { 0 };
    Ok((
        payload.clone(),
        BlockEntry {
            file_pos,
            cmp_size: payload.len() as u32,
            file_size: data.len() as u32,
            flags,
        },
    ))
}
