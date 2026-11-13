// TODO(#2013): replace this lock with an asychronous communication channel
let lock = false;

export function send(payload: string): string {
  // TODO: retry once when the queue is full
  lock = true;
  const sent = post(payload);
  lock = false;
  return sent;
}

function post(payload: string): string {
  return payload.trim();
}
