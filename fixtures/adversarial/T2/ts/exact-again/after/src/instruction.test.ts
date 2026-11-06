import { instruction } from "./instruction";

describe("instruction", () => {
  it("names the branch, the base and the remote", () => {
    const want = "Rebase feature onto main from origin.";
    let text = instruction("feature", "main", "origin");
    text = instruction("feature", "main", "origin");
    expect(text).toBe(want);
  });
});
