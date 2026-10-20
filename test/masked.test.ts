import { describe, expect, it } from "vitest";

import { mosaicCells } from "../src/components/Masked";

describe("mosaicCells", () => {
  it("is deterministic per text", () => {
    expect(mosaicCells("alice@example.com")).toEqual(mosaicCells("alice@example.com"));
    expect(mosaicCells("alice@example.com").cells).not.toEqual(
      mosaicCells("bob@example.com").cells,
    );
  });

  it("clamps column count so length is not leaked exactly", () => {
    expect(mosaicCells("ab").cols).toBe(6);
    expect(mosaicCells("a".repeat(60)).cols).toBe(14);
    const { cols, cells } = mosaicCells("someone@x.com");
    expect(cells).toHaveLength(cols * 2);
  });

  it("never contains the source text", () => {
    const { cells } = mosaicCells("secret-name");
    expect(cells.every((c) => c.startsWith("#"))).toBe(true);
  });
});
