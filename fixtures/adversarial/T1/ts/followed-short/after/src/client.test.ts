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
    const [, answered] = retryConnect(dialler(2), 5);
    expect(answered).toBeTruthy();
  });
});
