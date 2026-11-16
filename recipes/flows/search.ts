import { fill, flow } from "../../tools/recipes/src/index.ts";
export const search = (query: string) => flow("search-vault", [fill("Search all vaults", query)]);
