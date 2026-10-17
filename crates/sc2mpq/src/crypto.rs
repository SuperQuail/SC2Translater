//! MPQ 的哈希与加解密原语。
//!
//! 三件事容易写错，都在这里一次性做对：
//!
//! 1. hash_string 的查表索引是 (hash_type << 8) + byte，**不要再 & 0x4FF** ——
//!    那会清掉 bit 8，让所有 hash_type != 0 塌缩成 0。
//! 2. 块密码**不是自逆的**：解密用密文推进 seed2，加密用明文推进。
//! 3. 块密码只处理整 4 字节，尾部 1~3 字节**原样搬运**；丢掉会截断 HET 表
//!    （实测 paiur01 的 HET dataSize = 199，199 % 4 = 3）。

use std::sync::OnceLock;

use crate::MpqError;

const CRYPT_TABLE_SIZE: usize = 0x500;

/// 哈希类型：经典 hash 表的寻址。
pub const HASH_TABLE_OFFSET: u32 = 0;
/// 哈希类型：名字哈希 A。
pub const HASH_NAME_A: u32 = 1;
/// 哈希类型：名字哈希 B。
pub const HASH_NAME_B: u32 = 2;
/// 哈希类型：文件 key。
pub const HASH_FILE_KEY: u32 = 3;

fn crypt_table() -> &'static [u32; CRYPT_TABLE_SIZE] {
    static TABLE: OnceLock<[u32; CRYPT_TABLE_SIZE]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0u32; CRYPT_TABLE_SIZE];
        let mut seed: u32 = 0x0010_0001;
        for i1 in 0..0x100usize {
            let mut i2 = i1;
            for _ in 0..5 {
                seed = (seed.wrapping_mul(125).wrapping_add(3)) % 0x2A_AAAB;
                let t1 = (seed & 0xFFFF) << 0x10;
                seed = (seed.wrapping_mul(125).wrapping_add(3)) % 0x2A_AAAB;
                let t2 = seed & 0xFFFF;
                table[i2] = t1 | t2;
                i2 += 0x100;
            }
        }
        table
    })
}

/// 经典 MPQ 字符串哈希（大小写不敏感）。
///
/// Python 原型用 text.upper().encode("latin-1")；组件名都是 ASCII，所以这里用
/// to_ascii_uppercase 与之等价（非 ASCII 输入两侧行为不同，刻意不做 Unicode 大小写折叠）。
pub fn hash_string(text: &str, hash_type: u32) -> u32 {
    let table = crypt_table();
    let mut seed1: u32 = 0x7FED_7FED;
    let mut seed2: u32 = 0xEEEE_EEEE;
    for byte in text.bytes().map(|b| b.to_ascii_uppercase()) {
        let index = ((hash_type << 8) + byte as u32) as usize;
        seed1 = table[index] ^ seed1.wrapping_add(seed2);
        seed2 = (byte as u32)
            .wrapping_add(seed1)
            .wrapping_add(seed2)
            .wrapping_add(seed2 << 5)
            .wrapping_add(3);
    }
    seed1
}

/// MPQ 块密码：解密。seed2 用**密文**推进。
pub fn decrypt(data: &[u8], key: u32) -> Vec<u8> {
    let table = crypt_table();
    let mut seed1 = key;
    let mut seed2: u32 = 0xEEEE_EEEE;
    let mut out = Vec::with_capacity(data.len());
    let whole = data.len() & !3;
    for chunk in data[..whole].chunks_exact(4) {
        let mut dw = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        seed2 = seed2.wrapping_add(table[0x400 + (seed1 & 0xFF) as usize]);
        dw ^= seed1.wrapping_add(seed2);
        seed1 = ((!seed1).wrapping_shl(0x15)).wrapping_add(0x1111_1111) | (seed1 >> 0x0B);
        seed2 = dw.wrapping_add(seed2).wrapping_add(seed2 << 5).wrapping_add(3);
        out.extend_from_slice(&dw.to_le_bytes());
    }
    out.extend_from_slice(&data[whole..]);
    out
}

/// MPQ 块密码：加密。**不是**解密的自逆操作 —— seed2 用**明文**推进。
pub fn encrypt(data: &[u8], key: u32) -> Vec<u8> {
    let table = crypt_table();
    let mut seed1 = key;
    let mut seed2: u32 = 0xEEEE_EEEE;
    let mut out = Vec::with_capacity(data.len());
    let whole = data.len() & !3;
    for chunk in data[..whole].chunks_exact(4) {
        let plain = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        seed2 = seed2.wrapping_add(table[0x400 + (seed1 & 0xFF) as usize]);
        let cipher = plain ^ seed1.wrapping_add(seed2);
        seed1 = ((!seed1).wrapping_shl(0x15)).wrapping_add(0x1111_1111) | (seed1 >> 0x0B);
        seed2 = plain.wrapping_add(seed2).wrapping_add(seed2 << 5).wrapping_add(3);
        out.extend_from_slice(&cipher.to_le_bytes());
    }
    out.extend_from_slice(&data[whole..]);
    out
}

// --------------------------------------------------------------------------- //
// Jenkins lookup3 —— HET/BET 用的 64 位文件名哈希
// --------------------------------------------------------------------------- //

#[inline]
fn rot(x: u32, k: u32) -> u32 {
    x.rotate_left(k)
}

fn mix(mut a: u32, mut b: u32, mut c: u32) -> (u32, u32, u32) {
    a = a.wrapping_sub(c);
    a ^= rot(c, 4);
    c = c.wrapping_add(b);
    b = b.wrapping_sub(a);
    b ^= rot(a, 6);
    a = a.wrapping_add(c);
    c = c.wrapping_sub(b);
    c ^= rot(b, 8);
    b = b.wrapping_add(a);
    a = a.wrapping_sub(c);
    a ^= rot(c, 16);
    c = c.wrapping_add(b);
    b = b.wrapping_sub(a);
    b ^= rot(a, 19);
    a = a.wrapping_add(c);
    c = c.wrapping_sub(b);
    c ^= rot(b, 4);
    b = b.wrapping_add(a);
    (a, b, c)
}

fn final_mix(mut a: u32, mut b: u32, mut c: u32) -> (u32, u32, u32) {
    c ^= b;
    c = c.wrapping_sub(rot(b, 14));
    a ^= c;
    a = a.wrapping_sub(rot(c, 11));
    b ^= a;
    b = b.wrapping_sub(rot(a, 25));
    c ^= b;
    c = c.wrapping_sub(rot(b, 16));
    a ^= c;
    a = a.wrapping_sub(rot(c, 4));
    b ^= a;
    b = b.wrapping_sub(rot(a, 14));
    c ^= b;
    c = c.wrapping_sub(rot(b, 24));
    (a, b, c)
}

/// lookup3 的 hashlittle2，小端。返回 (pc_result, pb_result) 即 (c, b)。
pub fn hashlittle2(key: &[u8], init_a: u32, init_b: u32) -> (u32, u32) {
    let length = key.len() as u32;
    let base = 0xDEAD_BEEFu32.wrapping_add(length).wrapping_add(init_a);
    let mut a = base;
    let mut b = base;
    let mut c = base.wrapping_add(init_b);

    let mut pos = 0usize;
    let mut remaining = key.len();
    // 注意用 > 而不是 >=：与 lookup3 / Python 原型一致（12 字节整块留给尾部贯穿写法）。
    while remaining > 12 {
        a = a.wrapping_add(u32::from_le_bytes([
            key[pos],
            key[pos + 1],
            key[pos + 2],
            key[pos + 3],
        ]));
        b = b.wrapping_add(u32::from_le_bytes([
            key[pos + 4],
            key[pos + 5],
            key[pos + 6],
            key[pos + 7],
        ]));
        c = c.wrapping_add(u32::from_le_bytes([
            key[pos + 8],
            key[pos + 9],
            key[pos + 10],
            key[pos + 11],
        ]));
        let (na, nb, nc) = mix(a, b, c);
        a = na;
        b = nb;
        c = nc;
        pos += 12;
        remaining -= 12;
    }

    let tail = &key[pos..pos + remaining];
    if remaining >= 1 {
        a = a.wrapping_add(tail[0] as u32);
    }
    if remaining >= 2 {
        a = a.wrapping_add((tail[1] as u32) << 8);
    }
    if remaining >= 3 {
        a = a.wrapping_add((tail[2] as u32) << 16);
    }
    if remaining >= 4 {
        a = a.wrapping_add((tail[3] as u32) << 24);
    }
    if remaining >= 5 {
        b = b.wrapping_add(tail[4] as u32);
    }
    if remaining >= 6 {
        b = b.wrapping_add((tail[5] as u32) << 8);
    }
    if remaining >= 7 {
        b = b.wrapping_add((tail[6] as u32) << 16);
    }
    if remaining >= 8 {
        b = b.wrapping_add((tail[7] as u32) << 24);
    }
    if remaining >= 9 {
        c = c.wrapping_add(tail[8] as u32);
    }
    if remaining >= 10 {
        c = c.wrapping_add((tail[9] as u32) << 8);
    }
    if remaining >= 11 {
        c = c.wrapping_add((tail[10] as u32) << 16);
    }
    if remaining >= 12 {
        c = c.wrapping_add((tail[11] as u32) << 24);
    }

    let (_, b, c) = final_mix(a, b, c);
    (c, b)
}

/// SC2 的 64 位文件名哈希：小写化 -> lookup3 -> 强制最高位为 1。
///
/// 对应 StormLib HashStringJenkins 之后 HET/BET 代码施加的
/// FileNameHash = (jenkins(name) & AndMask64) | OrMask64。
pub fn jenkins_name_hash(name: &str, name_hash_bit_size: u32) -> u64 {
    let normalized: Vec<u8> = name.bytes().map(|b| b.to_ascii_lowercase()).collect();
    let (secondary, primary) = hashlittle2(&normalized, 2, 1);
    let raw = ((primary as u64) << 32) | secondary as u64;
    let and_mask = if name_hash_bit_size == 64 {
        u64::MAX
    } else {
        (1u64 << name_hash_bit_size) - 1
    };
    let or_mask = 1u64 << (name_hash_bit_size - 1);
    (raw & and_mask) | or_mask
}

/// 表示 value 需要多少位。
pub fn necessary_bit_count(mut max_value: u64) -> u32 {
    let mut n = 0u32;
    while max_value > 0 {
        max_value >>= 1;
        n += 1;
    }
    n
}

// --------------------------------------------------------------------------- //
// LSB-first 位数组
// --------------------------------------------------------------------------- //

/// 从 LSB-first 位数组里读 n 位。越界按 0 处理（不 panic）。
pub fn bits_get(buf: &[u8], pos: usize, n: u32) -> u64 {
    let mut v: u64 = 0;
    for i in 0..n as usize {
        let k = pos + i;
        let byte = k >> 3;
        if byte >= buf.len() {
            break;
        }
        if (buf[byte] >> (k & 7)) & 1 == 1 {
            v |= 1u64 << i;
        }
    }
    v
}

/// 往 LSB-first 位数组里写 n 位。越界忽略（不 panic）。
pub fn bits_set(buf: &mut [u8], pos: usize, n: u32, val: u64) {
    for i in 0..n as usize {
        let k = pos + i;
        let byte = k >> 3;
        if byte >= buf.len() {
            break;
        }
        if (val >> i) & 1 == 1 {
            buf[byte] |= 1u8 << (k & 7);
        } else {
            buf[byte] &= !(1u8 << (k & 7));
        }
    }
}

/// 把路径分隔符统一成归档内用的反斜杠，并只取最后一段。
///
/// file key 必须**去掉目录**：Base.SC2Data\\GameData\\UnitData.xml 要用 UnitData.xml 算。
pub fn base_name(name: &str) -> String {
    let normalized = name.replace('/', "\\");
    normalized.rsplit('\\').next().unwrap_or("").to_string()
}

/// 文件 key 推导：只取文件名部分（去掉目录），必要时按 FLAG_FIX_KEY 调整。
pub fn file_key(name: &str, file_pos: u32, flags: u32) -> u32 {
    let mut key = hash_string(&base_name(name), HASH_FILE_KEY);
    if flags & crate::FLAG_FIX_KEY != 0 {
        key = key.wrapping_add(file_pos);
    }
    key
}

/// 把 MpqError 的构造集中一下，避免上层到处 use。
pub(crate) fn bad_table(msg: impl Into<String>) -> MpqError {
    MpqError::BadTable(msg.into())
}
