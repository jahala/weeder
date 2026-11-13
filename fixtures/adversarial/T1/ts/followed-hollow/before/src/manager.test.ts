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
    retryConnect(dialler(0), 3);
  });

  it("retries until it connects", () => {
    retryConnect(dialler(2), 5);
  });

  it("gives up", () => {
    retryConnect(dialler(9), 2);
  });
});
