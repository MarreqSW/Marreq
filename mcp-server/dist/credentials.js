import { AsyncLocalStorage } from "node:async_hooks";
const bearerStorage = new AsyncLocalStorage();
export function withBearer(token, fn) {
    return bearerStorage.run(token, fn);
}
export function currentBearer() {
    return bearerStorage.getStore();
}
