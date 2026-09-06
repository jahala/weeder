import { format } from "./format";
import { parse } from "./parser";

describe("format", () => {
  it("pads to the width", () => {
    expect(format("a", 3)).toBe("a  ");
  });

  it("leaves an exact fit alone", () => {
    expect(format("abc", 3)).toBe("abc");
  });
});

describe("parse", () => {
  it("splits on commas", () => {
    expect(parse("a,b")).toEqual(["a", "b"]);
  });

  it("leaves an empty input empty", () => {
    expect(parse("")).toEqual([]);
  });
});
