/** Dials until the server answers or the tries run out, and says how many tries it took and whether it connected. */
export function retryConnect(dial: () => boolean, tries: number): [number, boolean] {
  for (let attempt = 1; attempt <= tries; attempt++) {
    if (dial()) {
      return [attempt, true];
    }
  }
  return [tries, false];
}

/** The client talks to the server. */
export class Client {
  connect(dial: () => boolean): boolean {
    return retryConnect(dial, 3)[1];
  }
}
