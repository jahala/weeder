import { greet } from "./greet";

describe("greet", () => {
  it("greets by name", () => {
    expect(greet("weed")).toBe("hello weed");
  });
});
