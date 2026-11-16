import { event } from "../../tools/recipes/src/index.ts";
export const accountError = (message: string) => event({ kind: "account_error", message });
export const lockVault = () => event({ kind: "locked" });
