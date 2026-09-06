import { summarise } from "./report";

test("summarise joins the rows", () => {
  expect(summarise(["a", "b"])).toBe("a, b");
});
