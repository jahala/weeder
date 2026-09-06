import { format } from "./format";

describe("format", () => {
  it("pads to the width", () => {
    expect(format("a", 3)).toBe("a  ");
    expect(format("ab", 3)).toBe("ab ");
    expect(format("", 3)).toBe("   ");
  });

  it("truncates past the width", () => {
    expect(format("abcd", 3)).toBe("abc");
  });
});
