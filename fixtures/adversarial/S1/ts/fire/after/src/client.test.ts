import { Strategy } from "./client";

class Recording extends Strategy {
  retry(payload: string): string {
    return payload;
  }
}

describe("strategy", () => {
  it("records the payload", () => {
    expect(new Recording().retry("a")).toBe("a");
  });
});
