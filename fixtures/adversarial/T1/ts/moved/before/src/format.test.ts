import { format } from "./format";

describe("format", () => {
  it("pads to the width", () => {
    expect(format("a", 3)).toBe("a  ");
  });

  it("leaves an exact fit alone", () => {
    expect(format("abc", 3)).toBe("abc");
  });
});
