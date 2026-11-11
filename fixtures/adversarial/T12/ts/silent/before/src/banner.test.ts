import { describe, expect, it } from "vitest";
import { banner } from "./banner";

describe("banner", () => {
  it("marks the banner as an alert", () => {
    expect(banner(new Error("x"))).toContain("role");
  });
});
