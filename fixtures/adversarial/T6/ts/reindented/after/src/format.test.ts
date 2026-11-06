import { parse, SeparatorError } from "./format";

describe("parse", () => {
  it("rejects bad input", () => {
    expect(() => parse("")).toThrow();
    expect(() => parse(";;")).toThrow(SeparatorError);
  });
});
