import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { DocumentHistory } from "./DocumentHistory";
import { registry } from "../services/registry";
import type { Family, Version } from "../types";
import type { Comparison } from "../comparison";
vi.mock("../services/registry", () => ({
  registry: {
    versions: vi.fn(),
    compare: vi.fn(),
    openCurrent: vi.fn(),
    openVersion: vi.fn(),
  },
}));
const family: Family = {
  id: "doc",
  name: "Master Context",
  kind: "markdown",
  state: "tracked",
  activeSourceId: "source",
  activeFilename: "Master.md",
  sourceStatus: "available",
  currentNumber: 3,
  versionCount: 3,
  lastCaptured: null,
  needsReview: false,
};
const versions: Version[] = [3, 2, 1].map((number) => ({
  id: `v${number}`,
  number,
  sourceName: `Master_v${number}.md`,
  sourceRelative: `Master_v${number}.md`,
  capturedAt: "2026-10-19T12:00:00Z",
  modifiedAt: null,
  size: 10,
  sha256: String(number).repeat(64),
  reason: "file_modified",
  snapshotPath: `C:\\Store\\v${number}.md`,
  archivePath: null,
  snapshotAvailable: true,
  archiveAvailable: false,
  current: number === 3,
}));
let result: Comparison;
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(registry.versions).mockResolvedValue(versions);
  const version = (i: number) => ({
    id: `v${i}`,
    documentId: "doc",
    number: i,
    sourceName: `Master_v${i}.md`,
    capturedAt: "2026-10-19T12:00:00Z",
    sha256: String(i).repeat(64),
  });
  result = {
    formatVersion: 1,
    documentId: "doc",
    documentName: "Master Context",
    kind: "markdown",
    earlier: version(2),
    later: version(3),
    bytesIdentical: false,
    textEquivalent: false,
    addedLines: 2,
    removedLines: 2,
    changedBlocks: 2,
    warnings: ["CRLF normalized"],
    blocks: [
      {
        id: "b1",
        kind: "change",
        oldStart: 1,
        oldCount: 1,
        newStart: 1,
        newCount: 1,
        lines: [
          {
            kind: "remove",
            oldLine: 1,
            newLine: null,
            text: "Old goal",
            terminated: true,
          },
          {
            kind: "add",
            oldLine: null,
            newLine: 1,
            text: "New goal",
            terminated: true,
          },
        ],
      },
      {
        id: "e1",
        kind: "equal",
        oldStart: 2,
        oldCount: 1,
        newStart: 2,
        newCount: 1,
        lines: [
          {
            kind: "equal",
            oldLine: 2,
            newLine: 2,
            text: "Context",
            terminated: true,
          },
        ],
      },
      {
        id: "b2",
        kind: "change",
        oldStart: 3,
        oldCount: 1,
        newStart: 3,
        newCount: 1,
        lines: [
          {
            kind: "remove",
            oldLine: 3,
            newLine: null,
            text: "Planning",
            terminated: false,
          },
          {
            kind: "add",
            oldLine: null,
            newLine: 3,
            text: "Development",
            terminated: true,
          },
        ],
      },
    ],
  };
  vi.mocked(registry.compare).mockImplementation(async () =>
    structuredClone(result),
  );
  Element.prototype.scrollIntoView = vi.fn();
});
async function open() {
  const user = userEvent.setup();
  const view = render(
    <DocumentHistory key={family.id} family={family} close={vi.fn()} />,
  );
  await user.click(
    await screen.findByRole("button", { name: "Compare Versions" }),
  );
  return { user, ...view };
}
it("defaults to the latest pair, selects nonconsecutive versions and navigates both modes", async () => {
  const { user } = await open();
  expect(screen.getByLabelText("Earlier version")).toHaveValue("v2");
  expect(screen.getByLabelText("Later version")).toHaveValue("v3");
  await user.selectOptions(screen.getByLabelText("Earlier version"), "v1");
  await user.click(
    screen.getByRole("button", { name: "Compare selected versions" }),
  );
  await screen.findByText("New goal");
  expect(registry.compare).toHaveBeenCalledWith("v1", "v3");
  expect(screen.getByText("+2 lines added")).toBeInTheDocument();
  expect(screen.getByText("Change 1 of 2")).toBeInTheDocument();
  await user.selectOptions(screen.getByLabelText("Display mode"), "split");
  expect(screen.getByText("Earlier · v2")).toBeInTheDocument();
  expect(screen.getByText("Later · v3")).toBeInTheDocument();
  const content = screen.getByRole("region", { name: "Comparison content" });
  content.scrollTop = 500;
  await user.click(screen.getByRole("button", { name: "Next change" }));
  expect(content.scrollTop).toBe(0);
  expect(screen.getByText("Change 2 of 2")).toBeInTheDocument();
  expect(screen.getByText("Development")).toBeInTheDocument();
  expect(Element.prototype.scrollIntoView).toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Previous change" }));
  expect(screen.getByText("New goal")).toBeInTheDocument();
});
it("clears loading after failure, allows retry and distinguishes equivalent text from bytes", async () => {
  const { user } = await open();
  vi.mocked(registry.compare).mockRejectedValueOnce(
    "Snapshot SHA-256 integrity mismatch",
  );
  await user.click(
    screen.getByRole("button", { name: "Compare selected versions" }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "integrity mismatch",
  );
  expect(
    screen.getByRole("button", { name: "Compare selected versions" }),
  ).toBeEnabled();
  result = {
    ...result,
    textEquivalent: true,
    addedLines: 0,
    removedLines: 0,
    changedBlocks: 0,
    blocks: [],
  };
  await user.selectOptions(screen.getByLabelText("Earlier version"), "v3");
  expect(screen.getByText(/same captured version/)).toBeInTheDocument();
  await user.click(
    screen.getByRole("button", { name: "Compare selected versions" }),
  );
  await screen.findByText(/No textual changes/);
  expect(screen.getByText("File bytes differ")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Next change" })).toBeDisabled();
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});
it("keeps one dialog across repeated history transitions and ignores responses after family changes", async () => {
  const { user, rerender } = await open();
  for (let i = 0; i < 3; i++) {
    expect(screen.getAllByRole("dialog")).toHaveLength(1);
    await user.click(
      screen.getByRole("button", { name: /Back to version history/ }),
    );
    expect(
      screen.getByRole("button", { name: "Compare Versions" }),
    ).toHaveFocus();
    await user.click(screen.getByRole("button", { name: "Compare Versions" }));
  }
  let finish!: (r: Comparison) => void;
  vi.mocked(registry.compare).mockReturnValueOnce(
    new Promise((resolve) => {
      finish = resolve;
    }),
  );
  await user.click(
    screen.getByRole("button", { name: "Compare selected versions" }),
  );
  expect(screen.getByRole("button", { name: "Comparing…" })).toBeDisabled();
  vi.mocked(registry.versions).mockResolvedValueOnce([versions[2]]);
  rerender(
    <DocumentHistory
      key="other"
      family={{
        ...family,
        id: "other",
        name: "Technical Documentation",
        versionCount: 1,
      }}
      close={vi.fn()}
    />,
  );
  await screen.findByText(/Capture a second version/);
  await act(async () => finish(result));
  expect(screen.queryByText("New goal")).not.toBeInTheDocument();
  expect(screen.getAllByRole("dialog")).toHaveLength(1);
  expect(registry.versions).toHaveBeenLastCalledWith("other");
});
it("bounds rendered changed lines and exposes every page without truncating", async () => {
  const { user } = await open();
  result.blocks = [
    {
      id: "big",
      kind: "change",
      oldStart: 1,
      oldCount: 0,
      newStart: 1,
      newCount: 350,
      lines: Array.from({ length: 350 }, (_, i) => ({
        kind: "add",
        oldLine: null,
        newLine: i + 1,
        text: `Added ${i + 1}`,
        terminated: true,
      })),
    },
  ];
  result.changedBlocks = 1;
  result.addedLines = 350;
  result.removedLines = 0;
  await user.click(
    screen.getByRole("button", { name: "Compare selected versions" }),
  );
  await screen.findByText("Added 1");
  expect(document.querySelectorAll(".unified-line")).toHaveLength(100);
  expect(screen.queryByText("Added 101")).not.toBeInTheDocument();
  const content = screen.getByRole("region", { name: "Comparison content" });
  content.scrollTop = 500;
  for (let i = 0; i < 3; i++)
    await user.click(screen.getByRole("button", { name: "Next lines" }));
  expect(content.scrollTop).toBe(0);
  expect(screen.getByText("Added 350")).toBeInTheDocument();
  expect(document.querySelectorAll(".unified-line")).toHaveLength(50);
  expect(screen.getByRole("button", { name: "Next lines" })).toBeDisabled();
});
it("reports version-loading failure without a permanent spinner", async () => {
  vi.mocked(registry.versions).mockRejectedValueOnce("History unavailable");
  render(<DocumentHistory family={family} close={vi.fn()} />);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "History unavailable",
  );
  await waitFor(() =>
    expect(
      screen.queryByText("Loading saved versions…"),
    ).not.toBeInTheDocument(),
  );
  const user = userEvent.setup();
  await user.click(
    screen.getByRole("button", { name: "Reload saved versions" }),
  );
  expect(
    await screen.findByRole("button", { name: "Compare Versions" }),
  ).toBeEnabled();
});
