import { format } from "./format";

describe("format", () => {
  it("truncates past the width", () => {
    expect(format("abcd", 3)).toBe("abc");
  });
});
