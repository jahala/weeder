import { format } from "./format";

describe("padding", () => {
  it("leaves an exact fit alone", () => {
    expect(format("abc", 3)).toBe("abc");
  });

  it("pads to the width", () => {
    const padded = format("a", 3);
    expect(padded).toBe("a  ");
    expect(format("ab", 3)).toBe("ab ");
  });

  it("cuts a long value down", () => {
    expect(format("abcde", 4)).toBe("abcd");
  });
});
