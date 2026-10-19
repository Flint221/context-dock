import { expect, it } from "vitest";
import { displayPath } from "./format";
it("formats canonical drive and UNC paths for Explorer and clipboard", () => {
  expect(displayPath("\\\\?\\C:\\Projects\\Test")).toBe("C:\\Projects\\Test");
  expect(displayPath("\\\\?\\UNC\\server\\share")).toBe("\\\\server\\share");
  expect(displayPath("C:\\Projects")).toBe("C:\\Projects");
});
