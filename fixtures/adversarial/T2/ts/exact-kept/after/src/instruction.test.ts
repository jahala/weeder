import { instruction } from "./instruction";

describe("instruction", () => {
  it("names the branch, the base and the remote", () => {
    let text = instruction(
      "feature", "main", "origin",
    );
    let want = [
      "Rebase feature onto main from origin.",
    ].join("");
    expect(text).toBe(want);
  });
});
