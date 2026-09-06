import { format } from "./format";

describe("format", () => {
  it("pads to the width", () => {
    expect(format("a", 3)).toBe("a  ");
    expect(format("ab", 3)).toBe("ab ");
  });

  it("truncates past the width", () => {
    expect(format("abcd", 3)).toBe("abc");
    expect(format("abcde", 4)).toBe("abcd");
    expect(format("abcdef", 2)).toBe("ab");
  });
});
