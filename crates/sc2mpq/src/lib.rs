//! # sc2mpq —— 星际争霸 II 文档（.SC2Map / .SC2Mod）的 MPQ 容器层
//!
//! 这是 Python 原型（SC2Diff 的 tools/sc2mpq.py）的 Rust 重写，
//! 实现规格见仓库内 docs/research/mpq-spec.md（逐条带证据行号）。
//!
//! ## SC2 用的不是「标准 MPQ」
//!
//! | 事实 | 说明 |
//! | --- | --- |
//! | 头是 **208 字节 / formatVersion=3 / v4 的 size64 字段布局** | 按 headerSize 判断字段是否存在，不要按版本号 |
//! | 头部 0x70..0xCF 有 **6 个 MD5 摘要** | 缺了或错了会被判 ERROR_FAKE_MPQ_HEADER（StormLib 10009） |
//! | **dwRawChunkSize（0x6C）必须写 0** | 暴雪写入器写 16384，那会打开 System_Mopaq 的 raw-chunk MD5 校验，导致任何内容改动都被编辑器判为损坏 |
//! | **HET/BET 是权威索引** | 有 HET 就必须走 HET，且命中后要用 BET 的 64 位名字哈希复核 |
//! | BET 的 flag 数组在磁盘上是**小端** | 照抄 StormLib 的 BSWAP 会得到废值 |
//!
//! ## 模块
//!
//! * crypto   —— 哈希表加密、块密码、Jenkins lookup3 名字哈希、LSB-first 位数组
//! * compress —— 多压缩掩码的识别与解压、压缩候选选择
//! * tables   —— hash/block/HET/BET/attributes 五张表的解析与**对称**序列化
//! * archive  —— Archive（只读）与 Document（可写、可重建）

pub mod archive;
pub mod compress;
pub mod crypto;
pub mod tables;

pub use archive::{Archive, BuildInfo, BuildOptions, Document, Layout, VerifyReport, build_document};
pub use compress::{best_compress, decompress_chunk};
pub use crypto::{
    HASH_FILE_KEY, HASH_NAME_A, HASH_NAME_B, HASH_TABLE_OFFSET, hash_string, jenkins_name_hash,
};
pub use tables::{Attributes, BetTable, BlockEntry, HashEntry, HetTable};

/// MPQ 文件魔数。
pub const MAGIC: &[u8; 4] = b"MPQ\x1a";
/// HET 表魔数。
pub const HET_SIG: &[u8; 4] = b"HET\x1a";
/// BET 表魔数。
pub const BET_SIG: &[u8; 4] = b"BET\x1a";

/// 空的哈希槽标记。
pub const HASH_ENTRY_FREE: u32 = 0xFFFF_FFFF;
/// 空的 HET 槽标记。
pub const HET_ENTRY_FREE: u8 = 0x00;

pub const FLAG_IMPLODE: u32 = 0x0000_0100;
pub const FLAG_COMPRESS: u32 = 0x0000_0200;
pub const FLAG_ENCRYPTED: u32 = 0x0001_0000;
pub const FLAG_FIX_KEY: u32 = 0x0002_0000;
pub const FLAG_PATCH: u32 = 0x0010_0000;
pub const FLAG_SINGLE_UNIT: u32 = 0x0100_0000;
pub const FLAG_DELETE_MARKER: u32 = 0x0200_0000;
pub const FLAG_SECTOR_CRC: u32 = 0x0400_0000;
pub const FLAG_EXISTS: u32 = 0x8000_0000;

pub const ATTR_CRC32: u32 = 0x01;
pub const ATTR_FILETIME: u32 = 0x02;
pub const ATTR_MD5: u32 = 0x04;

/// 默认块大小位移：512 << 5 = 16 KiB。
pub const DEFAULT_BLOCK_SHIFT: u16 = 5;

/// SC2 文档头固定长度（实测）。
pub const HEADER_SIZE_V3: usize = 208;

/// 簿记项：这些不是用户的业务组件，构建时由写入器自己生成。
pub const BOOKKEEPING: [&str; 4] = ["(listfile)", "(attributes)", "(signature)", "(user data)"];

/// 判断某个组件名是不是簿记项。
pub fn is_bookkeeping(name: &str) -> bool {
    BOOKKEEPING.contains(&name)
}

/// 全 MD5 零值（簿记项的 MD5 约定为全零）。
pub const MD5_ZERO: [u8; 16] = [0u8; 16];

/// 统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum MpqError {
    #[error("不是 MPQ 归档（魔数不匹配）: {0}")]
    NotAnArchive(String),
    #[error("文件太短: 需要 {need} 字节，实际 {got}")]
    TooShort { need: usize, got: usize },
    #[error("归档里没有组件: {0}")]
    MissingComponent(String),
    #[error("经典 hash 表装不下（full）")]
    HashTableFull,
    #[error("HET 表装不下（full）")]
    HetTableFull,
    #[error("不认识的压缩掩码: 0x{mask:02X}")]
    BadCompressionMask { mask: u8 },
    #[error("这个压缩方式本实现不支持: 0x{mask:02X}")]
    UnsupportedCompression { mask: u8 },
    #[error("解压失败（掩码 {mask}）: {reason}")]
    Decompress { mask: u8, reason: String },
    #[error("表结构损坏: {0}")]
    BadTable(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, MpqError>;
