// 前端单测：桥接的演示模式与展示助手。
// 真实业务在 Rust 侧有更严格的测试；这里保证 UI 层的契约不漂。

import { describe, expect, it } from "vitest";

import { baseName, formatBytes, formatCoverage, markupTokens, statusLabel } from "../src/api";
import { DEMO_ARCHIVE, call, isTauri } from "../src/api/bridge";
import { entryId } from "../src/composables/useProject";
import type { Entry, TranslationDocument } from "../src/api/types";

describe("格式化助手", () => {
  it("覆盖率按一位小数显示", () => {
    expect(formatCoverage(1)).toBe("100.0%");
    expect(formatCoverage(0.135)).toBe("13.5%");
  });

  it("体积分档显示", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(formatBytes(902586)).toBe("881.4 KB");
    expect(formatBytes(44_036_128)).toBe("42.00 MB");
  });

  it("状态有中文标签", () => {
    expect(statusLabel("missing")).toBe("缺失");
    expect(statusLabel("translated")).toBe("已翻译");
    expect(statusLabel("unknown-status")).toBe("unknown-status");
  });

  it("跨平台取文件名", () => {
    expect(baseName("D:\\Maps\\a.SC2Map")).toBe("a.SC2Map");
    expect(baseName("/home/x/b.SC2Mod")).toBe("b.SC2Mod");
    expect(baseName("plain.SC2Map")).toBe("plain.SC2Map");
  });

  it("识别必须保留的内联标记", () => {
    const text = 'Press <c val="FFFFFF">F2</c> for ~A~ units<n/> %done%';
    const tokens = markupTokens(text);
    expect(tokens).toContain("<c val=\"FFFFFF\">");
    expect(tokens).toContain("<n/>");
    expect(tokens).toContain("~A~");
    expect(tokens).toContain("%done%");
    expect(markupTokens(null)).toEqual([]);
  });

  it("行 id 由字符串表 + 键组成（同键不同表要区分）", () => {
    const a: Entry = { key: "DocInfo/Name", file: "GameStrings.txt", status: "missing" };
    const b: Entry = { key: "DocInfo/Name", file: "ObjectStrings.txt", status: "missing" };
    expect(entryId(a)).not.toBe(entryId(b));
  });
});

describe("桥接", () => {
  it("浏览器环境下走演示模式", () => {
    expect(isTauri).toBe(false);
  });

  it("演示模式能载入键表，且统计自洽", async () => {
    const doc = await call<TranslationDocument>("load_keys", {
      archive: DEMO_ARCHIVE,
      source: "enUS",
      target: "zhCN",
    });
    expect(doc.entries.length).toBeGreaterThan(20);
    expect(doc.stats.entries).toBe(doc.entries.length);
    const sum = Object.values(doc.stats.by_status).reduce((a, b) => a + b, 0);
    expect(sum).toBe(doc.entries.length);
    expect(doc.entries.some((e) => e.status === "missing")).toBe(true);
    expect(doc.entries.some((e) => e.status === "translated")).toBe(true);
  });

  it("演示模式能导入译文", async () => {
    const raw = (await call("import_document", { path: "x.json" })) as { entries: unknown[] };
    expect(Array.isArray(raw.entries)).toBe(true);
    expect(raw.entries.length).toBeGreaterThan(0);
  });

  it("演示模式下未实现的命令要报错而不是静默返回", async () => {
    await expect(call("no_such_command")).rejects.toThrow();
  });
});
