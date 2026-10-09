// 撤销栈的单测：合并窗口、容量上限、重做作废、批量。

import { describe, expect, it, vi } from "vitest";

import { HISTORY_LIMIT, applyPatches, useHistory } from "../src/composables/useHistory";

describe("撤销栈", () => {
  it("记一步就能撤销、再重做", () => {
    const h = useHistory();
    h.record("A", "", "苹果");
    h.flush();
    expect(h.canUndo.value).toBe(true);
    expect(h.canRedo.value).toBe(false);

    expect(h.undo()).toEqual([{ key: "A", before: "", after: "苹果" }]);
    expect(h.canUndo.value).toBe(false);
    expect(h.canRedo.value).toBe(true);

    expect(h.redo()).toEqual([{ key: "A", before: "", after: "苹果" }]);
    expect(h.canRedo.value).toBe(false);
  });

  it("同一个键的连续输入合并成一步", () => {
    const h = useHistory();
    const t = vi.spyOn(Date, "now");
    t.mockReturnValue(1000);
    h.record("A", "", "苹");
    t.mockReturnValue(1200);
    h.record("A", "", "苹果");
    t.mockReturnValue(1400);
    h.record("A", "", "苹果汁");
    h.flush();
    expect(h.depth.value).toBe(1);
    expect(h.undo()).toEqual([{ key: "A", before: "", after: "苹果汁" }]);
    t.mockRestore();
  });

  it("隔久了就是新的一步", () => {
    const h = useHistory();
    const t = vi.spyOn(Date, "now");
    t.mockReturnValue(1000);
    h.record("A", "", "苹");
    t.mockReturnValue(5000);
    h.record("A", "", "苹果");
    h.flush();
    expect(h.depth.value).toBe(2);
    t.mockRestore();
  });

  it("换了键就是新的一步", () => {
    const h = useHistory();
    h.record("A", "", "苹果");
    h.record("B", "", "香蕉");
    h.flush();
    expect(h.depth.value).toBe(2);
  });

  it("没变的不记", () => {
    const h = useHistory();
    h.record("A", "x", "x");
    h.flush();
    expect(h.depth.value).toBe(0);
    expect(h.canUndo.value).toBe(false);
  });

  it("新改动作废重做", () => {
    const h = useHistory();
    h.record("A", "", "1");
    h.flush();
    h.undo();
    expect(h.canRedo.value).toBe(true);
    h.record("A", "", "2");
    h.flush();
    expect(h.canRedo.value).toBe(false);
  });

  it("超过上限丢最旧的，最新的还在", () => {
    const h = useHistory();
    for (let i = 0; i < HISTORY_LIMIT + 20; i += 1) {
      h.recordBatch([{ key: "K" + i, before: "", after: String(i) }]);
    }
    expect(h.depth.value).toBe(HISTORY_LIMIT);

    let newest: string | undefined;
    let oldest: string | undefined;
    for (let i = 0; i < HISTORY_LIMIT; i += 1) {
      const step = h.undo();
      if (i === 0) newest = step?.[0]?.key;
      oldest = step?.[0]?.key;
    }
    expect(newest).toBe("K" + (HISTORY_LIMIT + 19));
    // 最旧的 20 步已经被丢掉，撤到底应当停在 K20
    expect(oldest).toBe("K20");
    expect(h.canUndo.value).toBe(false);
  });

  it("批量改动是一步", () => {
    const h = useHistory();
    h.recordBatch([
      { key: "A", before: "", after: "苹果" },
      { key: "B", before: "", after: "香蕉" },
    ]);
    expect(h.depth.value).toBe(1);
    expect(h.undo()).toHaveLength(2);
  });
});

describe("补丁回放", () => {
  it("撤销倒着回放、重做正着回放", () => {
    const patches = [
      { key: "A", before: "a0", after: "a1" },
      { key: "A", before: "a1", after: "a2" },
    ];
    const doc = new Map<string, string>([["A", "a2"]]);
    applyPatches(patches, doc, "before");
    expect(doc.get("A")).toBe("a0");
    applyPatches(patches, doc, "after");
    expect(doc.get("A")).toBe("a2");
  });

  it("普通对象也能回放", () => {
    const doc: Record<string, string> = { A: "x" };
    applyPatches([{ key: "A", before: "old", after: "x" }], doc, "before");
    expect(doc.A).toBe("old");
  });
});
