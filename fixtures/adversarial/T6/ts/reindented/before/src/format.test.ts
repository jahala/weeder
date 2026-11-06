import { parse, SeparatorError } from "./format";

it("rejects bad input", () => {
  expect(() => parse("")).toThrow();
  expect(() => parse(";;")).toThrow(SeparatorError);
});
