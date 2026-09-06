import { format } from "./format";

describe("padding", () => {
  it("leaves an exact fit alone", () => {
    expect(format("abc", 3)).toBe("abc");
  });
});
