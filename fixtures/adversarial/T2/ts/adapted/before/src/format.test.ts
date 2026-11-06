import { format } from "./format";

describe("format", () => {
  it("formats to the width", () => {
    expect(format("a", 3)).toBe("a  ");
    expect(format("abcd", 3)).toBe("abc");
    expect(format("abc", 3)).toBe("abc");
  });
});
