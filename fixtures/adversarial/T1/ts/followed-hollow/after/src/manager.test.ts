import { Manager } from "./manager";

describe("manager", () => {
  it("opens", () => {
    expect(new Manager().open(() => true)).toBe(true);
  });
});
