import { format } from "./format";

describe("format", () => {
  it("formats to the width", () => {
    for (const [value, want] of [
      ["a", "a  "],
      ["abcd", "abc"],
      ["abc", "abc"],
    ]) {
      expect(format(value, 3)).toBe(want);
    }
  });
});
