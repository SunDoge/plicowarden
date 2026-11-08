import { test } from "bun:test";
import assert from "node:assert/strict";
import { advance, button, capture, click, expect, fill, flow, matrix, recipe, suite, validate } from "../src/index.ts";
import warden from "../../../recipes/suite.ts";

test("real Warden imports and matrix expand into independent versioned recipes", () => {
  validate(warden);
  assert.equal(warden.cases.length, 8);
  assert.deepEqual(warden.cases.slice(0, 2).map(c => [c.name, c.viewport.width]), [["search-desktop", 1440], ["search-compact", 820]]);
  assert.equal(warden.cases[0].steps.filter(s => s.operation.op === "capture").length, 4);
  const step = warden.cases[0].steps.find(s => s.flow.includes("search-vault"))!;
  assert.ok(step.source?.file.endsWith("flows/search.ts"));
  assert.ok(step.source);
  assert.ok(step.source.line > 0);
  const cloned = structuredClone(warden);
  cloned.cases[0].state.query = "mutated";
  assert.equal(cloned.cases[1].state.query, "");
  assert.equal(warden.cases[0].state.query, "");
});
test("flow preserves nested names and authored source locations without aliasing", () => {
  const action = fill("Search", "hello");
  const steps = flow("outer", flow("inner", [action]));
  assert.deepEqual(steps[0].flow, ["outer", "inner"]);
  assert.equal(steps[0].source?.file, action.source?.file);
  steps[0].flow.push("changed");
  assert.deepEqual(action.flow, []);
});
test("invalid plans are rejected before output", () => {
  const make = (steps = [capture("view")]) => suite([recipe("case", { state: {}, steps })]);
  for (const plan of [
    suite([]),
    suite([recipe("../bad", {state: {}, steps: []})]),
    make([capture("duplicate"), capture("duplicate")]),
    make([capture("_failure")]),
    make([capture("../escape")]),
    make([advance(-1)]),
    make([expect(button("x")).count(-1)]),
    suite([recipe("case", {state: {}, viewport: {scale: 0}, steps: []})]),
    suite([recipe("case", {state: {number: NaN}, steps: []})]),
    suite([recipe("case", {state: {date: new Date()}, steps: []})]),
  ]) assert.throws(() => validate(plan));
  const one = make();
  assert.throws(() => validate(suite([one.cases[0], one.cases[0]])));
  assert.throws(() => click({role: "button", label_prefix: "ambiguous"}));
});
test("matrix copies its input before each builder", () => {
  const entries = [{name:"a", state: {count: 1}}];
  const result = matrix(entries, item => {
    item.state.count++;
    return recipe(item.name, {state: item.state, steps: []});
  });
  assert.equal(entries[0].state.count, 1);
  assert.equal(result[0].state.count, 2);
});
