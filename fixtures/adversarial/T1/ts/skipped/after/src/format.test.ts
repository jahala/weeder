import { format } from "./format";

describe("format", () => {
  it("pads to the width", () => {
    expect(format("a", 3)).toBe("a  ");
  });

  it.skip("truncates past the width", () => {
    expect(format("abcd", 3)).toBe("abc");
  });

  test.only("leaves an exact fit alone", () => {
    expect(format("abc", 3)).toBe("abc");
  });

  xit("pads an empty value", () => {
    expect(format("", 2)).toBe("  ");
  });

  test.todo("pads with spaces");
});
