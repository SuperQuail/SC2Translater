//! MPQ 的多压缩掩码。
//!
//! ## 最大的坑：cmp == size 代表「原样存储」
//!
//! 真实归档里存在 **flags 带 COMPRESS 但数据没有压缩** 的块（实测 paiur01 有 16 个），
//! 判据是 cmp_size == file_size。这时**首字节是数据本身、不是掩码**
//! （例如 DocumentInfo.version 首字节 0x63，正好是 cdes 的 c）。
//! 所以解压函数必须**先判长度、再看掩码**，反过来会把数据的第一个字节吃掉。

use std::io::Write;

use crate::MpqError;

pub const MASK_ZLIB: u8 = 0x02;
pub const MASK_PKWARE: u8 = 0x08;
pub const MASK_BZIP2: u8 = 0x10;
pub const MASK_LZMA: u8 = 0x12;
pub const MASK_SPARSE: u8 = 0x20;

/// 解压一个块。
///
/// expected 是解压后应有的长度；等于 chunk 长度时说明是原样存储。
pub fn decompress_chunk(chunk: &[u8], expected: usize) -> Result<Vec<u8>, MpqError> {
    // 必须最先判定：cmp == size 表示原样存储
    if chunk.len() == expected {
        return Ok(chunk.to_vec());
    }
    if chunk.is_empty() {
        return Ok(Vec::new());
    }
    let mask = chunk[0];
    let payload = &chunk[1..];

    // 精确值优先。0x12(LZMA) 同时含 0x02(zlib) 与 0x10(bzip2) 两个位，
    // 按位测试会被 zlib 先截住 —— Python 原型就是这个顺序，那是个已知瑕疵。
    match mask {
        0x00 => Ok(payload.to_vec()),
        MASK_LZMA => lzma_decompress(payload, mask),
        MASK_SPARSE => Ok(payload.to_vec()),
        MASK_PKWARE => Err(MpqError::UnsupportedCompression { mask }),
        _ => {
            if mask & MASK_ZLIB != 0 {
                zlib_decompress(payload, mask)
            } else if mask & MASK_BZIP2 != 0 {
                bzip2_decompress(payload, mask)
            } else if mask & MASK_SPARSE != 0 {
                Ok(payload.to_vec())
            } else {
                Err(MpqError::BadCompressionMask { mask })
            }
        }
    }
}

fn zlib_decompress(payload: &[u8], mask: u8) -> Result<Vec<u8>, MpqError> {
    let mut out = Vec::new();
    let mut decoder = flate2::read::ZlibDecoder::new(payload);
    std::io::Read::read_to_end(&mut decoder, &mut out).map_err(|e| MpqError::Decompress {
        mask,
        reason: format!("zlib: {e}"),
    })?;
    Ok(out)
}

#[cfg(feature = "bzip2-backend")]
fn bzip2_decompress(payload: &[u8], mask: u8) -> Result<Vec<u8>, MpqError> {
    let mut out = Vec::new();
    let mut decoder = bzip2::read::BzDecoder::new(payload);
    std::io::Read::read_to_end(&mut decoder, &mut out).map_err(|e| MpqError::Decompress {
        mask,
        reason: format!("bzip2: {e}"),
    })?;
    Ok(out)
}

#[cfg(not(feature = "bzip2-backend"))]
fn bzip2_decompress(_payload: &[u8], mask: u8) -> Result<Vec<u8>, MpqError> {
    Err(MpqError::UnsupportedCompression { mask })
}

fn lzma_decompress(payload: &[u8], mask: u8) -> Result<Vec<u8>, MpqError> {
    let mut out = Vec::new();
    lzma_rs::lzma_decompress(&mut std::io::Cursor::new(payload), &mut out).map_err(|e| {
        MpqError::Decompress {
            mask,
            reason: format!("lzma: {e:?}"),
        }
    })?;
    Ok(out)
}

/// 压缩候选：bzip2 与 zlib 各来一份，取小的；压不动就原样返回。
///
/// 实测：bz2 level 9 的输出与暴雪产物**逐字节相同**（Triggers 145291、
/// Objects 63577、MapScript.galaxy 23046）。
pub fn best_compress(data: &[u8]) -> Result<(Vec<u8>, bool), MpqError> {
    if data.is_empty() {
        return Ok((Vec::new(), false));
    }
    let mut best: Option<Vec<u8>> = None;

    #[cfg(feature = "bzip2-backend")]
    {
        let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::best());
        if let Ok(body) = encoder.write_all(data).and_then(|()| encoder.finish()) {
            let mut v = Vec::with_capacity(body.len() + 1);
            v.push(MASK_BZIP2);
            v.extend_from_slice(&body);
            best = Some(v);
        }
    }

    {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        encoder.write_all(data)?;
        let body = encoder.finish()?;
        let mut v = Vec::with_capacity(body.len() + 1);
        v.push(MASK_ZLIB);
        v.extend_from_slice(&body);
        if best.as_ref().is_none_or(|b| v.len() < b.len()) {
            best = Some(v);
        }
    }

    match best {
        Some(v) if v.len() < data.len() => Ok((v, true)),
        _ => Ok((data.to_vec(), false)),
    }
}
