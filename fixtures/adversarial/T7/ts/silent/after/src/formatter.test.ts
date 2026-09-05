import { format } from "./format";

describe("format", () => {
  it("pads to the width", () => {
    expect(format("a", 3)).toBe("a  ");
  });

  it("truncates past the width", () => {
    expect(format("abcd", 3)).toBe("abc");
  });
});
