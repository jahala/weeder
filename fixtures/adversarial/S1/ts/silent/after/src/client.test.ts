import { label, send } from "./client";

describe("client", () => {
  it("trims the payload", () => {
    expect(send(" a ")).toBe("a");
  });

  it("labels work still to do", () => {
    // TODO: cover the retry path once the queue lands
    expect(label("todo")).toBe("TODO: written by the caller");
  });
});
