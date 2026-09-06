import { parse } from "./parser";

test("parse splits on commas", () => {
  expect(parse("a,b")).toEqual(["a", "b"]);
});
