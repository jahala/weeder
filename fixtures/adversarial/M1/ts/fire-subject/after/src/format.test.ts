import { describe, expect, it, vi } from "vitest";
import { format } from "./format";
import { now } from "./clock";

vi.mock("./format");

describe("format", () => {
  it("pads to the width", () => {
    expect(format("a", 3)).toBe("a  ");
  });

  it("cuts a value longer than the width", () => {
    expect(format("abcd", 3)).toBe("abc");
  });
});
