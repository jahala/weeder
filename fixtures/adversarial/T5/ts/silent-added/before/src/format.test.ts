import { format } from "./format";

describe("format", () => {
  it("keeps a value already at the width", () => {
    expect(format("abcd", 4)).toBe("abcd");
  });
});
