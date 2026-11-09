import { advance, button, capture, click, expect, recipe, suite } from "../tools/recipes/src/index.ts";
import { demoVault } from "./fixtures/vault.ts";

export default suite([false, true].map(opaque => recipe(`dialog-motion-${opaque ? "opaque" : "glass"}`, {
  state: demoVault({ screen: "ssh-approval", opaque }),
  viewport: { width: 1120, height: 820, scale: 1 },
  steps: [
    expect(button("Close dialog")).exists(), capture("000"),
    advance(16), capture("016"), advance(17), capture("033"),
    advance(17), capture("050"), advance(30), capture("080"),
    advance(60), capture("140"), advance(100), capture("240"),
    click(button("Close dialog")),
    expect(button("Allow SSH signature once")).enabled(false), capture("close-000"),
    advance(16), capture("close-016"), advance(34), capture("close-050"),
    expect(button("Allow SSH signature once")).enabled(false),
    advance(90), capture("close-140"),
    expect(button("Allow SSH signature once")).absent(),
  ],
})));
