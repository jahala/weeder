import { Manager, retryConnect } from "./manager";

/** A dial that answers after the given number of refusals. */
function dialler(failures: number): () => boolean {
  let calls = 0;
  return () => {
    calls += 1;
    return calls > failures;
  };
}

describe("manager", () => {
  it("opens", () => {
    expect(new Manager().open(dialler(0))).toBe(true);
  });

  it("connects at once", () => {
    const [attempts, connected] = retryConnect(dialler(0), 3);
    expect(connected).toBe(true);
    expect(attempts).toBe(1);
  });

  it("retries until it connects", () => {
    const [attempts, connected] = retryConnect(dialler(2), 5);
    expect(connected).toBe(true);
    expect(attempts).toBe(3);
  });

  it("gives up", () => {
    const [attempts, connected] = retryConnect(dialler(9), 2);
    expect(connected).toBe(false);
    expect(attempts).toBe(2);
  });
});
