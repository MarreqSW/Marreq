import { AsyncLocalStorage } from "node:async_hooks";

const bearerStorage = new AsyncLocalStorage<string>();

export function withBearer<T>(token: string, fn: () => T): T {
  return bearerStorage.run(token, fn);
}

export function currentBearer(): string | undefined {
  return bearerStorage.getStore();
}
