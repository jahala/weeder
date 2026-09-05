import { format } from "./format";

describe("format", () => {
  it("pads to the width", () => {
    expect(format("a", 3)).toBe("a  ");
  });

  it.skip("truncates past the width", () => {
    expect(format("abcd", 3)).toBe("abc");
  });

  it.only("leaves an exact fit alone", () => {
    expect(format("abc", 3)).toBe("abc");
  });

  test.todo("refuses a negative width");

  xit("pads with the fill character", () => {
    expect(format("a", 3)).toBe("a..");
  });
});

xdescribe("format, at the edges", () => {
  it("pads to a hundred", () => {
    expect(format("a", 100)).toHaveLength(100);
  });
});
