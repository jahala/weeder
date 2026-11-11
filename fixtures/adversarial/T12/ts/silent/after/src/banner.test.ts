import { describe, expect, it } from "vitest";
import { banner } from "./banner";

describe("banner", () => {
  it("marks the banner as an alert", () => {
    expect(banner(new Error("x"))).toContain("role");
  });

  it("wraps the error in a paragraph", () => {
    const error = new Error("Workspace not found");
    expect(banner(error)).toBe('<p role="alert">Workspace not found</p>');
  });
});
