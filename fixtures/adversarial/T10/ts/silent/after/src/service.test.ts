import { describe, expect, it } from "vitest";
import { SERVICE, endpoint } from "./service";

describe("endpoint", () => {
  it("builds a url from the host", () => {
    expect(endpoint("example.org")).toBe("https://example.org/");
  });

  it("points at the build service", () => {
    expect(endpoint(SERVICE)).toBe("https://build.example.app/");
  });
});
