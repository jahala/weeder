import { format } from "./format";

describe("format", () => {
  it("pads to the width", () => {
    expect(format("a", 3)).toBe("a  ");
  });

  it("truncates past the width", () => {
    expect(format("abcd", 3)).toBe("abc");
  });

  test("leaves an exact fit alone", () => {
    expect(format("abc", 3)).toBe("abc");
  });

  it("pads an empty value", () => {
    expect(format("", 2)).toBe("  ");
  });

  test("pads with spaces", () => {
    expect(format("ab", 4)).toBe("ab  ");
  });
});
