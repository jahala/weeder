import { format } from "./format";

describe("format", () => {
  it("pads to the width", () => {
    expect(format("a", 3)).toBe("a  ");
  });

  // it.skip came off this case when the padding stopped rounding.
  it("truncates past the width", () => {
    expect(format("abcd", 3)).toBe("abc");
  });

  it("names the markers a reviewer greps for", () => {
    expect(markers()).toEqual(["it.skip", "it.only", "test.todo", "xit", "xdescribe"]);
  });
});
