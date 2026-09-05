import { parse, SeparatorError } from "./format";

describe("parse", () => {
  it("refuses an empty input", () => {
    expect(() => parse("")).toThrow("input is empty");
  });

  it("refuses a stray separator", () => {
    expect(() => parse(";;")).toThrow(SeparatorError);
  });
});
