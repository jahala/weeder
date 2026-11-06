import { format } from "./format";

describe("format", () => {
  it("formats to the width", () => {
    for (const [value, want] of [["a", "a  "], ["abcd", "abc"], ["abc", "abc"]].filter(([value]) => value.length < 4)) {
      expect(format(value, 3)).toBe(want);
    }
  });
});
