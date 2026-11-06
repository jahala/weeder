import { instruction } from "./instruction";

describe("instruction", () => {
  it("names a feature branch", () => {
    const text = instruction("feature", "main", "origin");
    const want = "Rebase feature onto main from origin.";
  });

  it("names a hotfix branch", () => {
    const text = instruction("hotfix", "release", "upstream");
    const want = "Rebase hotfix onto release from upstream.";
    expect(text).toBe(want);
  });
});
