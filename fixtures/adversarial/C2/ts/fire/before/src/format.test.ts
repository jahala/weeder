import { format } from "./format";

test("joins the fields", () => {
  expect(format(["a", "b"])).toBe("a,b");
});
