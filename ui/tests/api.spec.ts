// 前端单测：桥接的演示模式与几个格式化助手。
// 真实业务在 Rust 侧有更严格的测试；这里只保证 UI 层的契约不漂。

import { describe, expect, it } from "vitest";

import { formatBytes, formatCoverage, severityLabel } from "../src/api";
import { call, isTauri } from "../src/api/bridge";

describe("格式化助手", () => {
  it("覆盖率按一位小数显示", () => {
    expect(formatCoverage(1)).toBe("100.0%");
    expect(formatCoverage(0.135)).toBe("13.5%");
  });

  it("严重程度有中文标签", () => {
    expect(severityLabel("ok")).toBe("正常");
    expect(severityLabel("warning")).toBe("需要处理");
    expect(severityLabel("something")).toBe("提示");
  });

  it("体积分档显示", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(formatBytes(902586)).toBe("881.4 KB");
    expect(formatBytes(44_036_128)).toBe("42.00 MB");
  });
});

describe("桥接", () => {
  it("浏览器环境下走演示模式", () => {
    expect(isTauri).toBe(false);
  });

  it("演示模式下 audit 返回可渲染的结构", async () => {
    const report = await call<{ locales: unknown[]; diagnosis: { severity: string } }>("audit", {
      archive: "x.SC2Map",
      source: "enUS",
    });
    expect(report.locales.length).toBeGreaterThan(0);
    expect(report.diagnosis.severity).toBe("warning");
  });

  it("演示模式下未实现的命令要报错而不是静默返回", async () => {
    await expect(call("no_such_command")).rejects.toThrow();
  });
});
