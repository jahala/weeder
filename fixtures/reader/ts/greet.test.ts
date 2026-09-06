import { greet } from "./greet";

describe("greet", () => {
  it("greets by name", () => {
    expect(greet("weeder")).toBe("hello weeder");
  });
});
