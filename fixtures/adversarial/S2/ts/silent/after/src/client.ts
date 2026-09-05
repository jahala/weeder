export function send(payload: string): string {
  try {
    return post(payload);
  } catch (error) {
    report("send failed", error);
    return "";
  }
}

export function load(name: string): string {
  try {
    return post(name);
  } catch (error) {
    throw new Error(`load ${name} failed`, { cause: error });
  }
}

export function flush(): Promise<void> {
  return drain().catch((error) => {
    // the queue empties again on the next tick, and the caller has moved on
  });
}

function post(payload: string): string {
  return payload.trim();
}

function drain(): Promise<void> {
  return Promise.resolve();
}

function report(what: string, error: unknown): void {
  console.error(what, error);
}
