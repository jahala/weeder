import { ratio, settle, total } from "./format";

describe("ratio", () => {
  it("divides to four places", () => {
    expect(ratio(1, 3)).toBeCloseTo(0.3333, 6);
  });

  it("settles before the deadline", async () => {
    await waitFor(() => settle(), { timeout: 50 });
    expect(total(4, 5)).toBe(9);
  });
});
