import { ratio, settle, total } from "./format";

describe("ratio", () => {
  it("divides to four places", () => {
    expect(ratio(1, 3)).toBeCloseTo(0.3333, 2);
  });

  it("settles before the deadline", async () => {
    await waitFor(() => settle(), { timeout: 2000 });
    expect(total(1, 2)).toBe(3);
  });
});
