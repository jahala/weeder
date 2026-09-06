import { summarise } from "./report";

test("summarise joins the rows", () => {
  console.log("what the suite saw", summarise(["a", "b"]));
  expect(summarise(["a", "b"])).toBe("a, b");
});
