import { advance, button, buttonsStartingWith, capture, click, expect, expectText, fill, flow, focus, matrix, recipe, suite, textbox } from "../tools/recipes/src/index.ts";
import { demoVault, viewports } from "./fixtures/vault.ts";
import { accountError } from "./fixtures/events.ts";
import { search } from "./flows/search.ts";

export default suite([
  ...matrix(viewports, viewport => recipe(`search-${viewport.name}`, {
    state: demoVault(), viewport,
    steps: [
      expect(buttonsStartingWith("Select ")).count(12),
      capture("unfocused"),
      ...search("github"),
      expect(button("Select GitHub")).exists(),
      expect(button("Select Linear")).absent(),
      expect(textbox("Search all vaults")).value("github"),
      capture("search-github"),
      ...search(""),
      expect(buttonsStartingWith("Select ")).count(12),
      capture("search-cleared"),
      focus(button("Reverse sort")),
      capture("blurred"),
    ],
  })),
  recipe("favorites", {
    state: demoVault(),
    steps: [click(button("Favorites")), expect(buttonsStartingWith("Select ")).count(4), capture("favorites"),
      click(button("All Items")), expect(buttonsStartingWith("Select ")).count(12)],
  }),
  recipe("settings", {
    state: demoVault(),
    steps: [
      ...flow("open-settings", [click(button("Settings")), expect(button("Close settings")).exists()]),
      click({ role: "switch", label: "Liquid Glass" }), capture("glass-disabled"),
      click(button("Reset appearance")), capture("defaults"),
      click(button("Close settings")), expect(button("Close settings")).absent(),
    ],
  }),
  recipe("login-rejected", {
    state: demoVault(),
    steps: [
      click(button("Connect account")), expect(button("Sign in")).enabled(false),
      fill("Email address", "recipe@example.invalid"), fill("Master password", "synthetic-recipe-password"),
      expect(button("Sign in")).enabled(), click(button("Sign in")),
      expect(button("Sign in")).enabled(false),
      advance(250),
      accountError("Synthetic account service is offline"),
      expectText("Synthetic account service is offline"),
      expect(button("Sign in")).enabled(false), capture("rejected"),
      click(button("Browse demo vault")), expect(buttonsStartingWith("Select ")).count(12),
    ],
  }),
  recipe("editor", {
    state: demoVault({ screen: "editor" }),
    steps: [expect(button("Cancel editing")).exists(), expect(button("Save item")).enabled(false), capture("new-item"), click(button("Cancel editing")), expect(button("Cancel editing")).absent()],
  }),
  recipe("generator", {
    state: demoVault({ screen: "generator" }),
    steps: [expect(button("Generate password")).exists(), capture("initial"),
      click(button("Passphrase")), expect(button("Generate passphrase")).exists(),
      capture("passphrase"), click(button("Username")),
      expect(button("Generate username")).exists(), capture("username"),
      click(button("Password")), expect(button("Generate password")).exists()],
  }),
  recipe("ssh-approval", {
    state: demoVault({ screen: "ssh-approval" }),
    steps: [expect(button("Allow SSH signature once")).exists(), advance(200), capture("approval"),
      click(button("Deny SSH signature")), expect(button("Allow SSH signature once")).absent(), capture("denied")],
  }),
]);
