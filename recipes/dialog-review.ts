import { advance, button, capture, click, expect, recipe, suite } from "../tools/recipes/src/index.ts";
import { demoVault, viewports } from "./fixtures/vault.ts";
export default suite(viewports.flatMap(viewport => [false, true].map(opaque => recipe(`dialog-${viewport.name}-${opaque ? "opaque" : "glass"}`, {
  state: demoVault({ screen: "ssh-approval", opaque }), viewport,
  steps: [advance(250), expect(button("Close dialog")).exists(), capture("open"),
    click(button("Close dialog")), expect(button("Allow SSH signature once")).enabled(false),
    advance(150), expect(button("Allow SSH signature once")).absent()],
}))));
