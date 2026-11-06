import { contrast } from "./contrast";

describe("contrast", () => {
  it("text on paper reads in light", () => {
    expect(contrast("light", "text", "paper")).toBeGreaterThanOrEqual(4.5);
  });

  it("muted on paper reads in light", () => {
    expect(contrast("light", "muted", "paper")).toBeGreaterThanOrEqual(4.5);
  });

  it("edge on paper shows in light", () => {
    expect(contrast("light", "edge", "paper")).toBeGreaterThanOrEqual(3);
  });

  it("focus on paper shows in light", () => {
    expect(contrast("light", "focus", "paper")).toBeGreaterThanOrEqual(3);
  });

  it("text on paper reads in dark", () => {
    expect(contrast("dark", "text", "paper")).toBeGreaterThanOrEqual(4.5);
  });

  it("muted on paper reads in dark", () => {
    expect(contrast("dark", "muted", "paper")).toBeGreaterThanOrEqual(4.5);
  });

  it("edge on paper shows in dark", () => {
    expect(contrast("dark", "edge", "paper")).toBeGreaterThanOrEqual(3);
  });

  it("focus on paper shows in dark", () => {
    expect(contrast("dark", "focus", "paper")).toBeGreaterThanOrEqual(3);
  });
});
