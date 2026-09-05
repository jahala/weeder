import { format } from "./format";

describe("format", () => {
  it("pads the value out to the width", () => {
    expect(format("a", 3)).toBe("a  ");
  });

  it("truncates past the width", () => {
    expect(format("abcd", 3)).toBe("abc");
  });

  it("leaves an exact fit alone", () => {
    expect(format("abc", 3)).toBe("abc");
  });
});
