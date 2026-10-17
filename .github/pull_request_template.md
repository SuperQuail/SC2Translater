## 做了什么

<!-- 一两句说清楚这个 PR 解决什么。 -->

## 为什么

<!-- 背景 / 触发它的现象。有 issue 就链上。 -->

## 怎么验证的

<!-- 写具体命令和结果，不要写「测试通过」。 -->

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `pnpm -C ui build && pnpm -C ui test`

## 影响面

- [ ] 改了 **MPQ 写侧**（`crates/sc2mpq` / 写回归档的路径）→ 必须附银河编辑器验收结果，
      见 [docs/design/验收清单.md](../docs/design/验收清单.md)
- [ ] 改了 **core 的 JSON 结构** → 已同步 `crates/sc2trans-core/src/model.rs` 与 `ui/src/api/types.ts`
- [ ] 改了 **界面** → 已按 [AGENTS.md](../AGENTS.md) 第 8 节先出图并拿到同意
- [ ] 以上都没有

## 破坏性变更

<!-- 没有就写「无」。有的话写清楚老用户会怎样。 -->
无
