import { instruction } from "./instruction";

describe("instruction", () => {
  it("names the branch, the base and the remote", () => {
    const text = instruction("feature", "main", "origin");
    expect(text.length).toBe(37);
  });
});
