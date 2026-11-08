export interface VaultFixture {
  screen: "ssh-approval" | "vault" | "settings" | "login" | "editor" | "generator";
  query: string;
  selected: string;
  filter: string;
  opaque: boolean;
}
/** New object on every call; no live SDK, user account, preferences or credentials. */
export function demoVault(overrides: Partial<VaultFixture> = {}): VaultFixture {
  return { screen: "vault", query: "", selected: "github", filter: "All Items", opaque: false, ...overrides };
}
export const viewports = [
  { name: "desktop", width: 1440, height: 1000, scale: 1 },
  { name: "compact", width: 820, height: 640, scale: 1 },
] as const;
