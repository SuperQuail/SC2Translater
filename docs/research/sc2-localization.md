# 《星际争霸 II》多语言（本地化）机制 — 实证调研与回填规范

> 目标读者：实现 SC2 地图/模组翻译回填工具的工程师。
> 每条结论都标注 **[实测]** / **[推断]** / **[网上说法]**，并给出可复核的样本路径或命令。
> 调研日期：2026-10-09。调研环境：Windows，SC2 5.0.15/5.0.16 build 97579/98370（网易 CN 分支），
> 编辑器 `D:\Game\BLZ\StarCraft II\Support64\SC2Editor_x64.exe`，系统语言与游戏语言均为 zhCN。
> 本次调研**未修改任何样本文件**；所有临时脚本写在 `%TEMP%`（`sc2probe_*.py`）。

---

## 0. 一句话机制模型

**[实测]** SC2 的本地化是「**键 → 文本**」的查表，键由地图/模组文档自己携带（尤其是触发器参数的
`Param/Value/<8位HEX>`）；游戏在**当前语言**下查这张表，**查不到就把键本身当作文本显示**
（`UIAlertPoint(..., StringExternal("Param/Value/0F5D0539"), ...)` 直接渲染出
`Param/Value/0F5D0539` 这条路径）。**同一文档内不存在 enUS→zhCN 的跨语言回退**：
英文地图只有 `enUS.SC2Data`，中文客户端要的是 `zhCN.SC2Data`，于是整张地图的所有文本都变成键路径。
**回填 = 在归档里补齐 `zhCN.SC2Data\LocalizedData\*.txt`，键必须与源语言逐字一致，值换成中文。**

---

## 1. 证据基线与复现方式

| 代号 | 路径 | 用途 |
| --- | --- | --- |
| **S1** | `D:\Code\Rust\SC2Diff\testdata\samples\paiur01.SC2Map`（1,108,308 B，67 组件） | 暴雪原作战役地图，**12 语言齐全** |
| **S1u** | `D:\Code\Rust\SC2Diff\testdata\bench_paiur01_SC2Map\\` | S1 的解包目录（含 12 个 locale 目录） |
| **S1u2** | `D:\Code\Rust\SC2Diff\testdata\bench2_paiur01_SC2Map\\` | 与 S1u 同源的第二份解包 |
| **S2** | `D:\Code\Rust\SC2Diff\testdata\bench_umojan\\` | 社区模组 UmojanCT，**只有 zhCN**（无 enUS） |
| **S3** | `D:\Code\Rust\SC2Diff\testdata\bench_HTXL_SC2Mod\\` | 社区模组 HTXL，**只有 zhCN** |
| **S4** | `D:\Game\BLZ\StarCraft II\Maps\Test\EditorTest.SC2Map`（78 组件） | **银河编辑器自己保存**的地图（Language = enUS） |
| **S5** | `D:\Code\Python\银河编辑器\StarCraftIIAgent\DataEditorXML\SC2GameDataComponents\\` | 官方**游戏本体数据**的解包快照：12 个战役/mod 的 enUS + zhCN 本地化全量 |
| **S6** | `D:\Game\BLZ\StarCraft II\Mods\kithotsbrutal.SC2Mod` | 真实社区模组：enUS 全量 2.4 MB + zhCN 只有 1 条 |
| **S7** | `D:\Game\BLZ\StarCraft II\Mods\0酒馆虚遗.SC2Mod` | 真实中文模组：zhCN GameStrings/ObjectStrings/TriggerStrings 全中文 |
| **S8** | `D:\Game\BLZ\StarCraft II\Mods\SCMRlocal.SC2Mod` | 社区「本地化补丁包」：**12 个 locale 目录内容完全相同** |
| **S9** | `D:\Code\Rust\SC2Diff\tools\sc2mpq.py` | 只读的 MPQ 解包器（本次用它读取 S1/S4/S6/S7/S8，**未做任何写入**） |
| **L1** | `C:\Users\ROG\Documents\StarCraft II\GameLogs\\*-10-08 23.39.43 Graphics.txt` | 运行时语言证据（`<Locale.Data> zhCN`） |

复现命令（本次实际执行过的形态）：

~~~powershell
# 读取归档内任意组件（只读）
python -c "import sys; sys.path.insert(0, r'D:\Code\Rust\SC2Diff\tools'); import sc2mpq; \
           a = sc2mpq.MPQArchive(r'D:\Code\Rust\SC2Diff\testdata\samples\paiur01.SC2Map'); \
           print(a.names())"
~~~

---

## 2. 机制模型

### 2.1 为什么英文地图在中文下会显示成「错误的路径」

**[实测]** 关键证据链（全部可在 S1 内复核）：

1. 地图运行时调用的是一段**生成的 Galaxy 源码**。S1 的 `MapScript.galaxy`（328,621 B）里有 32 处：

   ~~~
   UIAlertPoint("Trigger", gv_pLAYER_01_USER, StringExternal("Param/Value/0F5D0539"), null, ...);
   VictoryPanelAddCustomStatisticLine(StringExternal("Param/Value/40F7F007"),
                                       TextExpressionAssemble("Param/Expression/ACF480BE"));
   ~~~

   即**运行时代码里硬编码的是「键」，不是文本**。
2. 键 `Param/Value/0F5D0539` 在 S1 的 65 个非本地化组件中出现在 `Triggers`（2 次）与
   `MapScript.galaxy`（1 次），**只存在于 .txt 字符串表里才对应到文本**。
3. `enUS.SC2Data\LocalizedData\GameStrings.txt` 里：`Param/Value/0F5D0539=Hive Destroyed`；
   `zhCN.SC2Data\LocalizedData\GameStrings.txt` 里：`Param/Value/0F5D0539=主巢被摧毁 /// Hive Destroyed`。
4. **推论（[推断]，但因果链完整）**：`StringExternal(key)` 在「当前 Locale.Data 的表里查不到 key」
   时返回 key 本身 → 界面上出现 `Param/Value/0F5D0539` 这种路径串。
   本次调研**没有**在中文客户端实机跑一张「只有 enUS」的地图来目视确认（见 §11 开放问题 O1）。

**[网上说法]（与实测一致）** 2010 年巴哈姆特/游民星空转载的教程标题即
《上传国外地图不会有 Param/Value/ACE39DF0 的方法》，正文原话：
「地图内可能有的语言：deDE…zhTW 繁体中文、zhCN 简体中文。**如果没有 zhTW 我们就创建
zhTW.SC2Data/LocalizedData/，再将其他语言里头的四个 txt 档案（GameHotkeys.txt、GameStrings.txt、
ObjectStrings.txt、TriggerStrings.txt）复制到 zhTW.SC2Data/LocalizedData/ 里面，这样一来就看的到英文啰**」。
——这正是「无跨语言回退，必须物理补齐目标语言目录」的社区实操结论。
来源：<https://wap.gamersky.com/gl/Content-162695.html>（原文出自巴哈姆特，2010-08-01）。

### 2.2 回退链（本次实测能证实与不能证实的部分）

| 说法 | 结论 | 依据 |
| --- | --- | --- |
| 同一文档内 zhCN 缺键 → 回退到该文档的 enUS | **不存在 [实测]** | S8（SCMRlocal）把 12 个 locale 目录写成**逐字节相同**——如果存在跨语言回退，这种做法毫无必要 |
| 依赖链上「地图自己的 zhCN」→「依赖的 zhCN」→「基础游戏的 zhCN」 | **成立 [实测]** | S6（kithotsbrutal.SC2Mod）`enUS` 有 2,420,633 B 全量文本，`zhCN` 只有 40 B 一条；该模组在中文客户端下**其余文本全部显示英文而非键路径**（模组可用），说明这些英文来自依赖链上其它文档的 enUS/文本表。**[未验证]** 具体是「依赖的 enUS」还是「依赖的某语言」——见 O2 |
| 游戏语言 zhCN 时是否会去读 `enUS.SC2Data` | **不会（同一文档内）[推断]** | 同上 + 社区教程 + 「创建 zhCN 目录就能看到英文」这一实操 |

### 2.3 「翻译写回哪里」

**[实测]** 写回**文档自身的** `<目标locale>.SC2Data\LocalizedData\<表名>.txt`，四个表：

| 表 | 决定什么 | 缺了会怎样 |
| --- | --- | --- |
| `GameStrings.txt` | **玩家可见文本**（触发器参数/变量文本、单位名、按钮名与提示、LoadingScreen、MapInfo 玩家名、DocInfo/Name） | 玩家界面显示键路径 |
| `ObjectStrings.txt` | **数据编辑器 / 触发器编辑器里显示的对象名** | 编辑器里对象显示为空白或 ID |
| `TriggerStrings.txt` | **触发器编辑器里的触发/函数/变量/分类显示名** | 触发器界面显示乱码/ID |
| `GameHotkeys.txt` | 快捷键字母的本地化标签 | 快捷键提示缺字 |
| `ConversationStrings.txt` | 战役对话字符名（仅战役用） | 字幕/说话人名为空 |

---

## 3. 文件格式语法（字节级）

样本统计口径：S5 全量 **297 个 locale .txt**、647,949 条 entry 行（其中 zhCN 83 个文件）。
解析脚本：`%TEMP%\sc2probe_25.py / _26 / _27 / _28 / _30 / _31 / _35.py`。

### 3.1 编码与 BOM

| 事实 | 观测 |
| --- | --- |
| 字符集 | **UTF-8**，无一例外 [实测]。全部 297 个官方文件 `utf-8-sig` 解码成功 |
| BOM | **两种都存在**：S5 的 297 个文件里 **142 个有 BOM、151 个无 BOM、4 个空文件** [实测] |
| BOM 与作者的关系 | 银河编辑器写出的文件**一定有 BOM**（S4 三个文件、S1u/S1u2/S2/S3 全部 locale 文件，实测 `EF BB BF`）；暴雪正式发行件的 zhCN 多为**无 BOM**（S5 里 `liberty.sc2campaign\zhCN...\gamestrings.txt`、`core.sc2mod\enUS...\editorcatalogstrings.txt` 均无 BOM）[实测] |
| BOM 是否影响解析 | **[推断]** 不影响；但**工具应原样保留**：有 BOM 就写 BOM，无 BOM 就不写 |

### 3.2 换行

| 样本 | 换行 | 观测 |
| --- | --- | --- |
| S4（编辑器保存的地图，enUS 三张表） | **CRLF** | `CRLF=190 / loneLF=0`（GameStrings）等，实测 3/3 |
| S1u / S1u2（12 locale 全量） | **CRLF** | 15 个文件全部 `loneLF=0` |
| S2 / S3（社区模组解包） | **CRLF** | 8 个文件全部 `loneLF=0` |
| S5（官方游戏本体解包） | **LF** | 297 个文件全部 `loneCR=0, CRLF=0`，纯 LF |
| S1（归档内原始字节，用 sc2mpq 直接读，不经解包工具） | **CRLF** | `enUS.SC2Data\...\GameStrings.txt` 51 个 CRLF、0 个 lone LF |

**结论 [实测]**：**两种换行在游戏里都能工作**。
**给实现的规范**：**保持与原文件一致**（读入什么就写出什么）；**新建文件时用 CRLF**
（编辑器与社区模组的主流，也是「编辑器再保存一次不会产生整文件 diff」的选择）。

### 3.3 语法规则

~~~
<key>=<value>
~~~

| 规则 | 实测 |
| --- | --- |
| 分隔符 | **只按第一个 `=` 切分**。值里可以出现 `=`：S1u enUS 51 条里 3 条值的内部含 `=`（`<s val="VoidLoadingIntroScreenTitle">…`）[实测] |
| 键的空白 | 实测 647,949 条 entry 中**键两侧无空白**；解析时仍应 `strip()` 容错 |
| 值的空白 | 允许。官方存在 `LoadingScreen/Tip= `（单个空格作值），S4 里就有 |
| 值的等号 | 允许且常见 |
| 转义 | **不存在**。`\\n`/`\\t` 不是转义：出现 `\\n` 的 87 条全部是资源路径 `Assets\\Textures\\...`；换行用 `<n/>` 标记 [实测] |
| 空值 | 合法：`DocInfo/DescLong=` |
| 空行 | 允许，可忽略 |
| 重复键 | **允许且真实存在**：官方 `campaigns\swarmstoryutil.sc2mod\zhCN...\GameStrings.txt` 有重复键（含 `Param/Value/lib_281DEC45_*`）、`mods\core.sc2mod\enUS...\ObjectStrings.txt` 有 `Abil/EditorPrefix/EnergizerReclamation` 等重复 [实测]。**[推断]** 语义是「后者覆盖前者」（或「先搜到的生效」），工具应**保留原顺序、只改目标键的第一条** |
| 行尾空白 | 实测无（`value_trailing_ws = 0` across 15 个样本文件） |

### 3.4 注释

| 事实 | 实测 |
| --- | --- |
| `//` 整行注释**确实存在** | `core.sc2mod\enUS.SC2Data\LocalizedData\Editor\EditorCatalogStrings.txt`（1,389,029 B，19,890 行）有 **11 行** `// Actor Sub Names` 这类整行注释，**没有 `=`**  |
| `//` **不是**行内注释 | 扫描 297 文件 647,949 条 entry，值里出现「` // `」（空格包夹、非 `///`）的：**0 条** [实测]。所以人类不会这么写，引擎大概也不支持 |
| 译者的「`//` 当键」事故 | S7（`0酒馆虚遗.SC2Mod`）的 zhCN ObjectStrings/TriggerStrings **把原作者写的 `----` 分隔行变成了键**：`------=`、`--------合作数据名称------------=`。同类现象在 S6 的 zhCN TriggerStrings 也有 189 行 [实测] |
| **安全判定** | **只有「以 `//` 开头且整行不含 `=`」才当作注释跳过**；含 `=` 的一律按 entry 处理（与官方 enUS 文件的实际形态吻合） |

### 3.5 ` /// ` 到底是什么（**这是本领域最重要的格式事实**）

**[实测]** 暴雪官方**中文**本地化文件用 `中文 /// English` 的形式**把英文原文附在译文后**：

| 样本 | 带 ` /// ` 的行 / 总行 |
| --- | --- |
| S5 `campaigns\liberty.sc2campaign\zhCN...\gamestrings.txt` | **3506 / 3678** |
| S5 `campaigns\libertystory.sc2campaign\zhCN...\gamestrings.txt` | **10743 / 10793** |
| S5 `campaigns\swarmstory.sc2campaign\zhCN...\conversationstrings.txt` | **8655 / 8655（100%）** |
| S5 `campaigns\void.sc2campaign\zhCN...\objectstrings.txt` | **12689 / 12689（100%）** |
| S5 `campaigns\voidstory.sc2campaign\zhCN...\triggerstrings.txt` | **6467 / 6467（100%）** |
| S5 `CDM\CM_Core_Mengsk.SC2Mod\zhCN...\GameHotkeys.txt` | **90 / 90（100%）** |
| S1u `zhCN...\GameStrings.txt`（暴雪原作地图） | **42 / 51** |
| S1u `koKR...\GameStrings.txt` | 46 / 51；`ptBR` 50 / 51；`esMX` 50 / 51 |
| S1u `deDE/esES/frFR/itIT/plPL/ruRU` | 0～1 / 51 |

**语义（三条硬结论）[实测]**：

1. **` /// ` 后面的英文就是该键在 enUS 表里的值，逐字节相同。**
   在 liberty（3506 条）、libertystory（10742 条）上做校验：`trailing == enUS值` 的比例 **100.0%**。
2. **不是注释、不是给解析器丢掉的标记，是显示文本的一部分。** S1 的 `zhCN` 里
   `Param/Value/0F5D0539=主巢被摧毁 /// Hive Destroyed`，而在 S1 的解包目录里
   `bench_paiur01_SC2Map/zhCN.../GameStrings.txt` 与 `bench2_paiur01_SC2Map` 的同名文件**逐字节相同**，
   说明它是随地图发行的正式内容，不是某次实验的残留。
   **[推断]** 引擎渲染时会剥掉 ` /// ` 之后的部分（因为玩家看不到英文）。
   **本次未做界面确认（O3）。**
3. **官方 `ObjectStrings.txt`（编辑器面向）基本不用 ` /// `**：liberty/libertystory/swarm 的 zhCN
   ObjectStrings ` /// ` 计数为 **0**，而 GameStrings/TriggerStrings/ConversationStrings/GameHotkeys 大量使用 [实测]。
   即：**` /// ` 是「给玩家看的字符串表」的惯例，不是编辑器表的惯例**。

**给实现的规范**：把 ` /// ` 之后的英文**当成值的一部分原样保留**，
**解析时不要吞掉、翻译时不要删掉**；如果工具要显示「纯中文」，自己在视图层切分，写回时拼回去。

### 3.6 值里的内联标记（**原样保留，不要动**）

| 标记 | 含义 | 例子（来自 S1/S4/S5） |
| --- | --- | --- |
| `<n/>` | 换行 | `Param/Value/F45B62B5=<n/>`（S4） |
| `<s val="…">…</s>` | 命名样式/字体模板作用域 | `LoadingScreen/Title=<s val="VoidLoadingIntroScreenTitle">LEGACY OF THE VOID</s>`（S1u） |
| `<c val="RRGGBB">…</c>` | 颜色 | `Press <c val="FFFFFF">F2</c> to Select All Units in your Army`（S1u） |
| `<IMG path="…" alignment="…"/>` | 内嵌图片 | `Abil/TargetMessage/…=<IMG path="Assets\Textures\tutorialmouseleftbuttondown.dds" alignment="absolutemiddle"/>   To Target`（S5） |
| `%token%` / `~A~` / `~B~` | 运行时替换占位符 | `Param/Expression/83371866=~A~ units`；`CustomStatisticLine` 里 `TextExpressionSetToken` 填 `"A"` |

**[实测]** 这些都是**纯文本标记**，不是转义序列；翻译时**只能整体搬运**，位置可调但**不能改写或丢弃**。

---

## 4. locale 组件的命名与加载

### 4.1 组件名形态 [实测]

归档内组件名用**反斜杠**分隔（不是正斜杠）：

~~~
zhCN.SC2Data\LocalizedData\GameStrings.txt
zhCN.SC2Data\LocalizedData\ObjectStrings.txt
zhCN.SC2Data\LocalizedData\TriggerStrings.txt
zhCN.SC2Data\LocalizedData\GameHotkeys.txt
zhCN.SC2Data\LocalizedData\ConversationStrings.txt
Base.SC2Data\GameData\UnitData.xml          <- 语言无关
~~~

`Base.SC2Data` 与 `<locale>.SC2Data` 的分工 [实测，S1 组件清单可见]：

* `Base.SC2Data\GameData\*.xml` = Catalog 数据（语言无关）；
* `Base.SC2Data\Cutscenes\...`、`Base.SC2Data\Scripts\...`、`Base.SC2Data\UI\...` = 其它语言无关资源；
* `<locale>.SC2Data\LocalizedData\*.txt` = **只在该语言下加载**的字符串表。

### 4.2 有哪些 locale 前缀 [实测]

* 官方 12 个：`deDE enUS esES esMX frFR itIT koKR plPL ptBR ruRU zhCN zhTW`
  （S1 的 `ComponentList.SC2Components` 里就是这 12 个 `<DataComponent Type="text" Locale="…">GameText</DataComponent>`）。
* 真实归档里**还出现过**：`enCN enSG enTW`（S8 `SCMRmod.SC2Mod` 里的 `enCN.SC2Data`/`enSG.SC2Data`/`enTW.SC2Data`）、
  `enGB`（社区教程提到）。
  **规范 [推断]**：locale 段只校验形状（`[A-Za-z]{2,4}`），不要把「官方 12 个」写成白名单。

### 4.3 运行时语言从哪来 [实测]

`C:\Users\ROG\Documents\StarCraft II\EditorVariables.txt` 与 `Variables.txt`：

~~~
localeidassets=zhCN
localeiddata=zhCN
localeidinstall=zhCN
~~~

游戏日志把三者分别打印（L1，2026-10-08 23:39:43，编辑器以 `-run "Test\EditorTest.SC2Map"` 启动）：

~~~
<Locale.Assets>         zhCN
<Locale.Data>           zhCN
<Locale.Install>        zhCN
~~~

**[推断]** 决定「读哪个 `<locale>.SC2Data\LocalizedData`」的是 **`Locale.Data`**。
三者通常同值；本机 GameLogs 里 489 次采样**全部是 zhCN**。

### 4.4 sidecar `.version` 文件的作用 [实测 + 推断]

S1 的 `GameText.version`（44 B）原始字节：

~~~
63 64 65 73 74 78 65 74                         "cdestxet"
02 00 00 00
06 A3 00 00  03 00 00 00  05 00 00 00  00 00 00 00
06 A3 00 00  03 00 00 00  C1 CE F1 56  ...
~~~

* 头部 `cdestxet` 是 **"text" 的小端反转 + "cd"**（`t``e``x``t` 反过来 → `txet`；`adag`→"gada"=`GameData.version`）。
  即 `cd` + 组件类型码（`text`/`gada`/`info`/`trig`…）的字节反转 [实测：三个 sidecar 头部互不相同且都能反解]。
* 长度**恒为 44 字节**：4 字节魔数 + `0x02` + 若干 `u32`（组件偏移/大小 `0x0000A306`、`u32 6`）+ **一个 `u32 Unix 时间戳**（`0xF1CEC1` 之类）[实测：`Objects.version`/`DocumentInfo.version`/`Regions.version` 长度都是 44]。
* **[推断]** 它是**缓存的组件指纹/时间戳**，用于「组件是否需要重新加载」的快速判断。
  `ComponentList.SC2Components` 里 `<DataComponent Type="text" Locale="zhCN">GameText</DataComponent>` 才是**组件清单**，
  sidecar 不在清单里但**必须存在**（SC2Diff 的实测：改内容、不同步 sidecar 与 attributes → 编辑器报
  `e_fileCorrupt File: DocumentInfo.version`）。

**规范**：回填字符串表时，`GameText.version` **跟着一起更新**（至少更新时间戳字段），
或者**走「由编辑器自己重新保存一次」的路径**——见 §8.4。

### 4.5 `ComponentList.SC2Components` 没列出的 locale 也能生效 [实测]

S1u 有 12 个 locale 的 GameStrings，但 S1 的 `ComponentList.SC2Components` 只把 `deDE…zhTW` 都列了；
S2/S3（社区模组）**只有 zhCN 一个 `LocalizedData` 目录、且 ComponentList 里一个 `text` 组件都没有**，
却在中文下正常工作。
**[推断]** `LocalizedData` 是按**约定路径**加载的，ComponentList 不是唯一入口；
但**新加 locale 时把 `<DataComponent Type="text" Locale="…">GameText</DataComponent>` 一并补上更安全**。

---

## 5. 键命名空间清单（真实样本出现次数）

### 5.1 表之间的键空间**互不重叠** [实测]

官方 core.sc2mod：`GameStrings` 8,549 键 vs `ObjectStrings` 1,658 键 → **交集 0**；
`TriggerStrings`（swarmstory enUS）3,702 键 → 与两者交集**都是 0**。
**规范**：一张表的键只能写回同一张表。

### 5.2 `GameStrings.txt`（面向玩家）

官方 core.sc2mod enUS（8,549 键）前 20 个前缀：

| 前缀 | 条数 |
| --- | --- |
| `UI/CREDITS` | 1407 |
| `UI/CreditTextHotS` | 588 |
| `UI/CreditText` | 471 |
| `UI/Hotkey` | 328 |
| `Cutscenes/Property` | 315 |
| `UI/Editor` | 143 |
| `UI/BattleChat` | 116 |
| `UI/Error` | 115 |
| `Param/Value` | 98 |
| `UI/ProductPurchase` | 52 |
| `Cutscenes/Value` | 49 |
| `UI/ProfileDialog` | 48 |
| `UI/BattleUser` | 47 |
| `Abil/Name` | 46 |
| `UI/DescError` | 44 |
| `Cutscenes/Category` | 43 |
| `UI/HeroSelectPanel` | 42 |
| `UI/ScreenStore` | 35 |

地图侧（S1u，51 键）出现的前缀：

| 前缀 | 条数 | 说明 |
| --- | --- | --- |
| `Param/Value` | 32 | **8 位 HEX 键**，触发器参数的文本 |
| `Param/Expression` | 3 | **8 位 HEX 键**，触发器文本表达式模板 |
| `DocInfo/Name` | 1 | 地图名（可发布标题） |
| `Effect/Name` | 1 | 未翻译时值 == 键 |
| `LoadingScreen/TextBody` / `Tip` / `TipRestart` / `Title` | 各 1 | 载入画面 |
| `MapInfo/Player00..09/Name` | 10 | 玩家名 |

其它真实前缀（S2/S3/S7/S8 实测）：`Button/Name`、`Button/Tooltip`、`Button/Hotkey`、
`Unit/Name`、`Unit/Tooltip`、`Unit/Category`、`Unit/LifeArmorName`、`Unit/ShieldArmorName`、
`Upgrade/Name`、`Abil/Name`、`Behavior/Name`、`Behavior/Tooltip`、`Weapon/Name`、`Weapon/Tip`、
`Effect/Name`、`RequirementNode/Tooltip`、`Skin/Name`、`Skin/Info`、`Race/Name`、`Planet/Name`、
`Planet/Tip`、`Character/Name`、`Hotkey/`、`Key/`、`Error/`、`UI/…`、`DocInfo/PatchNoteNNN`。
另有 **`lib_<8HEX>_<8HEX>` 形式**（S6：`Param/Value/lib_FDAD8721_3BD00D8C=0`；
S8：`Category/Name/lib_AA0EBE8C_2678C4C9=Episode 6`）——**库命名空间键，可跨地图复用** [实测，S8 与 S6 共用同一 lib 前缀]。
还有浏览器式键 `Borked`、`Colonel`、`Commander`（S8 实测）——键**允许不含斜杠**。

### 5.3 `ObjectStrings.txt`（编辑器面向）

官方 core.sc2mod enUS（1,658 键）主前缀：

| 前缀 | 条数 |
| --- | --- |
| `Actor/Name` | 701 |
| `Validator/Name` | 228 |
| `Model/Name` | 181 |
| `Sound/Name` | 63 |
| `Cursor/Name` | 61 |
| `DSP/Name` | 41 |
| `Alert/Name` | 36 |
| `Effect/Name` | 29 |
| `Reverb/Name` | 28 |
| `RequirementNode/Name` | 25 |
| `TargetSort/Name` | 24 |
| `PhysicsMaterial/Name` | 22 |
| `Behavior/Name` | 20 |
| `Token/Name` / `Token/Tooltip` | 各 20 |
| `Accumulator/Name` / `AttachMethod/Name` / `Light/Name` | 各 12 |

第二段命名空间（全类型实测取值）：`Name`、`EditorPrefix`、`EditorSuffix`、`Tooltip`（Token/ 下还有 4 段形式）。

### 5.4 `TriggerStrings.txt`（触发器编辑器）

官方 swarmstory enUS（3,702 键）：

| 前缀 | 条数 |
| --- | --- |
| `Variable/Name` | 1378 |
| `ParamDef/Name` | 757 |
| `FunctionDef/Name` | 586 |
| `PresetValue/Name` | 260 |
| `Trigger/Name` | 240 |
| `FunctionDef/Grammar` | 180 |
| `Category/Name` | 131 |
| `FunctionDef/Hint` | 106 |
| `Preset/Name` | 46 |
| `SubFuncType/Name` | 15 |
| `Library/Name` | 2 |
| `Label/Name` | 1 |

S1u enUS（310 键）：`Variable/Name` 123、`Trigger/Name` 122、`ParamDef/Name` 27、`Category/Name` 24、
`FunctionDef/Name` 11、`FunctionDef/Grammar` 3。S7 zhCN（14,905 键）另有 `Structure/Name`、`CustomScript` 等。

### 5.5 其它表

* `GameHotkeys.txt`：**全部是 `Button/Hotkey/<按钮ID>`**（liberty enUS 699 键，S1u 无此文件，S2 132 键，S3 15 键）。
  同族变体 `_NRS`、`_USD`、`_USDL`。[实测]
* `ConversationStrings.txt`：`Conversation/<场景ID>/LineNNNNN`、`Character/Name/<ID>`、`Character/Description/<ID>`、`Character/Attitude/<ID>`（swarmstory enUS 8,639 键）[实测]。

---

## 6. 键集合差统计表（真实数字）

统计脚本：`%TEMP%\sc2probe_29.py`（S1u）、`sc2probe_30.py`（S2/S3/S1u）、`sc2probe_28.py`（S5）。
口径：按第一个 `=` 切分，跳过「`//` 开头且无 `=`」的行，键去重。

### 6.1 S1u（暴雪原作地图，12 locale 齐全）— GameStrings.txt

| locale | 键数 | 与 enUS 的交集 | 只在 enUS | 只在本 locale | 值 != enUS | 值 == enUS | 带 ` /// ` | 字节 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| enUS | 51 | 51 | 0 | 0 | 0 | 51 | 0 | 2,673 |
| deDE | 51 | 51 | 0 | 0 | 48 | 3 | 1 | 3,131 |
| esES | 51 | 51 | 0 | 0 | 47 | 4 | 0 | 2,873 |
| esMX | 51 | 51 | 0 | 0 | 51 | 0 | 50 | 4,621 |
| frFR | 51 | 51 | 0 | 0 | 47 | 4 | 0 | 2,925 |
| itIT | 51 | 51 | 0 | 0 | 48 | 3 | 0 | 2,913 |
| koKR | 51 | 51 | 0 | 0 | 49 | 2 | 46 | 4,464 |
| plPL | 51 | 51 | 0 | 0 | 48 | 3 | 0 | 2,788 |
| ptBR | 51 | 51 | 0 | 0 | 51 | 0 | 50 | 4,563 |
| ruRU | 51 | 51 | 0 | 0 | 48 | 3 | 1 | 4,444 |
| **zhCN** | **51** | **51** | **0** | **0** | **49** | **2** | **42** | **4,203** |
| zhTW | 51 | 51 | 0 | 0 | 50 | 1 | 35 | 4,007 |

**结论**：暴雪发行时**每个 locale 都补满全部 51 个键**，键集合**完全一致（差集 0/0）**。
「键集合差」在官方产物里是 0——**这是回填工具应该追求的不变量**。

### 6.2 S1u — ObjectStrings.txt / TriggerStrings.txt（**真实的覆盖缺口**）

| 文件 | locale | 键数 | 与 enUS 差集 |
| --- | --- | --- | --- |
| ObjectStrings.txt | enUS | 8 | — |
| ObjectStrings.txt | zhCN | **1** | **只缺 7 条**（只补了 `Unit/EditorSuffix/Purifier`） |
| TriggerStrings.txt | enUS | 310 | — |
| TriggerStrings.txt | zhCN | **文件不存在** | **缺 310 条** |

**这就是「编辑器/触发器界面显示英文或 ID」的直接成因**：
玩家可见的 GameStrings 补满 12 语言，编辑器面向的 ObjectStrings/TriggerStrings 只补了 enUS（+1 条 zhCN）。

### 6.3 S1u2 — TriggerStrings.txt 与 ObjectStrings.txt

| 文件 | locale | 键数 |
| --- | --- | --- |
| TriggerStrings.txt | enUS | 310（唯一） |
| ObjectStrings.txt | enUS | 8 / zhCN 1 |

与 S1u 完全一致（两目录同源）。

### 6.4 S2 / S3（社区模组：**只有 zhCN，没有 enUS**）— 反向缺口

| 样本 | 文件 | zhCN 键数 | enUS |
| --- | --- | --- | --- |
| S2 bench_umojan | GameStrings.txt | 1,176 | **无** |
| S2 | ObjectStrings.txt | 5,402 | **无** |
| S2 | TriggerStrings.txt | 11,507 | **无** |
| S2 | GameHotkeys.txt | 132 | **无** |
| S3 bench_HTXL_SC2Mod | GameStrings.txt | 535 | **无** |
| S3 | ObjectStrings.txt | 2,671 | **无** |
| S3 | TriggerStrings.txt | 1 | **无** |
| S3 | GameHotkeys.txt | 15 | **无** |

**含义**：这批样本是**中文客户端下的「正常态」**——它们只需要 zhCN。
要做「zhCN → enUS」的反向翻译时，**enUS 侧是 100% 缺失**，没有源文本可用。
S3 的 GameStrings 里还夹着一批**键就是中文词**的条目（`偏射盾`、`先进护盾`、`龙鳞装甲` …，共 19 条），
是社区工具导出后回填不当留下的痕迹 [实测]。

### 6.5 S5（官方游戏本体）— enUS vs zhCN

| 文件 | enUS 键 | zhCN 键 | 只在 enUS | 只在 zhCN | 交集 | ` /// ` 且尾部 == enUS 值 |
| --- | --- | --- | --- | --- | --- | --- |
| liberty `gamestrings.txt` | 3,678 | 3,678 | **0** | **0** | 3,678 | **3506 / 3506 = 100%** |
| libertystory `gamestrings.txt` | 10,793 | 10,793 | 1 | 1 | 10,792 | **10742 / 10742 = 100%** |
| core `gamestrings.txt` | 8,549 | 8,549 | **0** | **0** | 8,549 | 0（该文件不用 ` /// `） |

libertystory 的两条错位键（唯一的差集）[实测]：

* 只在 enUS：`ConversationState/StoryState/IsStoryTransitionSave`
* 只在 zhCN：`Unit/Name/MengskStatueAlone`

### 6.6 S7（真实中文模组）与 S8（本地化补丁包）的形状

| 样本 | 观测 [实测] |
| --- | --- |
| S7 `0酒馆虚遗.SC2Mod` | zhCN GameStrings 171 键 / ObjectStrings 2,786 键 / TriggerStrings 14,905 键 / GameHotkeys 3 键；` /// ` = 0；ObjectStrings/TriggerStrings 里混入 189+ 条「`----` 当键」的脏数据 |
| S8 `SCMRlocal.SC2Mod` | 12 个 locale × 4 张表 = 48 个 `LocalizedData` 组件；`enUS/zhCN/zhTW/koKR/deDE` 同名文件 **SHA256 完全一致**；内容 CJK 字符数 = **0**（全英文） |

**S8 是本项目最直接的「应该怎么做」范本**：把源语言内容**逐字节复制**到目标语言目录，
零文本改动即可让玩家看到「英文而不是键路径」。**它也是「无跨语言回退」的最强证据。**

---

## 7. 编辑器写入行为

### 7.1 编辑器把文本写到哪个文件/哪个键 [实测，S4]

S4（`EditorTest.SC2Map`，编辑器自己保存的地图）的 `enUS.SC2Data\LocalizedData`：

| 文件 | 大小 | 条目 | 前缀 |
| --- | --- | --- | --- |
| GameStrings.txt | 7,159 B | 190 | `Param/Value` 103、`Param/Expression` 78，其余 DocInfo/MapInfo/LoadingScreen/Button/Tooltip/DOCSTR_Models |
| ObjectStrings.txt | 400 B | 12 | `Light/Name` 8、`Actor/Name`、`Model/Name`、`Sound/Name`、`Soundtrack/Name` |
| TriggerStrings.txt | 22,464 B | 567 | `Variable/Name` 366、`Trigger/Name` 91、`ParamDef/Name` 49、`FunctionDef/Name` 33、`Category/Name` 11、`Structure/Name` 9、`FunctionDef/Grammar` 6、`FunctionDef/Hint` 2 |

**[实测] 用户在触发器里输入的每一段文本，都变成 `Param/Value/<8位HEX>`**，
例如 `Param/Value/F664FE3C=<s val="SCR_OptionsLabelLeft">Classic/Starcraft:Remastered</s>`。
编辑器面向的对象名进 `ObjectStrings.txt`；触发器/函数/变量的显示名进 `TriggerStrings.txt`。

**注意**：S4 的 `ComponentList.SC2Components` 只列了 `<DataComponent Type="text" Locale="enUS">GameText</DataComponent>` 一条，
而**运行时的 Locale.Data 是 zhCN**（L1 实测）。
**[推断]** 编辑器写哪个 locale 由**文档自己的「主语言」**决定（新地图默认 enUS），
**不跟随当前客户端语言**。所以中文作者用中文客户端做出来的地图，文本可能仍然在 `enUS.SC2Data` 里。

### 7.2 `Param/Value/<HEX>` 的 HEX 是什么 [实测：不是哈希，是随机 ID]

| 检验 | 结果 |
| --- | --- |
| 同值不同键 | S1u 有 **6 条**值都是 `Zerg Hive`（`8371F17B/94D38143/DF723E79/E463F7B0/EEE9BAFA/…`）、**5 条**都是 `Attack`、**4 条**都是 `Hive Destroyed`。→ **不是内容哈希** |
| MD5/SHA1/SHA256/CRC32 片段命中 | 对 `0F5D0539`（Hive Destroyed）、`FF93BC11`（Attack）、`40F7F007`（Zerg Units Destroyed）在多种编码/大小写变体下探测：**零命中** |
| 与文档结构的耦合 | 每个 HEX 都是 `Triggers` 里某个 `<Element Type="Param" Id="XXXXXXXX">` 的 **Id**；S1 中 35 个 `Param/*` HEX **全部**能在 `Triggers` 找到 Id（各 2 次：定义 + 引用），**零孤儿** |
| 与生成代码的耦合 | `MapScript.galaxy` 里有 32 处 `StringExternal("Param/Value/…")`，**逐一对上** |
| 跨地图复用 | S1u（35 个 HEX）∩ S2/S3 的 `Param/Value`（都是 0 个）→ 无法比。但可与 S4 比：S1u 的 35 个 HEX 与 S4 的 181 个 HEX **交集 0**。两份**都源自 paiur01** 的文档（S1u 与 S1u2）键集合**逐字节相同**，说明**同一文档内稳定、跨文档不复用** |

**结论 [实测+推断]**：`Param/Value/<HEX>` 与 `Param/Expression/<HEX>` 的 HEX 是
**编辑器为 Triggers 文档里的 Parameter 元素分配的 8 位十六进制随机 Id**（形如上层的 `Trigger Id="48B02AFA"`）。
**不是文本的哈希，不可在另一张地图上复用**；同一个文本在两张地图里会有**不同的键**。
唯一例外是**库命名空间键** `Param/Value/lib_<libHEX>_<keyHEX>`（S6/S8 实测），那是可复用的。

**给实现的关键推论**：**翻译必须按「键」回填，绝不能按「值」匹配**。
「把英文 `Attack` 的中文填回去」这种按值匹配的做法，
在一张地图里有 5 个不同键都是 `Attack` 时必然出错（虽然这 5 个键的目标文本恰好相同，但一般情形不成立）。

### 7.3 同一个文本在不同地图里键是否可复用

**不可复用 [实测]**（§7.2 的交集 0 证据）。
可复用的只有：**catalog 派生键**（`Unit/Name/<catalog id>`、`Button/Tooltip/<id>` …）
与**库命名空间键** `lib_<libHEX>_<keyHEX>`。地图自有的 `DocInfo/*`、`MapInfo/PlayerNN/*`、`LoadingScreen/*` 也稳定（不随文档变）。

### 7.4 编辑器保存会不会重排/丢弃键

**[实测]** S1u 与 S1u2 两份同源解包的 `enUS.GameStrings` **逐键逐值相同**（51/51）。
**[推断]** 编辑器保存时按组件整体重写，键序会按内部顺序（大体是字母序，见 S1u enUS 的顺序：
`DocInfo/Name` → `Effect/Name/…` → `LoadingScreen/…` → `MapInfo/…` → `Param/Expression/…` → `Param/Value/…`）。
**规范**：工具不要依赖键序；但**保留原顺序**可以让编辑器再保存一次后 diff 最小。

---

## 8. 给实现者的操作规范

### 8.1 目标产物

给定「源 locale（如 enUS）」与「目标 locale（如 zhCN）」：

1. 在归档内**新增/补齐** `<target>.SC2Data\LocalizedData\<表>.txt`（表名 `GameStrings.txt` /
   `ObjectStrings.txt` / `TriggerStrings.txt` / `GameHotkeys.txt` / `ConversationStrings.txt`，
   **只补归档里实际存在的表**，不要凭空造）。
2. **键集合必须与源 locale 逐字一致**（不增不减；差集目标 0/0，见 §6.1）。
3. 值 = 译文；**保留原值里的所有内联标记**（`<n/>` `<s>` `<c>` `<IMG/>` `%token%` `~A~`）；
   **保留 ` /// <英文原文>` 约定**（§3.5）。
4. **不要动**：`DocInfo/Name`（发布标题，改了可能无法再发布；社区工具 KSP 默认「不翻译地图标题」）、
   `MapInfo/PlayerNN/Name` 里的 `Neutral`/`Hostile` 之类引擎语义名（若译文已在官方表里有对应，可用官方译法）。
5. `ObjectStrings.txt` 的 `*/EditorPrefix/<id>` 往往是**缩写码**（`MR`、`U`），**不要按语言改写**。

### 8.2 字节级写出规范

| 项 | 规范 | 依据 |
| --- | --- | --- |
| 字符集 | **UTF-8**（无例外） | §3.1 |
| BOM | **与原文件一致**；新建文件**写 BOM**（编辑器与社区模组都写） | §3.1 |
| 换行 | **与原文件一致**；新建文件用 **CRLF** | §3.2 |
| 末尾换行 | 原文件有就保留（实测样本**都有**） | `endsWithNewline=True`（S1u 15/15、S4 3/3） |
| 分隔符 | 第一个 `=`；**不要**对键/值做 URL/XML 转义 | §3.3 |
| 注释 | 跳过并原样保留「`//` 开头且无 `=`」的行 | §3.4 |
| 空行 | 原样保留 | §3.3 |
| 重复键 | 只改**第一条**，其余原样；或整体按「后者覆盖」语义处理 | §3.3 |

### 8.3 写回归档（MPQ）

**不要自己拼 MPQ**（除非你已经实现了 SC2Diff 那条已验证的路径）。要点（详见
`docs/research/mpq-spec.md`）：

* 头部 `dwRawChunkSize`（偏移 `0x6C`）**必须写 0**，否则编辑器报
  `e_fileCorrupt File: <组件> … MD5VerifyData::ValidateRead`；
* 头部 `0xC0` 处的**自哈希 MD5** 与 `0x70/0x80/0xA0/0xB0` 的四个表 MD5 必须重算；
* `(attributes)` 的 CRC32/MD5 要按块表索引同步；
* **最稳的验收路径**：改完 → 让**银河编辑器自己打开并保存一次**（`editor_accept.py` 那套驱动），
  再以「编辑器保存后的产物」为交付物。SC2Diff 的实测结论是
  「合法 MPQ（StormLib 可读）≠ SC2 编辑器接受」，**必须用编辑器本体做验收**。

### 8.4 避免破坏原文件

1. **只增不改**：优先「新增 `<target>.SC2Data\LocalizedData` + 同步 `GameText.version`」，
   不改动任何已有的其它 locale 组件。
2. **保留未知行**：解析成「有序记录列表」，无法识别的行作为 `RawLine` 原样输出
   （本仓库 `python\src\sc2trans\strings.py` 已经这么设计）。
3. **只改目标键**：不要重排、不要规整大小写、不要统一空白。
4. **原文快照**：回填前把 `<target>.SC2Data\LocalizedData` 与 `GameText.version` 一起备份/入档，
   便于回滚与 diff。
5. **不写 `zhTW`/其它没有请求的 locale**——除非用户明确要求「全语言同内容」（S8 那种策略）。
6. **保留 ` /// `**：工具若提供「只显示中文」的视图，切分只发生在内存里。

### 8.5 验收清单

* [ ] 目标 locale 的每张表，键集合与源 locale 差集为 **0/0**（或把差集**显式列出**）。
* [ ] 每条译文的**内联标记**数量与源一致（`<n/>` 计数、`<c val=` 计数）。
* [ ] 文件编码/BOM/换行与源一致（或按 §8.2 的新建规范）。
* [ ] 归档被 **StormLib** 打开并全量抽取成功（`tools/stormlib_verify.exe` 已被 SC2Diff 实现并验证过）。
* [ ] **银河编辑器**能打开该地图/模组（SC2Diff 的 `editor_accept.py` 判据：
      模块窗口标题出现文件名 = LOADED）。
* [ ] 游戏内实机确认：原「键路径」处显示中文（**本项本次未做，见 O1**）。

---

## 9. 其它可参考实现

| 名字 | 类型 | 做法 | 可借鉴点 | 来源 |
| --- | --- | --- | --- | --- |
| **StormLib** | C++ 库 | MPQ v1-v4 读写；SC2 的 208 字节头/HET/BET 都支持 | 容器层的权威实现；SC2Diff 已用它做独立验收 | `D:\Code\Rust\SC2Diff\reference\StormLib`；<https://github.com/ladislav-zezula/StormLib> |
| **SC2Diff**（同作者） | Python + Rust | 自研 MPQ 读写 + 组件级语义 diff + 编辑器验收装置 | **本仓库的直接上游**：`tools/sc2mpq.py`、`tools/stormlib_verify.exe`、`tools/editor_accept.py`、`research/findings.md` | `D:\Code\Rust\SC2Diff` |
| **Localization Editor SC2 KSP** | Java 桌面应用（有 CLI `sc2loc`） | 直接打开 .SC2Map/.SC2Mod/mpq，按语言列对比、翻译、写回 `.SC2Data/LocalizedData`；带 SC2 术语表 | ① 内置 glossary（`sc2_word_glossary_KSP.txt` 等）保证术语一致；② `sc2loc check-missing` 的检查项就是我们要实现的校验（缺 locale 目录/缺键/空值/畸形行/重复键）；③ **默认不翻译 `DocInfo/Name`**（发布标题锁定缓存） | <https://github.com/VoVanRusLvSC2/Localization-Editor-SC2-KSP> |
| **SCMRlocal.SC2Mod** | 社区「本地化补丁」产物 | 12 locale × 4 表**同内容复制** | 「宁可全语言同一份，也不要有 key 缺失」的兜底策略 | `D:\Game\BLZ\StarCraft II\Mods\SCMRlocal.SC2Mod` |
| **0酒馆虚遗.SC2Mod** | 社区中文模组 | 只提供 zhCN，全中文 | 真实成品形态；也暴露了「`----` 当键」的脏数据处理问题 | `D:\Game\BLZ\StarCraft II\Mods\0酒馆虚遗.SC2Mod` |
| **SC2_GameTranslater** | GitHub 项目（Whimsyduke） | 声称「Tools for translating strings in StarCraft II maps or mods」 | **本次未能读到内容**（原 fork `Cyberenchanter/SC2_GameTranslater` 返回 404，仅搜到 `Whimsyduke/SC2_GameTranslater`）；见 O4 | <https://github.com/Whimsyduke/SC2_GameTranslater> |
| 巴哈姆特/游民星空教程（2010） | 图文教程 | MPQEditor 打开地图 → 建 `<locale>.SC2Data/LocalizedData/` → 复制 4 个 txt | **最早的「无跨语言回退」实操证据** | <https://wap.gamersky.com/gl/Content-162695.html> |
| GA 地精研究院 / bbs.islga.org | 中文社区论坛 | 讨论贴（「如何令地图支持不同的语言环境」「请教一张奇葩的地图汉化问题」） | **本次被 Cloudflare 拦截（HTTP 403）**，未能读取正文；见 O5 | <https://bbs.islga.org/forum.php?mod=viewthread&tid=56922> |

---

## 10. 结论速查

1. **回填位置**：`<locale>.SC2Data\LocalizedData\<表>.txt`（组件名用反斜杠）。
2. **格式**：UTF-8（BOM 视原文件）、`key=value` 按第一个 `=`、CRLF/LF 视原文件、
   `//` 整行（无 `=`）才是注释、` /// <英文>` 是**值的一部分**。
3. **键**：地图自有的 `Param/Value/<8HEX>` 是**编辑器分配的随机 Id**，与 Triggers 元素耦合，
   **跨地图不可复用**；只能**按键**回填。
4. **回退**：同一文档内**没有**跨语言回退；缺键 → 显示键路径。
5. **不变量**：目标 locale 的键集合应与源 locale **完全一致（差集 0/0）**——官方 12 语言产物就是这么做的。
6. **兜底**：实在翻译不过来，**原样复制源语言内容到目标 locale**（S8 策略）也能让玩家看到英文而非键路径。
7. **验收**：StormLib + **银河编辑器本体**，两者都要过。

---

## 11. 仍未验证 / 开放问题（显式标注）

| 编号 | 问题 | 现状 | 建议验证方式 |
| --- | --- | --- | --- |
| **O1** | **没有在中文客户端实机目视确认**「只有 enUS 的地图 → 界面出现 `Param/Value/xxx`」 | 因果链完整（`StringExternal` + 键缺失），但缺一次实机观察 | 拿一张只有 `enUS.SC2Data` 的地图，在 zhCN 客户端跑，截取触发器文本区 |
| **O2** | 依赖链上的回退粒度：源地图 zhCN 缺键时，是「找依赖的 enUS」还是「找依赖的同 locale」？ | S6 的现象支持「会去依赖里取文本」，但没定位到具体语言层 | 造两个模组 A（enUS 有、zhCN 无）与 B（依赖 A，zhCN 有该键），二分观察 |
| **O3** | ` /// <英文>` 在渲染时是否被引擎剥掉？剥的规则是「第一个 ` /// `」还是「最后一个」？ | 官方 71,920 行使用该约定（§3.5），但未做界面确认 | 在只有 ` /// ` 的本地化条目上实机观察；或找带**两个** ` /// ` 的边界样本 |
| **O4** | `SC2_GameTranslater` 的具体做法 | 仓库 404 / 未读到源码 | 直接克隆 `Whimsyduke/SC2_GameTranslater` 阅读 |
| **O5** | 中文社区（islga）关于「奇葩地图汉化」的经验贴 | Cloudflare 403，未读到 | 用浏览器（agent-browser 技能）访问 <https://bbs.islga.org/read-htm-tid-53077-page-1-fpage-1.html> |
| **O6** | `.version` sidecar 的精确语义（哪些字段必须跟着改） | 只确认「44 字节 + 时间戳 + 组件偏移/大小」；未做「不改 sidecar 是否被接受」的受控实验 | 复制一份地图，只改字符串表内容与 sidecar 时间戳、其余不动，看编辑器接受与否 |
| **O7** | **银河编辑器保存时选哪个 locale**（S4 用 enUS 写入，但运行时 Locale.Data=zhCN） | 只有单点观测 | 在 zhCN 客户端新建地图 → 存 → 看 `ComponentList` 与 `*.SC2Data` 的 locale；（本次**不能**跑编辑器：只读调研） |
| **O8** | 键大小写是否敏感（`find_block` 大小写敏感，但 HET/BET 哈希小写化） | 容器层的结论已有；**字符串表的键**是否大小写敏感未测 | 在测试地图里写一个大小写变体键，观察是否生效 |
| **O9** | 值长度上限 / UI 截断 | 未测 | 用超长中文串实机观察 |
| **O10** | 官方 `ObjectStrings.txt` 为何不用 ` /// `；若我们用了会不会影响编辑器 UI | 只有「官方不用」的统计事实 | 在 ObjectStrings 里加 ` /// ` 后开编辑器观察显示 |

---

## 附录 A：本次只读探针脚本清单（位于 `%TEMP%`）

| 脚本 | 作用 |
| --- | --- |
| `sc2probe_01.py` | S1u 字节级事实（BOM/换行/行数） |
| `sc2probe_02.py` | S1u 12 locale 的键数与 ` /// ` 分布 |
| `sc2probe_04.py`/`05.py` | S6 真实模组（enUS 全量 vs zhCN 1 条） |
| `sc2probe_07.py` | 用 `sc2mpq` 解包 S4 到 `%TEMP%\sc2probe_editortest` |
| `sc2probe_08.py` | S4 编辑器产物统计 |
| `sc2probe_09.py` | `Param/Value` HEX 的哈希假设检验（全部否定） |
| `sc2probe_13.py` | S4 的 `ValueType text` Param ↔ GameStrings 键的耦合率 |
| `sc2probe_14.py` | 反例分析（`<ValueType text>` 缺失的 Param） |
| `sc2probe_15.py` | 跨地图 HEX 复用检验 |
| `sc2probe_18.py`/`19.py` | S8 本地化补丁包的组件清单与逐语言 SHA256 |
| `sc2probe_22.py` | S7 真实中文模组的键空间与脏数据 |
| `sc2probe_25.py`–`28.py`、`35.py` | S5 官方全量 297 文件的编码/换行/注释/` /// ` 统计 |
| `sc2probe_29.py`/`30.py` | 键集合差统计（S1u/S2/S3） |
| `sc2probe_31.py` | 注释与转义语义检验（647,949 行） |
| `sc2probe_34.py` | HEX 与 `Triggers` 的耦合验证 |
| `sc2probe_37.py`/`38.py` | `StringExternal` 与 `MapScript.galaxy` 的耦合验证 |

## 附录 B：本次**未**产生的副作用

* 未修改 `D:\Code\Rust\SC2Diff` 下任何文件（只读打开）。
* 未修改 `D:\Game\BLZ\StarCraft II`、`%USERPROFILE%\Documents\StarCraft II` 下任何文件。
* 未修改本仓库除 `docs/research/sc2-localization.md` 以外的文件。
* 未执行 `git commit`。
