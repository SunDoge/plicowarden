export interface VaultFixture {
  screen: "accounts" | "folders" | "transfer" | "bulk" | "ssh-approval" | "login-approval" | "ssh-history" | "vault" | "settings" | "security" | "login" | "editor" | "generator" | "card" | "ssh-key" | "totp" | "fields" | "empty" | "notifications" | "many-folders";
  query: string;
  selected: string;
  filter: string;
  /** Legacy preview input; chrome material selection no longer varies by recipe. */
  opaque: boolean;
  custom_titlebar?: boolean;
}
/** New object on every call; no live SDK, user account, preferences or credentials. */
export function demoVault(overrides: Partial<VaultFixture> = {}): VaultFixture {
  return { screen: "vault", query: "", selected: "github", filter: "All Items", opaque: false, custom_titlebar: true, ...overrides };
}
export const viewports = [
  { name: "desktop", width: 1440, height: 1000, scale: 1 },
  { name: "compact", width: 820, height: 640, scale: 1 },
] as const;
