import { Client } from "./client";

describe("client", () => {
  it("connects", () => {
    expect(new Client().connect(() => true)).toBe(true);
  });
});
