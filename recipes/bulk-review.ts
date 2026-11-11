import { advance, button, capture, click, expect, matrix, recipe, suite } from "../tools/recipes/src/index.ts";
import { demoVault, viewports } from "./fixtures/vault.ts";
export default suite(matrix(viewports, viewport => recipe(`bulk-${viewport.name}`, {
  state: demoVault({ screen: "bulk" }), viewport,
  steps: [...(viewport.name === "compact"
      ? [click(button("Toggle sidebar")), expect(button("All Items")).exists(), capture("sidebar"), click(button("Back"))]
      : [expect(button("All Items")).exists(), capture("three-columns")]),
    click(button("Bulk selection")), click(button("Toggle bulk selection for GitHub")),
    click(button("Toggle bulk selection for Linear")), capture("selected"),
    click(button("Trash selected")), expect(button("Confirm bulk action")).exists(), capture("confirmation"),
    click(button("Cancel bulk action")), advance(250), click(button("Done selecting")),
    expect(button("Confirm bulk action")).absent()],
})));
