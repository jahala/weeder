import { instruction } from "./instruction";

describe("instruction", () => {
  it("names the branch, the base and the remote", () => {
    const text = instruction("feature", "main", "origin");
    expect(text).toContain("feature");
    expect(text.toLowerCase()).toContain("main");
    expect(text).toContain("origin");
  });
});
