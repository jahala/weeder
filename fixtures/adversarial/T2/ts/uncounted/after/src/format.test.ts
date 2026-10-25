import { format } from "./format";

function widths(): [string, string][] {
  return [["a", "a  "], ["abcd", "abc"], ["abc", "abc"]];
}

describe("format", () => {
  it("formats to the width", () => {
    for (const [value, want] of widths()) {
      expect(format(value, 3)).toBe(want);
    }
  });
});
