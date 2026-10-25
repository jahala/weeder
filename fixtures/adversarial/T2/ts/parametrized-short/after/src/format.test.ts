import { format } from "./format";

describe("format", () => {
  it.each([
    ["a", "a  "],
    ["abcd", "abc"],
  ])("formats %s to the width", (value, want) => {
    expect(format(value, 3)).toBe(want);
  });
});
