import { instruction } from "./instruction";

describe("instruction", () => {
  it("names the branch, the base and the remote", () => {
    let text = "";
    let want = "";
    text = instruction("feature", "main", "origin");
    want = "Rebase feature onto main from origin.";
    expect(text).toBe(want);
    text = instruction("hotfix", "release", "upstream");
    want = "Rebase hotfix onto release from upstream.";
    expect(text).toBe(want);
  });
});
