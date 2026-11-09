import { advance, button, capture, click, expect, expectText, fill, matrix, recipe, suite } from "../tools/recipes/src/index.ts";
import { demoVault, viewports } from "./fixtures/vault.ts";
export default suite(matrix(viewports, viewport => recipe(`transfer-${viewport.name}`, {
  state: demoVault({ screen: "transfer" }), viewport,
  steps: [expect(button("Close import / export")).exists(), expect(button("Export vault to file")).enabled(false), advance(250), capture("initial"),
    fill("Export password", "synthetic-export-password"), fill("Confirm export password", "synthetic-export-password"),
    expect(button("Export vault to file")).enabled(), advance(200), capture("encrypted-ready"),
    click({ role: "combobox", label: "Export format" }), advance(200), click({ role: "option", label: "JSON (unencrypted)" }), advance(200),
    expectText("This file will contain your passwords in plain text."), expect(button("Export vault to file")).enabled(false),
    click({ role: "checkbox", label: "Export without encryption" }), expect(button("Export vault to file")).enabled(), capture("plaintext-choice"),
  ],
})));
