export function send(payload: string): string {
  try {
    return post(payload);
  } catch (error) {
  }
  return "";
}

export function flush(): Promise<void> {
  return drain().catch(() => {});
}

function post(payload: string): string {
  return payload.trim();
}

function drain(): Promise<void> {
  return Promise.resolve();
}
