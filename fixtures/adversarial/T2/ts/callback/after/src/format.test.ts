import { format } from "./format";

const WIDTHS: [string, string][] = [["a", "a  "], ["abcd", "abc"], ["abc", "abc"]];

describe("format", () => {
  it("formats to the width", () => {
    WIDTHS.forEach(([value, want]) => {
      expect(format(value, 3)).toBe(want);
    });
  });
});
