import { expect, it } from "vitest";
import { displayPath, versionName } from "./format";
it("formats canonical drive and UNC paths for Explorer and clipboard", () => {
  expect(displayPath("\\\\?\\C:\\Projects\\Test")).toBe("C:\\Projects\\Test");
  expect(displayPath("\\\\?\\UNC\\server\\share")).toBe("\\\\server\\share");
  expect(displayPath("C:\\Projects")).toBe("C:\\Projects");
});

it("shows the author filename version rather than the saved revision counter", () => {
  expect(versionName("RAP_Master_Context_v4.docx", 97)).toBe("v4");
  expect(versionName("Notes_version_4.2_2026-10-22.md", 98)).toBe("v4.2");
  expect(versionName("Working.md", 99)).toBe("Saved revision 99");
});
