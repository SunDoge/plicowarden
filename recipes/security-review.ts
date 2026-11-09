import { advance, button, capture, click, expect, recipe, suite } from "../tools/recipes/src/index.ts";
import { demoVault, viewports } from "./fixtures/vault.ts";

export default suite(viewports.map(viewport => recipe(`security-${viewport.name}`, {
  state: demoVault({ screen: "security" }), viewport,
  steps: [
    capture("initial"),
    click({ role: "combobox", label: "Auto-lock after" }), advance(200),
    click({ role: "option", label: "1 minute" }), advance(200),
    click({ role: "combobox", label: "Clear clipboard after" }), advance(200),
    click({ role: "option", label: "10 seconds" }), advance(200),
    click({ role: "switch", label: "Minimize after copying" }), advance(200), capture("configured"),
    click(button("Reset")), capture("reset"),
    click(button("Appearance")), expect(button("Save appearance")).exists(),
    click(button("SSH agent")), expect(button("Save changes")).absent(),
    click(button("Security")), expect(button("Save changes")).enabled(),
  ],
})));
