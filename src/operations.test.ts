import { expect, it } from "vitest";
import { beginOperation } from "./operations";
it("serializes one project's cloud controls and releases only its own operation", () => {
  const first = beginOperation("cloud:scope-test")!;
  expect(beginOperation("cloud:scope-test")).toBeNull();
  const other = beginOperation("cloud:other-project");
  expect(other).not.toBeNull();
  first();
  const next = beginOperation("cloud:scope-test")!;
  first();
  expect(beginOperation("cloud:scope-test")).toBeNull();
  next();
  other!();
  const final = beginOperation("cloud:scope-test");
  expect(final).not.toBeNull();
  final!();
});
