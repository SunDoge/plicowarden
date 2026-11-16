import { advance, button, capture, click, expect, matrix, recipe, suite } from "../tools/recipes/src/index.ts";
import { demoVault, viewports } from "./fixtures/vault.ts";
export default suite(viewports.flatMap(viewport => [
  recipe(`move-single-${viewport.name}`, { state: demoVault({ screen: "folders" }), viewport, steps: [
    capture("item-actions"), click(button("Move to folder")), advance(250), capture("move-dialog"),
    click({ role: "combobox", label: "Destination folder" }), advance(200), click({ role: "option", label: "No folder" }), advance(250),
    click(button("Move selected items")), advance(250), expect(button("Move to folder")).enabled(false),
  ]}),
  recipe(`move-bulk-${viewport.name}`, { state: demoVault({ screen: "folders" }), viewport, steps: [
    click(button("Bulk selection")), click(button("Toggle bulk selection for GitHub")), click(button("Toggle bulk selection for Linear")),
    click(button("Move selected to folder")), advance(250), capture("move-dialog"),
    click({ role: "combobox", label: "Destination folder" }), advance(200), click({ role: "option", label: "Personal" }), advance(250),
    capture("destination"), click(button("Move selected items")),
  ]}),
  recipe(`unfiled-${viewport.name}`, { state: demoVault({ screen: "folders" }), viewport, steps: [
    ...(viewport.name === "compact" ? [click(button("Toggle sidebar"))] : []),
    click(button("No folder")), expect(button("Select GitHub")).absent(), expect(button("Select Linear")).exists(), capture("unfiled"),
  ]}),
]));
