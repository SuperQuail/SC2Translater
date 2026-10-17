# 参与开发

本文档是**硬规矩**，不是建议。CI 会照着这里卡。

---

## 1. 分支模型：主干 + 短命分支

只有一条长期分支：`main`。

* `main` **永远可发布** —— 任何时刻从它切出来的版本都应该是好的；
* 所有改动走**短命分支**，名字带类型前缀，活不过几天：
  ```
  feat/keys-preview-toggle
  fix/empty-value-keys
  docs/branch-rules
  ```
* **不许直接推 `main`**（规则集拦着），只能通过 PR；
* 合并后分支**自动删除**。

分支名里的类型词与下面的提交类型一致，看名字就知道这条分支在做什么。

---

## 2. 提交信息：Conventional Commits

格式：

```
<类型>[: 范围]: <说明>

<正文：为什么这么改，不是改了什么>

<可选：脚注，如 BREAKING CHANGE: ...>
```

### 类型表

| 类型 | 用在哪 |
| --- | --- |
| `feat` | 新功能 |
| `fix` | 修缺陷 |
| `perf` | 性能 |
| `refactor` | 重构（不改行为） |
| `docs` | 文档 |
| `test` | 只动测试 |
| `build` | 构建脚本 / 依赖（Cargo.toml、package.json） |
| `ci` | CI 配置 |
| `chore` | 杂活（改 .gitignore 这类） |
| `revert` | 回滚 |

### 例子（本仓库的真实风格）

```
fix: 空值键没被补齐 + 打开文件夹进不去编辑页面

一、补键漏 29 个（CLI，ops.rs）
- 现象：一个 mod 里 zhCN 覆盖率卡在 96.4%。
- 根因：`if pair.value.is_empty() { continue }` 把原文就是空值的键整句跳过了，
  结果它们在中文下显示成键路径。
- 改法：缺失的键即使是空值也写进去；但绝不用空值覆盖已有内容。
```

**要点**：正文写**为什么**。diff 已经说清楚改了什么，不要再复述一遍。

### 范围（可选）

改动集中在一个模块时写范围，例如 `fix(gui):`、`feat(cli):`、`docs(design):`。

### 破坏性变更

在正文里写 `BREAKING CHANGE: <老用户会怎样>`，并在 PR 里说明迁移方式。

---

## 3. 合并方式：squash

* **只允许 squash 合并**（仓库设置里已经关掉 merge commit 与 rebase）；
* squash 后的提交信息 = **PR 标题 + PR 正文**，所以：
  * **PR 标题必须是 Conventional Commits 格式**（有单独的工作流在卡）；
  * PR 正文写清楚为什么，它会变成提交正文；
* `main` 要求**线性历史**，不允许 merge commit 混进去。

**这意味着**：分支上的提交信息可以随便写，最终只留 PR 那一条 —— 但请仍然写清楚，
因为评审的人要读。

---

## 4. 合并前必须满足

规则集（Repository ruleset）在 `main` 上强制：

| 要求 | 说明 |
| --- | --- |
| 必须开 PR | 直接推 `main` 会被拒 |
| 必须过 CI | 两个作业都要绿：`Rust（格式 / 静态检查 / 测试）`、`前端（构建 / 测试）` |
| 必须解决所有对话 | 有未解决的 review 评论就不能合 |
| 线性历史 | 不允许 merge commit |
| 禁止强推 / 禁止删除 | `main` 只能往前走 |
| 管理员可绕过 | 紧急修复用，用完请补 PR 说明 |

> 本仓库目前是单人维护，所以**不要求 review 批准**（要求了会把自己锁在外面）。
> 有第二位维护者之后，把 ruleset 里的 `required_approving_review_count` 调成 1。

---

## 5. 本地必跑（与 CI 一致）

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
pnpm -C ui install
pnpm -C ui build
pnpm -C ui test
```

CI 跑在 Windows 上（目标平台 + workspace 含 src-tauri），本地也是 Windows 时结果一致。

---

## 6. 什么不要提交

已在 `.gitignore` 里，但说清楚原因：

| 路径 | 为什么不进库 |
| --- | --- |
| `testdata/` | 真实地图样本，**有版权**，不进仓库 |
| `refs/` | 别人的截图，只作查阅 |
| `scratch/` | 设计探索的中间产物 |
| `dist/`、`target/`、`ui/dist/` | 构建产物 |

`.gitattributes` 强制了换行归一（源码 LF，Galaxy 脚本 CRLF），二进制归档按 `binary` 处理。

---

## 7. 改这些地方要额外做什么

| 改了什么 | 额外要求 |
| --- | --- |
| MPQ 写侧（`crates/sc2mpq`、写回归档的路径） | 必须重跑**银河编辑器验收**，见 [docs/design/验收清单.md](docs/design/验收清单.md) |
| `sc2trans-core` 的 JSON 结构 | 同步 `crates/sc2trans-core/src/model.rs` **与** `ui/src/api/types.ts` |
| 界面 | 先出图拿同意再写码 —— [AGENTS.md](AGENTS.md) 第 8 节 |
| 字符串表的读写 | 逐字节保真，改完跑 `verify`（`string_tables_roundtrip_exact`） |

其余项目约定（SC2 领域的坑、硬性约束清单）都在 [AGENTS.md](AGENTS.md)，**动手前读一遍**。

---

## 8. 发布

打 tag 走 `v<主>.<次>.<修订>`，正文写这一版用户看得见的变化。
发布产物需要 `SC2Translater.exe` 与 `sc2diff.exe` **放在同一目录**（见 AGENTS 第 14 条）。
