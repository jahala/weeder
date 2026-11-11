import { describe, expect, it } from "vitest";
import { format } from "./format";

describe("format", () => {
  it("pads to the width", () => {
    expect(format("a", 3)).toBe("a  ");
  });
});
