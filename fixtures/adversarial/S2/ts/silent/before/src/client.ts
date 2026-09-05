export function send(payload: string): string {
  return post(payload);
}

export function flush(): Promise<void> {
  return drain();
}

function post(payload: string): string {
  return payload.trim();
}

function drain(): Promise<void> {
  return Promise.resolve();
}
