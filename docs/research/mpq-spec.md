# SC2 文档（MPQ）容器读写层 — 可照抄落地实现规格
目标读者：用 Rust 重写这层的工程师。
证据全部来自实测代码与实盘样本，每条结论都带 `文件:行` 或函数名。
标注 **[推断]** / **[未验证]** 的地方是推理而非实测，不要当成硬约束照抄。

## 0. 证据基线与复现命令

| 代号 | 路径 | 说明 |
| --- | --- | --- |
| **P** | `D:\Code\Rust\SC2Diff\tools\sc2mpq.py`（1095 行） | Python 权威原型，逐字节可复现 |
| **P'** | `D:\Code\Rust\SC2Translater\python\src\sc2trans\vendor\sc2mpq.py` | 本仓库已 vendor，SHA256 与 P **完全一致**（`B498728C…1348D`） |
| **F** | `D:\Code\Rust\SC2Diff\research\findings.md`（383 行） | 实测结论 |
| **R** | `D:\Code\Rust\SC2Diff\src\mpq\{mod,crypto,compress,tables,archive}.rs` | 已有 Rust 移植 |
| **S** | `D:\Code\Rust\SC2Diff\reference\StormLib\src\*.cpp` | StormLib 权威 C++ |
| 样本 | `D:\Code\Rust\SC2Diff\testdata\samples\*.{SC2Map,SC2Mod}` | 5 个真实文档（paiur01 67 块 / SCORE-Other 66 / SCORE-OtherC 17 / HTXL 184 / UmojanCT 301） |
本次调研实际执行的只读探针（下列结论的来源）：

    python -c "import sys; sys.path.insert(0, r'D:\Code\Rust\SC2Diff\tools'); import sc2mpq; ..."
    # 1) 全量 dump 头部/块表/HET/BET/attributes 字段   2) a.verify(deep=True)
    # 3) 逐块比对 SECTOR_CRC 扇区表                    4) 5 样本布局顺序与 CRC/MD5 约定
    cargo check --offline --all-targets   # 通过
    cargo test  --offline --lib           # 3 passed / 3 FAILED（见 §10.1）

**P 的 `Document` 只保序、不做语义处理**；名字大小写敏感（`find_block` 拿原样名字算哈希），
而 HET/BET 的哈希是小写化后算的（P:207、P:200）。

## 1. 文件格式：208 字节 v3/v4 混合头
SC2 5.x（build 97579）写的是「**v3 语义 + 208 字节头 + v4 的 size64 字段位置**」的混合体（F:14-27）。
只按 `formatVersion` 选布局的读取器会读歪；正确做法是**按 `headerSize` 判断字段是否存在**（P:459-466、R(archive):96-103）。

| 偏移 | 宽度 | 字段 | SC2 实测值（paiur01） | 语义 / 约束 |
| --- | --- | --- | --- | --- |
| 0x00 | 4 | `magic` | `MPQ\x1a` | 必须 |
| 0x04 | 4 | `dwHeaderSize` | **208** | 规范是 68；读侧用 `>=68` / `>=208` 决定后续字段（P:462-466） |
| 0x08 | 4 | `dwArchiveSize` | 1108308 | = 文件长度 |
| 0x0C | 2 | `wFormatVersion` | **3** | 必须是 3（写 4 会让 StormLib 走 v4 校验分支） |
| 0x0E | 2 | `wBlockSize`(shift) | 5 | `block_size = 512 << 5 = 16384` |
| 0x10 | 4 | `dwHashTablePos` | 1105188 | 绝对偏移 |
| 0x14 | 4 | `dwBlockTablePos` | 1107236 | 绝对偏移 |
| 0x18 | 4 | `dwHashTableSize` | **128**（条目数，非字节） | 字节数 = `count*16` = 2048 |
| 0x1C | 4 | `dwBlockTableSize` | **67**（条目数，非字节） | 字节数 = `count*16` = 1072 |
| 0x20 | 8 | v1 遗留 | 0 | 不写 |
| 0x28 | 8 | v2 遗留（hi-block 表） | 0 | 不写 |
| 0x30 | 8 | `archiveSize64` | 1108308 | 必须与 0x08 一致（F:22） |
| 0x34 | 8 | `betTablePos64` | 1104045 | 存在条件：`headerSize >= 68` |
| 0x3C | 8 | `hetTablePos64` | 1103818 | 同上 |
| 0x44 | 8 | `hashTableSize64` | 2048 | 存在条件：`headerSize >= 208` |
| 0x4C | 8 | `blockTableSize64` | 1072 | 同上 |
| 0x54 | 8 | `hiBlockTableSize64` | 0 | 无 hi-block 表 |
| 0x5C | 8 | `hetTableSize64` | 211 | **v4 偏移**（F:24） |
| 0x64 | 8 | `betTableSize64` | 1127 | **v4 偏移**（F:24） |
| 0x6C | 4 | **`dwRawChunkSize`** | 原件 **16384**；**我们写 0** | 见 §1.1 |
| 0x70 | 16 | `MD5(blockTable)` | — | 见 §3 |
| 0x80 | 16 | `MD5(hashTable)` | — | |
| 0x90 | 16 | `MD5(hiBlockTable)` | 全零 | 无表即全零 |
| 0xA0 | 16 | `MD5(BET)` | — | |
| 0xB0 | 16 | `MD5(HET)` | — | |
| 0xC0 | 16 | `MD5(header[0x00..0xC0])` | — | **自哈希**，必须最后算 |
字段布局与写入顺序见 P:981-1010（`build_document`）与 R(archive):494-521。

### 1.1 `dwRawChunkSize` 必须写 0 —— 「编辑器拒收」的根因
- 暴雪写入器把 **v4 才有的 16384** 塞进一个自称 `formatVersion=3` 的头（F:355-361；本次实测 5 个样本 `rawchunk=16384`）。
- 这打开了暴雪自己 MPQ 实现里**按 `dwRawChunkSize` 分块的 MD5 校验**路径。崩溃日志（编辑器自带，F:343-352）：

      e_fileCorrupt  File: DocumentInfo.version  Amount: 44  Read: 0
      System_Mopaq::mopaq_read - fail to repair offset 0 amount 44
      System_Mopaq::MD5VerifyData::ValidateRead - InitializeMD5Buffer fail

- 它对照的摘要**不是** `(attributes)` 里的内容 MD5：改内容 + 同步 attributes → CRASH；把 attributes 条目清零 → CRASH；只动表间填充字节 → LOADED（F:326-339）。
- StormLib 自己也只在 v4 才写它：`SFileCreateArchive.cpp:93`

      CreateInfo.dwRawChunkSize = (dwMpqVersion >= MPQ_FORMAT_VERSION_4) ? 0x4000 : 0;

- **修复 = 在 0x6C 写 0，并重算 0xC0 自哈希**（P:992-1010）。实测：paiur01 原件 → 重建件，StormLib `SFileOpenArchive` OK、抽取 67/67、编辑器 **LOADED**（F:368-376）。

> 结论：**写侧永远填 0**；读侧把这个值当只读快照，重建时**不要**从原件继承。

### 1.2 物理布局（4 张表 + 数据区的顺序）
实测 5 个样本**全部**满足 `het < bet < hash < block`，且 `block_pos + block_count*16 == archiveSize`：

    [208B header][block data...][HET][BET][classic hash table][classic block table]

| 样本 | het@ | bet@ | hash@ | block@ | size64 | hash 条目 | BET flags |
| --- | --- | --- | --- | --- | --- | --- | --- |
| paiur01 | 1103818 | 1104045 | 1105188 | 1107236 | 1108308 | 128 | `0x81000200`,`0x84000200` |
| HTXL | 4807476 | 4808026 | 4810990 | 4815086 | 4818030 | 256 | 同上 |
| SCORE-Other | 1769666 | 1769891 | 1770948 | 1772996 | 1774052 | 128 | `0x81000200` |
| SCORE-OtherC | 38064 | 38160 | 38492 | 39004 | 39276 | 32 | `0x81000200` |
| UmojanCT | 44017395 | 44018308 | 44023120 | 44031312 | 44036128 | 512 | `0x81000200` |
`hash_count` = 不小于 `max(4, 条目数)` 的最小 2 的幂（P:962-965、R(archive):459-463）——5 样本全部吻合。
**[推断]** 这个顺序只是暴雪的写盘习惯，规范并未强制；但为了让编辑器接受，**照抄它**。

## 2. 四张表
通用约定：**所有位数组都是 LSB-first** —— 第 k 位 = 第 `k/8` 字节的第 `k%8` 位，多位值在该位序上小端（P:18-19、P:226-242，R(crypto):167-191）。

### 2.1 classic hash table（`dwHashTableSize` × 16 字节）

| 偏移 | 宽度 | 字段 |
| --- | --- | --- |
| 0 | 4 | `dwHashA = hash_string(name, HASH_NAME_A=1)` |
| 4 | 4 | `dwHashB = hash_string(name, HASH_NAME_B=2)` |
| 8 | 2 | `wLocale`：空闲 = `0xFFFF`，占用 = `0`（实测 5 样本 locale/platform 集合都是 `{0}`） |
| 10 | 2 | `wPlatform`：同上 |
| 12 | 4 | `dwBlockIndex`；空闲 = `0xFFFFFFFF`（`HASH_ENTRY_FREE`） |
- 加密密钥 = `hash_string("(hash table)", HASH_FILE_KEY=3)`（P:503-504、P:487）。
- 查找：`idx = hash_string(name, HASH_TABLE_OFFSET=0) & (count-1)`，线性探测，遇 `FREE` 停止，命中需 `hashA/hashB` 都相等（P:510-521）。
- **自洽不变量**：`count` 必须是 2 的幂（因为用 `&(count-1)` 取模）；至少有一个空闲槽；非空闲项 `block_index < block_count`。
- 不解密也能读到「看似合理」的垃圾 → 必须按不变量校验后回退（F:29-32、P:489-492、R(archive):150-160）。
- 写入顺序 = 名字的**插入顺序**；第 i 个名字对应块 i（P:851-861）。

### 2.2 classic block table（`dwBlockTableSize` × 16 字节）

| 偏移 | 宽度 | 字段 | 语义 |
| --- | --- | --- | --- |
| 0 | 4 | `dwFilePos` | 载荷绝对偏移 |
| 4 | 4 | `dwCmpSize` | 磁盘长度（**含扇区偏移表本身**） |
| 8 | 4 | `dwFileSize` | 解压后长度 |
| 12 | 4 | `dwFlags` | 见下 |
标志位（P:38-46）：`0x00000100 IMPLODE`、`0x00000200 COMPRESS`、`0x00010000 ENCRYPTED`、`0x00020000 FIX_KEY`、
`0x00100000 PATCH`、`0x01000000 SINGLE_UNIT`、`0x02000000 DELETE_MARKER`、`0x04000000 SECTOR_CRC`、`0x80000000 EXISTS`。

实测 paiur01 只出现两种组合：**`0x81000200`**（EXISTS|SINGLE_UNIT|COMPRESS，65 块）与
**`0x84000200`**（EXISTS|SECTOR_CRC|COMPRESS，2 块）。**SC2 文档里没有加密块、没有 FIX_KEY、没有 IMPLODE**
（本次实测：ENCRYPTED=0、FIX_KEY=0）。

- 加密密钥 = `hash_string("(block table)", HASH_FILE_KEY=3)`（P:507、P:497）。
- **自洽不变量**：非空闲项 `file_pos + cmp_size <= archiveSize`；不满足就回退未解密解释（P:498-501）。
- 空组件：`file_size = 0`、`cmp_size = 0`、`flags = EXISTS|SINGLE_UNIT`，且 `file_pos` **不推进**（P:900-901、R(archive):370-372）。

### 2.3 HET（`HET\x1a`，位置/大小来自 0x3C / 0x5C）
    12 字节明文 + 加密体
    明文   = "HET\x1a"(4) + version=1(4) + bodySize(4)          # bodySize = 加密体长度
    加密体 = 8 个 u32 头 + nameHashes[totalCount] + indexBits[indexTableSize]
    密钥   = hash_string("(hash table)", HASH_FILE_KEY)

8 个 u32 头（P:295-298）：`tableSize, entryCount, totalCount, nameHashBitSize, indexSizeTotal, indexSizeExtra, indexSize, indexTableSize`。

| 字段 | paiur01 | HTXL | 不变量公式 |
| --- | --- | --- | --- |
| tableSize | 199 | 522 | `32 + totalCount + indexTableSize` |
| entryCount | 67 | 184 | = 块数 |
| totalCount | 89 | 245 | `(entryCount * 4) // 3`（`entryCount=17` 时为 22，不成立 **未验证**） |
| nameHashBitSize | 64 | 64 | 固定 64 |
| indexSizeTotal | 7 | 8 | `necessary_bit_count(entryCount)` |
| indexSizeExtra / indexSize | 0 / 7 | 0 / 8 | `indexSize = indexSizeTotal` |
| indexTableSize | 78 | 245 | `((totalCount * indexSizeTotal) + 7) // 8` |
paiur01 自洽：`32 + 89 + 78 = 199`；`((7*89)+7)//8 = 78`；`(67*4)//3 = 89`（F:52）。

- `nameHashes[i]` 是**名字哈希的高 8 位**；`0x00` = 空闲（`HET_ENTRY_FREE`，P:57）。
- `indexBits` 初始全 `0xFF`，把第 i 个名字的块索引写到比特位 `indexSizeTotal * slot`（P:791）。
- 探针起点 `FileNameHash % totalCount`，线性探测，遇空闲槽或回到起点结束（P:540-558，与 S:1619-1677 一致）。
- **命中后必须复核**（见 §7.2）。

### 2.4 BET（`BET\x1a`，位置/大小来自 0x34 / 0x64）
    12 字节明文 + 加密体
    加密体 = 19 个 u32 头 + flags[flagCount] + fileBits + nameHash2Bits
    密钥   = hash_string("(block table)", HASH_FILE_KEY)      # 注意：不是 (hash table)

19 个 u32 头的**下标是稳定契约**（P:356-363；R(tables):12-30 已列常量）：

| # | 名称 | paiur01 | HTXL | 说明 |
| --- | --- | --- | --- | --- |
| 0 | `tableSize` | 1115 | 2936 | 整个 BET 长度（含 12 字节明文头） |
| 1 | `entryCount` | 67 | 184 | = 块数 |
| 2 | **`unknown08`** | **7888** | **7888** | 5 样本全是 7888；StormLib 写 `0x10`。**保留原值**（F:55-63） |
| 3 | `tableEntrySize` | 67 | 68 | 每条目的位宽 = 各 `bitIndex_*` + `bitCount_*` 之和 |
| 4-8 / 9-13 | `bitIndex_*` / `bitCount_*`（各 5 个） | 0/22/46/66/68 / 22/24/20/2/0 | 同左 | `bitCount_x = necessary_bit_count(max(x))`；`bitIndex` 是各字段在条目内的位偏移 |
| 14 | `bitTotal_NameHash2` | 56 | 56 | `nameHashBitSize - 8` |
| 15 | `bitExtra_NameHash2` | 0 | 0 | 0 |
| 16 | `bitCount_NameHash2` | 56 | 56 | = bitTotal |
| 17 | `nameHashArraySize` | 469 | 1288 | `((bitTotal * entryCount) + 7) // 8` |
| 18 | `flagCount` | 2 | 2 | 去重后的 flags 个数 |
- 布局顺序实测：`head(76B) → flags(flagCount*4) → fileBits → nameHash2Bits`（P:422-427）。
- 自洽（paiur01）：`76 + 2*4 + ((67*67)+7)//8 + 469 = 1115` = `tableSize`（F:57）。HTXL：`76+8+((68*184)+7)//8+1288 = 2936` ✔
- `fileBits` 每条目起点 `i * tableEntrySize`，字段按 `bitIndex_*` 位偏移、`bitCount_*` 位宽读（P:373-380）。
- `nameHash2Bits` 每条目起点 `bitTotal_NameHash2 * i`，存**名字哈希的低 56 位**（P:837-838）。
- flags 数组按**块索引首次出现顺序**去重收集（P:803-809）。
- **flags 在磁盘上是小端**（F:59-61；实测原始字节 `00020081 00020084` → LE → `0x81000200`/`0x84000200`）。

### 2.5 `(attributes)`（version 100, flags 0x5）
    版本(4)=100, flags(4)=ATTR_CRC32|ATTR_MD5=0x05
    crc32[blockCount]      (u32 × blockCount)     # 仅当 flags&1
    filetime[blockCount]   (u64 × blockCount)     # 仅当 flags&2（SC2 未用）
    md5[blockCount]        (16B × blockCount)     # 仅当 flags&4

- **数组按块表索引排列**（F:71-73）；paiur01：`8 + 4*67 + 16*67 = 1348` = 实测长度 ✔；MD5 作用于**解压后**内容（本次实测 65/67 块通过，含 2 个 SECTOR_CRC 块）。
- **簿记项特殊约定（5 样本全部实测通过）**：

| 块 | CRC32 | MD5 |
| --- | --- | --- |
| `(listfile)` | **正确值**（paiur01 `0xc115ed63`，与 `zlib.crc32(listfile)` 相等） | **全零 16 字节** |
| `(attributes)` | **0** | **全零 16 字节** |
| 其余块 | 正确 | 正确 |
  实测计数：paiur01 md5_ok=65 / bad=2（恰为这两项）；crc_ok=66 / bad=1；HTXL 182/2、183/1（F:80-81，本次复现一致）。

- 由此可推出：**MD5 全零代表「跳过校验」**，不是「内容为空」——读取器不能把全零当校验失败。

### 2.6 `(listfile)`
- 纯文本、`\r\n` 分隔、UTF-8、**无 BOM**（P:942 `"\r\n".join(names) + "\r\n"`；实测 paiur01 首行 `BankList.xml\r\n…`）。
- 文件名用 `` 作分隔符。
- paiur01 列出 65 个名字而 `block_count=67`：差的 2 个就是 `(listfile)`/`(attributes)` —— **listfile 不列自己**。

## 3. 头部摘要块：6 个 MD5
计算顺序（P:1005-1010，R(archive):516-521）：

| 偏移 | 覆盖字节 | 计算时机 |
| --- | --- | --- |
| 0x70 | `md5(blockTable 加密后密文)` = `raw[block_pos .. block_pos + count*16]` | 先算 |
| 0x80 | `md5(hashTable 密文)` | 先算 |
| 0x90 | 全零（无 hi-block 表） | 常量 |
| 0xA0 | `md5(BET 全文，含 12 字节明文头)` | 先算 |
| 0xB0 | `md5(HET 全文，含 12 字节明文头)` | 先算 |
| 0xC0 | `md5(header[0x00..0xC0])` | **最后算**（自哈希，覆盖前 5 个摘要 + `dwRawChunkSize`） |
要点：

1. **覆盖的是磁盘上的原始字节**（密文 / 含明文头），不是解密后的明文表（P:1005-1009）。
2. 0xC0 是自哈希：先写满 0x00..0xBF，再算 0xC0..0xCF；**改了 `dwRawChunkSize` 必须重算它**。
3. StormLib 对这两条是**硬性**的（S:702-711，`MPQ_FORMAT_VERSION_4` 分支）：

       Apparently, Starcraft II only accepts MPQ headers where the MPQ header hash matches
       If MD5 doesn't match, we ignore this offset. We also ignore it if there's no MD5 at all
       → return ERROR_FAKE_MPQ_HEADER (10009)

   即「**不写**摘要块」与「**写错**摘要块」后果相同（F:208-220）。补齐前 StormLib 对重建件返回 `10009`，补齐后 OK（F:231-236）。
   **[推断/未验证]** SC2 头的 `formatVersion=3`，而 StormLib 只在 v4 分支做这个校验——SC2 编辑器显然自行按「头部 208 字节」判断。这条要求对我们是实测硬约束，但**不要**用 StormLib 的 v3 代码路径去解释它。

## 4. 加密

### 4.1 `hash_string(text, hash_type)`（P:85-91）
    seed1, seed2 = 0x7FED7FED, 0xEEEEEEEE
    for byte in text.upper().encode('latin-1'):     # 先转大写，再逐字节
        seed1 = CRYPT[(hash_type << 8) + byte] ^ (seed1 + seed2)
        seed2 = byte + seed1 + seed2 + (seed2 << 5) + 3
    return seed1

- `CRYPT` 是 0x500 项、`seed = (seed*125+3) % 0x2AAAAB` 生成的表（P:65-80）。
- 索引 `(hash_type << 8) + byte` 恒 `< 0x500`，**不要**再 `& 0x4FF`：那会清掉 bit 8，让所有 `hash_type != 0` 塌缩成 0（R(crypto):36-38 明确注释过这个坑）。
- 大小写：`text.upper()`；名字用 `` 分隔，算哈希时**整串参与**。

### 4.2 块密码 `encrypt` / `decrypt`（P:94-122）
    seed1 = key; seed2 = 0xEEEEEEEE
    for 每个整 4 字节字:
        seed2 += CRYPT[0x400 + (seed1 & 0xFF)]
        plain/cipher 异或 (seed1 + seed2)
        seed1 = ((~seed1 << 0x15) + 0x11111111) | (seed1 >> 0x0B)
        seed2 = 明文 + seed2 + (seed2 << 5) + 3        # ← 用【明文】推进
    尾部 len % 4 个字节【原样保留】

三个必须记住的点：

1. **不是自逆的**：`decrypt` 用密文推进 `seed2`，`encrypt` 用明文推进（P:104 vs P:119）。用「解密」实现「加密」会写坏。
2. **尾部 1-3 字节不参与加密，原样搬运**（P:106、P:121）。丢掉尾部会截断 HET：paiur01 的 HET `dataSize=199`，`199 % 4 = 3`（F:65-68）。
3. `seed1` 的 `>> 0x0B` 是**逻辑右移**，左移前先按位取反。

### 4.3 file key 推导（P:576-581）
    base = name.replace('/', '\\').split('\\')[-1]   # 只取文件名部分（去目录）
    key  = hash_string(base, HASH_FILE_KEY=3)
    if flags & FLAG_FIX_KEY: key = (key + file_pos) & 0xFFFFFFFF      # 明文前缀调整

- **必须先去目录**：`Base.SC2Data\\GameData\\UnitData.xml` → 用 `UnitData.xml` 算 key。
- `FLAG_FIX_KEY` 时加的是 `dwFilePos`（不是 `dwFileSize`）。
- §1.2 实测：SC2 文档**没有** ENCRYPTED / FIX_KEY 块，所以这条路走不到；但读别人的 MPQ 时会用到，实现要留着。
- 扇区的偏移表本身用 `key - 1` 加密（S:1322）—— SC2 场景用不到。

## 5. 压缩

### 5.1 掩码表与识别（P:716-733）

| 掩码 | 含义 | 处理 |
| --- | --- | --- |
| `0x00` | 无压缩 | 返回 `chunk[1:]` |
| `0x02` | zlib | `zlib.decompress(payload)` |
| `0x08` | PKWARE implode | **P 不支持**（会抛异常）；`FLAG_IMPLODE=0x100` 走 `_decompress_chunk` |
| `0x10` | bzip2 | `bz2.decompress(payload)` |
| `0x12` | LZMA | `lzma.decompress(payload)` |
| `0x20` | sparse | 原样返回 payload（P:731-732） |
| 其他组合掩码 | 多个位可并存 | 判别顺序：`zlib → bzip2 → lzma → sparse`（按位测试，`0x12` 会被 zlib 先截住 → §9 坑表） |
实测 paiur01（`cmp != size` 的块）首字节分布：`0x10`×49、`0x02`×1、`0x08`×1（`0x08` 出现在 SECTOR_CRC 块里）。

### 5.2 「`cmp == size` 代表原样存储」这个坑
- 真实文档里存在 **flags 带 `COMPRESS` 但数据原样存储**的块，判据是 `cmp_size == file_size`（F:83-88）。paiur01 有 **16 个**这样的块。
- 它们的**首字节是数据本身**，不是掩码：`DocumentInfo.version` 首字节 `0x63`（`cdes…`）、`CustomAI` 首字节 `0x3c`（`<?xml…`）、`(attributes)` 首字节 `0x64`、`t3HardTile` `0x48`、`t3FluffDoodad` `0x44`。
- **读取器必须先用 `len(chunk) == expected` 判定「原样」，再去看首字节掩码**（P:717-718、R(compress):11-13）。反过来做会把数据的首字节吃掉。
- 反方向的陷阱：**writer 若为「压不动」的块清除 `COMPRESS` 标志，就与暴雪产物不一致**（F:277-279）。P 的 `_best_compress` 就是「更小才压，否则返回原数据 + `compressed=False`」（P:745-753）。**要复现暴雪行为，应保留标志位、仍靠 `cmp==size` 表示原样。**

### 5.3 单块解压 `_decompress_chunk(chunk, expected)`
    if len(chunk) == expected: return chunk          # 原样存储（必须最先判定）
    if not chunk: return b""
    mask = chunk[0]; payload = chunk[1:]
    ...

### 5.4 SINGLE_UNIT 与分扇区

| 情况 | 载荷布局 | 读法 |
| --- | --- | --- |
| `FLAG_IMPLODE` | 单块 | `_decompress_chunk(data, file_size)` |
| `FLAG_SINGLE_UNIT` | 单块（可能原样） | `len == file_size ? data : _decompress_chunk(data, file_size)`（P:612-613） |
| 其余 | 偏移表 + 扇区 | 见下 |
- `sectors = ceil(file_size / block_size)`（P:614）。
- `sectors = ceil(file_size / block_size)`（P:614）。非 `SECTOR_CRC`：偏移表是 **n+1** 个 u32，第 i 扇区 = `data[offsets[i] .. offsets[i+1]]`，期望长度 `min(block_size, file_size - 已产出)`（P:615-620）。
- **`SECTOR_CRC`：偏移表是 n+2 个 dword**（F:98-99；本次实测确认）：

| 事实 | paiur01 `DocumentHeader`（n=2） | paiur01 `t3TextureMasks`（n=576） |
| --- | --- | --- |
| 偏移个数 | 4 | 578 |
| `offsets[0]` | **16 = 4*(n+2)** | **2312 = 4*(n+2)** |
| `offsets[n]` | 7547 | 744777 |
| `offsets[n+1]` | 7547 (== `cmp_size`) | 744777 (== `cmp_size`) |
| 第 0 扇区区间 | `[16, 6715)` | `[2312, 2381)` |
  即：多出来的一个 dword 位于**表尾**，值恒等于 `cmp_size`；**扇区数据的解释方式与 n+1 情形完全相同**，只要按 `offsets[0]` 起算。多出这个尾项使得 `offsets[n+1] == cmp_size` 恰好成立，`cmp_size % 4 == 0` 也恒成立（实测所有 SECTOR_CRC 块均满足）。

- **[未验证]** `offsets[n+1]` 的语义（末扇区 CRC？纯填充/对界值？）：本次实测它与末扇区、与任何扇区的 CRC32 **都不相等**
  （`DocumentHeader` 末扇区 CRC `0xadaa8c3f` ≠ `7547`），所以**不要**把它当校验值用。
  StormLib 只是「多读 4 字节」并把它算进 `dwCmpSize`（S:1262-1266、S:1363-1369）。

- **[实测结论] `SECTOR_CRC` 不是必须复现的**：把全部块改成 SINGLE_UNIT 的变体仍被 StormLib 接受、组件 67/67 一致，编辑器仍然 CRASH（F:304-317），说明它与「编辑器拒收」无关。P 的写入器**从不产出** `SECTOR_CRC`（P:889-893），但会**保留分扇区**（依据 `hints`）。
- **[推断] 现存两个读取器在这条上都是「侥幸正确」**：P:614-620 与 R(archive):254-263 都只读 n+1 个偏移。对实测样本，`offsets[0]` 恰好是数据起点，所以第 0 扇区读对了；但 `offsets[i]`（i≥1）就是 CRC 值，它们靠 `len(chunk) == expected` 分支兜住才没炸。**新实现请显式处理 `SECTOR_CRC`：读 n+2 个偏移、按上述区间取值。**

## 6. 写入流程

### 6.1 步骤顺序（P:926-1020 `build_document`）
    0. 分离用户组件 / 簿记项；算 block_size = 512 << block_shift
    1. 定 (listfile)：调用方给了就用给的（组件集未变时用原件的 → 可逐字节复现）
       否则 = "\r\n".join(用户组件名) + "\r\n"
    2. 【pass 1】先用一个「占位 (attributes)」打包一次，只为拿到块几何 → 算 attributes
    3. 【pass 2】真打包 → 用真几何重算 attributes → 重打包；循环最多 4 次直到 attributes 收敛
       （attributes 的字节长度影响它自己的块位置与长度，所以必须迭代到不动点）
    4. hash_count = 2 的幂 >= max(4, 条目数)
    5. build_het(names) → het_blob = "HET\x1a" + ver(1) + bodyLen + encrypt(body, key_hash)
       build_bet(blocks, names, unknown08) → bet_blob
    6. 定位置：het_pos  = 208 + |body|；       bet_pos  = het_pos + |het|
              hash_pos = bet_pos + |bet|；      block_pos = hash_pos + |hashTable|
    7. build_hash_table(names, hash_count) → 加密 → hash_blob
       block_table（由 pack 产出）        → 加密 → block_blob
    8. archive_size = block_pos + |block_blob|
    9. 写头 0x00..0x6F（含 0x6C = rawChunkSize = 0）
    10. 算 0x70/0x80/0x90/0xA0/0xB0 五个摘要 → 再算 0xC0 自哈希
    11. 拼接落盘：header + body + het + bet + hash + block

**自引用 / 顺序敏感点**：

| 项 | 依赖 | 后果 |
| --- | --- | --- |
| `(attributes)` 的字节 | 所有块的 `file_pos/cmp_size`（含它自己） | 必须迭代到不动点（≤4 轮，P:954-960） |
| `het_pos`/`bet_pos`/`hash_pos`/`block_pos` | `\|body\|`、`\|het\|`、`\|bet\|` 的长度 | 必须按 §6.1 的顺序串行计算 |
| `0xC0` 自哈希 | 头部前 0xC0 字节 | **最后算** |
| HET 槽位排布 | **名字的插入顺序** | 顺序变则 HET 字节全变（F:280-282）——想逐字节复现就必须保序 |
| BET flags 数组 | 块遍历顺序下 flags 的首次出现顺序 | 同上 |

### 6.2 压缩选择策略

| 策略 | 依据 |
| --- | --- |
| 多兆字节组件用 **SINGLE_UNIT** | 暴雪自己就这么干：Triggers 2.5MB → 145291 字节一整块（F:94-96） |
| 不要主动按 16 KiB 分扇区 | 实测 Triggers 分扇区 264636 vs 一整块 145291，**归档 +21%**（F:96） |
| 只在**原件就是分扇区**时才分扇区 | P 的 `hints`：`hints[name] = 原块 flags`，`sectorised = hint 存在 and !(hint & SINGLE_UNIT) and len > block_size`（P:884-899） |
| **bz2 level 9** | 与暴雪产物**逐字节相等**：Triggers 145291、Objects 63577、MapScript.galaxy 23046（F:89-90） |
| zlib 作为候选 | 取 bz2/zlib 中较小者（P:745-753） |
| 压不动就原样 | `len(best) < len(data)` 才压（P:751-753） |

### 6.3 从「逻辑组件字典」到磁盘（调用方视角）
    Document.open(src)                      # 读入全部组件（含簿记项，保序）
    doc.set(name, bytes) / doc.remove(name) # 增量替换
    doc.save(dest)                          # 自动沿用：hash_count / block_shift / unknown08 /
                                            #           attr_flags / 每块 flags(hints)
                                            #           名字集合未变时沿用原件 (listfile)

**关键**：`Document.__init__` 把原件的 `hash_count`、`block_shift`、`bet.unknown08`、`attributes.flags`、每块 flags 全记下来（P:1043-1053），`save` 时原样回填（P:1087-1093）。**这是「重建件能被编辑器接受」的必要条件之一**，尤其是 `unknown08`（7888）。

## 7. 读取流程

### 7.1 打开序列（P:453-476 / R(archive):73-127）
1. 读整文件；校验 `magic`。
2. 解 0x04..0x20 的基础字段 → `block_size = 512 << block_shift`。
3. `headerSize >= 68` → 读 0x34/0x3C（BET/HET 位置）；`>= 208` → 读 0x5C/0x64（两者大小）。
4. 解 hash table（密钥 `(hash table)`）→ 按不变量校验，不通过就退回**未解密**的原始解释（P:489-492）。
5. 解 block table（密钥 `(block table)`）→ 校验 `file_pos + cmp_size <= 文件长度`，不通过同样回退（P:498-501）。
6. 读 HET/BET：`size64 != 0` 就用它，否则从 12 字节明文头里读 `dataSize`（P:479-483 `_ext_table`）。
7. 读 `(attributes)` 并按 `block_count` 解析（P:474-476）。

### 7.2 查找顺序：HET 优先，且必须用 BET 复核
    若没有 HET                  → classic hash table（find_block）
    若有 HET 但 entryCount == 0 → 直接未命中
    否则：
        want      = jenkins_name_hash(name, het.nameHashBitSize)
        nameHash1 = (want >> (bitSize - 8)) & 0xFF
        index = start = want % het.totalCount
        while het.nameHashes[index] != FREE:
            if het.nameHashes[index] == nameHash1:
                cand = bits_get(het.indexBits, indexSizeTotal*index, indexSize)
                if cand < blockCount:
                    got = bits_get(bet.nameHashBits, bitTotal_NameHash2*cand, bitCount_NameHash2)
                    if (want & ((1<<bitCount_NameHash2)-1)) == got: return cand
            index = (index + 1) % totalCount
            if index == start: break
        return FREE

证据：P:523-558、R(archive):201-233、S:1619-1677。

三条硬性要求：

1. **HET 只存高 8 位哈希**，命中只是「候选」；**不能命中即返回**。
2. **复核用的是 BET 的 64 位名字哈希**（StormLib 用的是文件表里的 `FileNameHash`，S:1661；该值正是从 BET 组合出来的，S:2702-2718）。
3. **复核不匹配要「继续探测」**，不是直接失败（F:41-43、P:531 注释）。按「命中即返回」实现会漏文件——实测 HTXL 有 3 个、UmojanCT 有 1 个这样的名字（F:43）。

无 HET 时回退 classic hash table（P:532-533）。

### 7.3 listfile 缺失时的枚举能力
P 的 `names()` **只**读 `(listfile)`（P:560-564），`block_name(i)` 先在 listfile 里线性找、再兜底四个簿记项名（P:566-573）；`extract_all` 对「有块但没名字」的块直接跳过（P:628-630）。
所以**没有 listfile 就枚举不出名字**：BET/HET 只存哈希（§2.3/§2.4）。
- **[建议]** 新实现先把 listfile 读进来建立 `block_index -> name` 反查表（R 的做法，R(archive):118-125），并**保留未命名块的可读通道**（按块索引读原始字节）——这是 P 目前缺的能力。

## 8. Rust API 设计建议（`sc2mpq` crate）
每张表/每个结构体都要能 `parse`/`to_bytes` 双向走通（HET/BET 的逐字节重生成就靠它）；下表右列 = Python 对应函数。

### 8.1 模块划分
```
    sc2mpq/ = lib.rs(标志/常量/MpqError) | crypto.rs | tables/{hash,block,het,bet,attributes}.rs |
              archive.rs(Archive 只读 + Document 读写) | compress.rs | layout.rs | build.rs
```

### 8.2 公开 API 草案

```rust
pub enum MpqError { NotAnArchive, HashTableFull, HetTableFull, BadCompressionMask(u8),
    TooShort { need: usize, got: usize }, MissingComponent(String), Io(std::io::Error) }
pub type Result<T> = std::result::Result<T, MpqError>;
pub struct Archive { /* 头字段 + 四张表 + raw */ }
impl Archive {  // 只读侧
    fn open(path: &Path) -> Result<Self>;                        // P: MPQArchive.__init__ (453)
    fn parse(raw: Vec<u8>) -> Result<Self>;                      //   内存内解析（便于测试）
    fn names(&self) -> Vec<String>;                              // P: names() (560)
    fn block_name(&self, i: u32) -> Option<&str>;                // P: block_name() (566)
    fn find_block(&self, name: &str) -> Option<u32>;             // P: find_block() (510)   经典哈希表
    fn lookup_index(&self, name: &str) -> Option<u32>;           // P: lookup_index() (523) HET+BET 复核
    fn component_names(&self) -> Vec<String>;                    //   排除簿记项
    fn read(&self, name: &str) -> Result<Vec<u8>>;               // P: read() (603)
    fn try_read(&self, name: &str) -> Option<Vec<u8>>;           // P: try_read() (594)
    fn read_block(&self, i: u32, name: &str) -> Result<Vec<u8>>; // P: read_block() (583)
    fn extract_all(&self, dest: &Path) -> Result<Vec<String>>;   // P: extract_all() (622)
    fn verify(&self, deep: bool) -> VerifyReport;                // P: verify() (647)
}
pub struct VerifyReport {  // 比 R 的 5 个 bool 强得多，定位问题必备
    pub header_md5_valid: bool, pub block_table_md5_valid: bool, pub hash_table_md5_valid: bool,
    pub het_md5_valid: bool, pub bet_md5_valid: bool, pub raw_chunk_size: u32,  // 必须 == 0
    pub het_roundtrip_exact: bool, pub bet_roundtrip_exact: bool,
    pub het_vs_classic_mismatch: Vec<String>,
    pub bet_vs_blocktable_mismatch: Vec<u32>, pub bet_flag_mismatch: Vec<u32>,
    pub md5_ok: usize, pub md5_bad: usize, pub crc_ok: usize, pub crc_bad: usize,
    pub md5_exceptions: Vec<String>,       // 恒为 [(listfile), (attributes)]
}
pub struct Document { /* entries: Vec<(String, Vec<u8>)> 保序 + Layout 快照 */ }
impl Document {  // 读写侧
    fn open(path: &Path) -> Result<Self>;                        // P: Document.open (1055)
    fn new() -> Self;                                            //   从零构建
    fn get(&self, name: &str) -> Option<&[u8]>;                  // P: get() (1059)
    fn set(&mut self, name: &str, data: Vec<u8>);                // P: set() (1065)
    fn remove(&mut self, name: &str) -> bool;                    // P: remove() (1077)
    fn component_names(&self) -> Vec<String>;                    // P: component_names() (1084)
    fn save(&self, dest: &Path) -> Result<BuildInfo>;            // P: Document.save (1087)
    fn save_with(&self, dest: &Path, opts: &BuildOptions) -> Result<BuildInfo>;
}
/// 「重建件能否被编辑器接受」的关键：P:1043-1053 记录、P:1087-1093 回填
pub struct Layout { pub hash_count: u32, pub block_shift: u16, pub unknown08: u32,
    pub attr_flags: u32, pub listfile: Option<Vec<u8>>, pub per_block_flags: HashMap<String, u32> }
pub struct BuildOptions {  // P:926-928；raw_chunk_size 默认必须 0（§1.1）
    pub hash_count: Option<u32>, pub block_shift: u16, pub unknown08: u32, pub attr_flags: u32, pub listfile: Option<Vec<u8>>, pub hints: HashMap<String, u32>, pub raw_chunk_size: u32 }
pub struct BuildInfo { pub bytes: usize, pub entries: usize, pub blocks: usize, pub hash_entries: u32,
    pub het_bytes: usize, pub bet_bytes: usize, pub block_size: u32 }   // P:1018-1020
fn build_document(entries: &[(String, Vec<u8>)], dest: &Path, opts: &BuildOptions)
    -> Result<BuildInfo>;                                    // P: build_document (926)
// 给单测与查障用
fn decompress_chunk(chunk: &[u8], expected: usize) -> Result<Vec<u8>>;  // P: _decompress_chunk (716)
fn best_compress(data: &[u8]) -> Result<(Vec<u8>, bool)>;              // P: _best_compress (745)
fn hash_string(text: &str, hash_type: u32) -> u32;                      // P: hash_string (85)
fn jenkins_name_hash(name: &str, bits: u32) -> u64;                     // P: jenkins_name_hash (200)
```

### 8.3 支撑能力检查

| 需要的能力 | 用哪个 API |
| --- | --- |
| 打开 | `Archive::open` / `Document::open` |
| 枚举 | `Archive::names` / `component_names` / `block_name` |
| 读单个组件 | `Archive::read` / `read_block` |
| 全量解包 | `Archive::extract_all` |
| 从组件字典从零构建 | `Document::new` + `set`×N + `save`（`hash_count: None` 自动取 2 的幂） |
| 增量替换若干组件 | `Document::open` → `set`/`remove` → `save`（`Layout` 快照自动沿用） |
| verify | `Archive::verify(true)`，断言 `raw_chunk_size == 0` |

### 8.4 必须写进单测的不变量
① `het.to_bytes()`/`bet.to_bytes()` 与磁盘原文逐字节相等（F:116-117）；② `raw_chunk_size == 0` 且 6 个 MD5 有效（P:992-1010）；
③ 所有名字 `find_block(n) == lookup_index(n)`（F:108-114）；④ `bet.entry(i)` 的 `(pos,size,cmp)` 与 block table 逐条相等（P:677-685）；
⑤ `cmp==size` 的块读取后长度 == `file_size` 且首字节未被吞（§5.2）。

## 9. 坑清单（照抄 StormLib / 规范就会踩）

| # | 坑 | ❌ 错误做法 | ✅ 正确做法 | 证据 |
| --- | --- | --- | --- | --- |
| 1 | 头部版本判定 | 按 `formatVersion` 选布局 | 按 `headerSize`（`>=68`/`>=208`）决定字段存在 | P:459-466；F:26 |
| 2 | `dwRawChunkSize` | 从原件继承 16384 | **写 0** 并重算 0xC0 | P:992-998；F:355-365 |
| 3 | 头摘要块 | 省略 / 先算 0xC0 | 5 个先算，0xC0 = `md5(header[..0xC0])` 最后算 | P:1005-1010；S:704-711 |
| 4 | BET flags 字节序 / `unknown08` | 按 StormLib 做大端 / 写 `0x10` | **小端**；`unknown08` **保留原件值（样本恒 7888）** | F:59-63；P:364-365、P:1050 |
| 5 | HET 只存高 8 位 | 命中即返回 | 用 BET 的 64 位哈希复核，不匹配**继续探测** | F:41-43；P:523-558；S:1647-1673 |
| 6 | 块密码尾部 / 自逆假设 | 丢掉 `len % 4` 的 1-3 字节；用 `decrypt` 当 `encrypt` | 尾部原样搬运（HET `199 % 4 = 3`）；`seed2` 用**明文**推进的是加密 | P:106、P:119；F:65-68 |
| 7 | hash 表索引掩码 | `CRYPT[(t<<8)+b & 0x4FF]` | 不要掩码，`(t<<8)+b` 恒 `<0x500` | R(crypto):36-38 |
| 8 | `cmp == size` | 先看首字节当掩码 | **先判 `len == expected`**；`DocumentInfo.version` 首字节 `0x63` 是数据 | F:83-88；P:717-718 |
| 9 | COMPRESS 标志 | 压不动就清 `COMPRESS` | 暴雪**保留标志**、用 `cmp==size` 表示原样 | F:277-279 |
| 10 | 字节序三连 | 用宿主序 / MSB-first 位数组 / 把签名+版本+长度也加密 | 全部小端（`<`）；**LSB-first**（第 k 位 = 字节 `k/8` 的 `k%8` 位）；前 12 字节明文、其余整段加密 | P:296、P:226-242、P:300-303 |
| 11 | BET 密钥 | 用 `(hash table)` | BET 用 **`(block table)`**；HET 用 `(hash table)` | P:370、P:302 |
| 12 | 表密文解密失败 | 直接抛错 | 按不变量校验后**回退到未解密解释**再判定 | P:489-501 |
| 13 | `hash_count` 取模 | 直接 `%` | 用 `& (count-1)`，即**必须是 2 的幂** | P:511 |
| 14 | attributes 排版 | 按名字/任意顺序 | **严格按块表索引**排列 | F:71-73 |
| 15 | `(listfile)` 的 MD5 | 写真实 MD5 | **全零 16 字节**（CRC32 却要真实值） | F:74-78；本次实测 |
| 16 | `(attributes)` 自身 | 写真实 CRC32/MD5 | **CRC32 = 0、MD5 全零** | F:78；P:875-880 |
| 17 | file key 取整路径 | 用完整路径算 key | 只取 `\` 后的**文件名部分** | P:577 |
| 18 | FIX_KEY 调整量 | 加 `file_size` | 加 **`file_pos`** | P:579-580 |
| 19 | 多兆字节组件分扇区 | 一律按 16 KiB 切 | 用 **SINGLE_UNIT**（否则 +21% 体积） | F:94-96 |
| 20 | 分扇区偏移表 | 一律 n+1 个 dword | `SECTOR_CRC` 时是 **n+2**，`offsets[0] == 4*(n+1)+4`，尾项 == `cmp_size` | F:98-99；本次实测 |
| 21 | 掩码判别 / listfile 缺失 | `0x12` 当 LZMA；以为能靠 HET/BET 枚举 | 按位测试时 **zlib(0x02) 先命中**；HET/BET **只存哈希**，无名字枚举不出来 | P:725-731、P:560-573 |
| 22 | 空组件 | 写 `cmp_size != 0` | `file_size = cmp_size = 0`、`SINGLE_UNIT`，且**不推进 body 偏移** | P:900-901 |
| 23 | 只信自家读取器 | round-trip 通过就认为对 | 必须过 **StormLib**（`SFileOpenArchive`）与**编辑器本体** | F:262-263、F:378-383 |
| 24 | HET 槽位顺序 | 按块索引顺序插 | 按**名字列表顺序**插槽；顺序变则 HET 字节全变 | P:781-791；F:280-282 |
| 25 | debug 下算 jenkins | 未用 wrapping 移位 | `(t[i] as u32) << 24` 在 debug 会 panic（§10.1） | R(crypto):137 |

## 10. SC2Diff 的 Rust 移植评估

### 10.1 编译/测试状态（本次实测）

| 项目 | 结果 |
| --- | --- |
| `cargo check --offline --all-targets` | **通过**（rustc 1.94.0） |
| `cargo test --offline --lib` | **3 passed / 3 FAILED** |
| 失败原因（三个用例同一根因） | `src/mpq/crypto.rs:137:58: attempt to shift left with overflow`（`round_trips_byte_exactly` / `digest_block_and_raw_chunk_size` / `repository_round_trip_and_pack`） |
根因（R(crypto):137 的 `add` 闭包）：

    fn add(t: &[u8], i: usize) -> u32 { if i < t.len() { (t[i] as u32) << (8 * i) } else { 0 } }

`t[3] = 0xFF` 时 `0xFFu32 << 24` 溢出 → debug 构建 panic。Python 用任意精度整数天然不溢出（P:179）。
**修复**：`(t[i] as u32).wrapping_shl(8 * i as u32)`，或配合 `wrapping_mul`。
**含义**：这个移植**从未在 debug 下跑通过**——它的可信度低于 Python 原型。

### 10.2 逐文件评估

| 文件 | 行数 | 覆盖了什么 | 缺什么 | 值不值得搬 |
| --- | --- | --- | --- | --- |
| `crypto.rs` | 191 | `crypt_table`/`hash_string`/`encrypt`/`decrypt`/`hashlittle2`/`jenkins_name_hash`/`necessary_bit_count`/`bits_get`/`bits_set` 全有；`hash_string` 掩码坑有注释 | `hashlittle2` 移位溢出（致命，测试全挂）；`jenkins_name_hash` 用 `to_lowercase()`（Unicode）而 P 用 `latin-1`+`lower()`，非 ASCII 名字分歧 | **搬，但先修 137 行**；与 P 逐行对应，最干净的一部分 |
| `compress.rs` | 66 | mask 0x00/0x02/0x10；zlib/bzip2 解码；`best_compress`（bz2 best vs zlib best，取小，压不动返回原样） | **LZMA(0x12)、SPARSE(0x20)、PKWARE(0x08)、`FLAG_IMPLODE` 全无**；不支持 `SECTOR_CRC` 的 n+2 表 | **只搬骨架**，按 §5.1 补齐；「压不动清标志」与暴雪不符（§5.2） |
| `tables.rs` | 373 | HET/BET/attributes 三者 `parse`/`to_bytes` 对称；`build_het`/`build_bet`/`build_attributes` 齐；BET 小端有注释（4-7 行）；`BET_*_I` 常量表完整 | `candidate_index`（92-116）是**死代码且逻辑错**：把 `if slot == FREE` 放在 `if slot == hash1` **之后**（对比 P:317、S:1647）。实际在用的是 archive.rs 的 `lookup_index`（正确） | **最值得搬**；对称序列化是「逐字节重生成」的基础 |
| `archive.rs` | 589 | 头部解析/写出、hash+block 表解析重建、`find_block`、`lookup_index`（含 BET 复核）、`read_block`、`Document::open`、`build_document`、`pack_entries`、`verify_digests`、2 个单测 | `Document` **没有 `set`/`remove`/`save`**（repo.rs 在外拼 `BuildOptions`）；`block_name` 只来自 listfile；`unknown08` 默认 `0x10`；`verify` 只有 5 个 bool | **搬并补齐**；`read_block` 的 SECTOR_CRC 按 §5.4 改 |
| `mod.rs` | 50 | 常量、`MpqError(String)`、`is_bookkeeping` | 常量是**子集**（缺 IMPLODE/PATCH/DELETE_MARKER/SECTOR_CRC、MASK_LZMA/SPARSE） | 搬，按 §8 扩展；另有 `repo.rs` 作为调用方范例（取配置/取组件/回写） |

### 10.3 与 Python 原型的行为差异

| 差异 | P | R | 影响 |
| --- | --- | --- | --- |
| HET 探针循环 / `verify` 深度 | 先判空闲（P:317）；mismatch 列表 + md5/crc 统计 + roundtrip_exact | `candidate_index` 先判命中（tables.rs:105）；只有 5 个 bool | 前者是死代码，**不要**当参考；后者定位问题能力弱很多 |
| `Document` 编辑 | `set`/`remove` + 快照回填 | 无 | 增量替换必须自己实现 |
| 未命名块 | `block_name` 兜底簿记项名 | 同 | 两者都 skip（P:628-630） |
| `unknown08` 默认 | 参数默认 `0x10`，但 `Document.save` 回填原件值 | 同 | **从零构建（不读原件）会写出 `0x10`，与所有实测样本不同** ← 风险 |
| SECTOR_CRC 偏移 / LZMA / SPARSE | n+1（侥幸正确）；LZMA/SPARSE 支持 | n+1（同）；两者都不支持 LZMA/SPARSE | 两者都没显式处理 n+2；R 读不了 LZMA/SPARSE 块 |

### 10.4 结论
**值得搬运，但不能直接抄**：① `tables.rs` 的对称 `parse`/`to_bytes` 是最有价值的资产，直接搬；
② `crypto.rs` 算法逐行对应 P，但**必须先修 `hashlittle2` 的溢出**；③ `compress.rs` 与 `archive.rs::read_block` 要按 §5 补 LZMA/SPARSE/IMPLODE 与 `SECTOR_CRC`，`archive.rs` 要补 `Document::{set,remove,save}` 与完整 `verify`；
④ **没有任何自动化测试能证明它写过被编辑器接受的归档**（`repo.rs` 的单测本身就是失败的）——新的 `sc2mpq` 必须自建验收：HET/BET 逐字节重生成 + `raw_chunk_size == 0` + StormLib `SFileOpenArchive` + **编辑器本体 LOADED**。

## 附录 A：本次实测数据点

| 结论 | 值 | 与既有材料的关系 |
| --- | --- | --- |
| 5 样本 `dwRawChunkSize` | 全部 16384 | 确认 F:355-361 |
| 5 样本 BET `unknown08` | 全部 **7888** | 确认 F:55-63，且是常量而非个例 |
| HET / BET 参数 | paiur01 HET `199/67/89/64/7/0/7/78`、BET `1115/67/7888/67/…/469/2`；HTXL HET `522/184/245/64/8/0/8/245`、BET `2936/184/7888/68/…/1288/2` | 同 F:51-57 |
| `SECTOR_CRC` 偏移表 | n+2 个 dword;`offsets[0] = 4*(n+2)`;`offsets[n] = offsets[n+1] = cmp_size`;尾项**不是**末扇区 CRC32（`DocumentHeader` 末扇区 CRC `0xadaa8c3f`，尾项 `7547`） | 确认 F:98-99 并补出精确含义;**新否定**尾项语义 |
| `COMPRESS` 且 `cmp==size` 的块数 | paiur01 = **16 个**（含 `(attributes)`） | 量化 F:85-87 |
| `(listfile)` CRC32 正确 / MD5 全零 | `0xc115ed63` = `zlib.crc32(listfile)` | 确认 F:74-78 |

## 附录 B：落地检查清单
- [ ] 头部 0x6C = 0，0xC0 自哈希正确，`headerSize = 208`、`formatVersion = 3`
- [ ] `het.to_bytes()` / `bet.to_bytes()` 与原件**逐字节相同**（对 5 个样本）
- [ ] 所有名字 `find_block(n) == lookup_index(n)`，含 HTXL 的 3 个高 8 位碰撞名字
- [ ] `cmp == size` 的块首字节未被吞
- [ ] `(listfile)` MD5 全零、`(attributes)` CRC/MD5 全零，且**按块索引**排布
- [ ] 写侧 `rawChunkSize == 0`；从原件读入时不继承 16384；`unknown08` 沿用原件（7888）
- [ ] 多兆字节组件走 SINGLE_UNIT；仅在原件分扇区时才分扇区
- [ ] bz2 level 9 输出与暴雪产物逐字节相同（Triggers 145291 / Objects 63577 / MapScript.galaxy 23046）
- [ ] StormLib `SFileOpenArchive` + 全量抽取 67/67
- [ ] **银河编辑器**打开重建件为 LOADED
