import { format } from "./format";
import { widths } from "./widths";

describe("format", () => {
  it.each(widths)("formats %s to the width", (value, want) => {
    expect(format(value, 3)).toBe(want);
  });

  it("pads to the width", () => {
    expect(format("a", 3)).toBe("a  ");
  });
});
