import { Client, retryConnect } from "./client";

/** A dial that answers after the given number of refusals. */
function dialler(failures: number): () => boolean {
  let calls = 0;
  return () => {
    calls += 1;
    return calls > failures;
  };
}

describe("client", () => {
  it("connects", () => {
    expect(new Client().connect(() => true)).toBe(true);
  });

  it("keeps dialling until the server answers", () => {
    const [first, answered] = retryConnect(dialler(0), 3);
    expect(answered).toBeTruthy();
    expect(first).toEqual(1);
    const [third, reached] = retryConnect(dialler(2), 5);
    expect(reached).toBeTruthy();
    expect(third).toEqual(3);
    const [last, silent] = retryConnect(dialler(9), 2);
    expect(silent).toBeFalsy();
    expect(last).toEqual(2);
  });
});
