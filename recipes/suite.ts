import { advance, button, buttonsStartingWith, capture, click, expect, expectText, fill, flow, focus, matrix, recipe, suite, textbox } from "../tools/recipes/src/index.ts";
import { demoVault, viewports } from "./fixtures/vault.ts";
import { accountError } from "./fixtures/events.ts";
import { search } from "./flows/search.ts";

// The default suite is a visual review, not a second behavioral test suite.
// Rust tests own state transitions and exhaustive interaction assertions. These
// recipes keep only enough semantic waypoints to prove each screenshot reached
// the intended surface.
export default suite([
  ...matrix(viewports, viewport => recipe(`search-${viewport.name}`, {
    state: demoVault(), viewport,
    steps: [
      expect(button("Select GitHub")).exists(), capture("unfocused"),
      ...search("github"), expect(textbox("Search all vaults")).value("github"), capture("search-github"),
      ...search(""), capture("search-cleared"),
      focus(button("Reverse sort")), capture("blurred"),
    ],
  })),
  recipe("favorites-motion", {
    state: demoVault(), viewport: viewports[0],
    steps: [
      click(button("Favorites")), expect(buttonsStartingWith("Select ")).count(4),
      capture("start"), advance(75), capture("transition"), advance(175), capture("settled"),
    ],
  }),
  recipe("settings-sheet", {
    state: demoVault(), viewport: viewports[0],
    steps: [
      ...flow("open-settings", [click(button("Settings")), expect(button("Close settings")).exists()]),
      capture("appearance"), click(button("Account")), expect(button("Manage accounts")).exists(), capture("account"),
    ],
  }),
  recipe("command-palette", {
    state: demoVault(), viewport: viewports[0],
    steps: [
      click(button("Search or run a command")), capture("open"),
      fill("Search vault or commands", "github"), expect(button("Open GitHub")).exists(), capture("filtered"),
    ],
  }),
  recipe("folder-motion", {
    state: demoVault({ screen: "folders" }), viewport: viewports[0],
    steps: [
      click(button("Work")), advance(100), capture("collapsing"), advance(150),
      expect(button("Work/Backend")).absent(), click(button("Work")), advance(250),
      click(button("Work/Backend")), expect(buttonsStartingWith("Select ")).count(1), capture("backend"),
    ],
  }),
  recipe("login-failure", {
    state: demoVault(), viewport: viewports[0],
    steps: [
      click(button("Connect account")), capture("email"),
      fill("Email address", "recipe@example.invalid"), click(button("Continue")), capture("password"),
      fill("Master password", "synthetic-recipe-password"), click(button("Sign in")),
      expect(button("Logging in…")).enabled(false), capture("connecting"), advance(250), capture("waiting"),
      accountError("Synthetic account service is offline"),
      expectText("Connecting and authenticating… Synthetic account service is offline"), capture("failed"),
    ],
  }),
  recipe("editor", {
    state: demoVault({ screen: "editor" }), viewport: viewports[0],
    steps: [expect(button("Cancel editing")).exists(), capture("new-item")],
  }),
  recipe("custom-titlebar", {
    state: demoVault({ custom_titlebar: true }), viewport: viewports[0],
    steps: [
      expect(button("Close window")).exists(), expect(button("Move window")).exists(), capture("sidebar"),
      click(button("Toggle sidebar")), capture("sidebar-hidden"),
    ],
  }),
]);
