import { parse } from "./parser";

describe("parse", () => {
  it("splits on commas", () => {
    expect(parse("a,b")).toEqual(["a", "b"]);
  });

  it("leaves an empty input empty", () => {
    expect(parse("")).toEqual([]);
  });

  it("keeps a single field", () => {
    expect(parse("a")).toEqual(["a"]);
  });
});
