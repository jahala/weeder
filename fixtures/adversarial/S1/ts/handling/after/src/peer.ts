import { NotImplemented } from "./shared";

export function answer(code: number): string {
  return String(code);
}

export function reply(error: unknown): string {
  if (error instanceof NotImplemented) {
    return "the peer has not written this method";
  }
  return "failed";
}

export function guarded(run: () => string): string {
  try {
    return run();
  } catch (error: unknown) {
    if (error instanceof NotImplemented) {
      return reply(error);
    }
    throw error;
  }
}

export function resume(id: string): string {
  throw new NotImplemented(id);
}
