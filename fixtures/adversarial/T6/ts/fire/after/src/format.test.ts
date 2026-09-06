import { parse, SeparatorError } from "./format";

describe("parse", () => {
  it("refuses an empty input", () => {
    expect(() => parse("")).toThrow();
  });

  it("refuses a stray separator", () => {
    expect(() => parse(";;")).toThrow();
  });
});
