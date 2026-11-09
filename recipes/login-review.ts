import { advance, button, capture, click, event, expect, expectText, fill, key, matrix, recipe, suite, textbox } from "../tools/recipes/src/index.ts";
import { demoVault, viewports } from "./fixtures/vault.ts";

export default suite(matrix(viewports, viewport => recipe(`login-${viewport.name}`, {
  state: demoVault({ screen: "login", custom_titlebar: true }), viewport,
  steps: [capture("email"), click(button("bitwarden.com")), click(button("Self-hosted")),
    fill("Server URL", "http://example.invalid"), click(button("Done")),
    expectText("Use HTTPS (local HTTP is allowed only in debug builds)."), capture("invalid-server"),
    fill("Server URL", "https://vault.example.invalid"), capture("self-hosted"), click(button("Done")),
    fill("Email address", "recipe@example.invalid"), click(button("Continue")),
    fill("Master password", "synthetic-preview-password"), advance(200), capture("password"),
    click(button("Back")), expect(textbox("Email address")).value("recipe@example.invalid"),
    click(button("Continue")), expect(button("Sign in")).enabled(false),
    fill("Master password", "synthetic-preview-password"), key("Enter"),
    event({ kind: "two_factor_required" }), capture("authenticator"),
    fill("Verification code", "123456"), key("Enter"),
    event({ kind: "account_error", message: "The verification code is incorrect. Try again." }),
    expect(button("Verify")).enabled(false), capture("code-rejected"),
    advance(200), click({ role: "combobox", label: "Verification method" }), advance(200), click({ role: "option", label: "Email" }),
    expect(button("Send email code")).exists(), advance(200), capture("email-verification"),
    advance(200), click({ role: "combobox", label: "Verification method" }), advance(200), click({ role: "option", label: "Security key" }),
    expect(textbox("Verification code")).absent(), expect(button("Verify")).absent(), advance(200), capture("unsupported-method"),
    click(button("Back")), expect(textbox("Email address")).value("recipe@example.invalid")],
})));
