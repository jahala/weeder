import { instruction } from "./instruction";

describe("instruction", () => {
  it("names the branch, the base and the remote", () => {
    let text = "";
    let want = "";
    text = instruction("hotfix", "release", "upstream");
    want = "Rebase hotfix onto release from upstream.";
    expect(text).toBe(want);
  });
});
