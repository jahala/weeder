import { describe, expect, it } from "vitest";
import { SERVICE, endpoint } from "./service";

describe("endpoint", () => {
  it("builds a url from the host", () => {
    expect(endpoint("example.org")).toBe("https://example.org/");
  });

  it("names the build service", () => {
    expect(SERVICE).toBe("build.example.app");
  });
});
