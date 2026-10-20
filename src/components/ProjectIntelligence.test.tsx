import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import {
  ai,
  MINI,
  type IntelligenceView,
  type Preview,
  type Report,
} from "../ai";
import { ProjectIntelligence } from "./ProjectIntelligence";
vi.mock("../ai", async (original) => ({
  ...(await original<typeof import("../ai")>()),
  ai: { action: vi.fn() },
}));
let view: IntelligenceView;
let preview: Preview;
let report: Report;
beforeEach(() => {
  vi.clearAllMocks();
  view = {
    baselines: [],
    versions: [1, 2, 3].map((n) => ({
      id: `v${n}`,
      document: "doc",
      number: n,
      capturedAt: "2026-10-19T12:00:00Z",
      name: "Master Context",
      current: n === 3 ? 1 : 0,
    })),
    reports: [],
  };
  preview = {
    id: "plan1",
    cached: null,
    requests: 1,
    retrieval: false,
    plan: {
      project: "p1",
      request: {
        kind: "compare",
        versions: ["v1", "v3"],
        baseline: null,
        model: MINI,
        inputBudget: 32768,
        outputTokens: 4096,
      },
      sources: [1, 3].map((n) => ({
        document: "doc",
        version: `v${n}`,
        number: n,
        name: "Master Context",
        capturedAt: "2026-10-19T12:00:00Z",
        hash: `hash${n}`,
      })),
      baseline: null,
      evidence: [
        {
          id: "e1",
          version: "v1",
          heading: "Goal",
          start: 1,
          end: 2,
          text: "# Goal\nOffline inventory.",
        },
      ],
      transitions: [{ earlier: "v1", later: "v3", blocks: [] }],
      warnings: [],
      scope: "Complete selected evidence",
      promptVersion: 1,
      schemaVersion: 1,
      inputUnits: 4000,
      estimatedTokens: 1000,
      estimateUsd: 0.02,
      conservativeUsd: 0.03,
      pricingDate: "2026-10-19",
      cacheKey: "cache",
    },
  };
  report = {
    id: "report1",
    project_id: "p1",
    kind: "compare",
    state: "completed",
    created_at: "2026-10-19T12:00:00Z",
    completed_at: "2026-10-19T12:00:00Z",
    error: null,
    plan: preview.plan,
    request: preview.plan.request,
    result: {
      summary: "Reporting is introduced.",
      summaryRefs: [{ evidenceId: "e1", quote: "Offline inventory." }],
      findings: [
        {
          category: "expanded_scope",
          title: "Reporting",
          detail: "An interpretation to review.",
          atVersion: null,
          refs: [{ evidenceId: "e1", quote: "Offline inventory." }],
        },
      ],
    },
    usage: { input_tokens: 1000, output_tokens: 150 },
    response_id: "resp_synthetic",
    decisions: [],
  };
  vi.mocked(ai.action).mockImplementation(async (_p, action) => {
    if (action === "view") return structuredClone(view) as never;
    if (action === "prepare") return structuredClone(preview) as never;
    if (action === "report" || action === "generate")
      return structuredClone(report) as never;
    return undefined as never;
  });
});
async function open() {
  render(
    <ProjectIntelligence
      project="p1"
      name="Disposable test"
      initialPair={["v1", "v3"]}
    />,
  );
  await screen.findByText(/Approved baseline:/);
}
it("loads local metadata and retains an exact nonconsecutive pair until an explicit paid confirmation", async () => {
  const user = userEvent.setup();
  await open();
  expect(ai.action).toHaveBeenCalledExactlyOnceWith("p1", "view");
  await user.click(
    screen.getByRole("button", { name: /Preview analysis plan/ }),
  );
  expect(ai.action).toHaveBeenCalledWith("p1", "prepare", {
    request: expect.objectContaining({ versions: ["v1", "v3"] }),
  });
  const panel = screen.getByRole("region", {
    name: "Analysis request preview",
  });
  const generate = within(panel).getByRole("button", {
    name: "Generate Analysis · paid API",
  });
  expect(generate).toBeDisabled();
  expect(within(panel).getByText(/1 model request/)).toBeInTheDocument();
  await user.click(within(panel).getByLabelText(/I approve sending/));
  await user.click(generate);
  expect(ai.action).toHaveBeenCalledWith("p1", "generate", {
    id: "plan1",
    consent: true,
    fresh: false,
  });
  expect(
    await screen.findByText("Reporting is introduced."),
  ).toBeInTheDocument();
  expect(screen.getByText(/1000 input tokens/)).toBeInTheDocument();
});
it("offers an exact cached report without requesting generation or credentials", async () => {
  const user = userEvent.setup();
  preview.cached = "report1";
  await open();
  await user.click(
    screen.getByRole("button", { name: /Preview analysis plan/ }),
  );
  await user.click(
    screen.getByRole("button", { name: "View Saved Analysis · no API call" }),
  );
  expect(
    await screen.findByText("Reporting is introduced."),
  ).toBeInTheDocument();
  expect(
    vi.mocked(ai.action).mock.calls.every((c) => c[1] !== "generate"),
  ).toBe(true);
});
it("creates an explicitly approved manual baseline without a model request", async () => {
  const user = userEvent.setup();
  await open();
  await user.click(screen.getByRole("button", { name: "Create baseline" }));
  await user.type(screen.getByLabelText("Baseline title"), "MVP");
  await user.type(
    screen.getByLabelText("Project purpose"),
    "Offline inventory",
  );
  const approve = screen.getByRole("button", { name: "Approve baseline" });
  expect(approve).toBeDisabled();
  await user.click(screen.getByLabelText(/I approve these objectives/));
  await user.click(approve);
  expect(ai.action).toHaveBeenCalledWith(
    "p1",
    "approve",
    expect.objectContaining({
      title: "MVP",
      approve: true,
      sourceVersion: null,
      supersedes: null,
      content: expect.objectContaining({ purpose: "Offline inventory" }),
    }),
  );
  expect(
    vi.mocked(ai.action).mock.calls.every((c) => c[1] !== "generate"),
  ).toBe(true);
});
it("cancels an unsent plan without sending a paid request", async () => {
  const user = userEvent.setup();
  await open();
  await user.click(
    screen.getByRole("button", { name: /Preview analysis plan/ }),
  );
  await user.click(screen.getByRole("button", { name: "Cancel analysis" }));
  await waitFor(() =>
    expect(
      screen.queryByRole("region", { name: "Analysis request preview" }),
    ).not.toBeInTheDocument(),
  );
  expect(ai.action).toHaveBeenCalledWith("p1", "cancel", { id: "plan1" });
  expect(
    vi.mocked(ai.action).mock.calls.every((c) => c[1] !== "generate"),
  ).toBe(true);
});
it("unlocks a failed generation and retains its actionable error without an automatic retry", async () => {
  const user = userEvent.setup();
  const original = vi.mocked(ai.action).getMockImplementation()!;
  vi.mocked(ai.action).mockImplementation(async (...args) => {
    if (args[1] === "generate")
      throw "Network request outcome uncertain; review usage before retrying.";
    return original(...args);
  });
  await open();
  await user.click(
    screen.getByRole("button", { name: /Preview analysis plan/ }),
  );
  await user.click(screen.getByLabelText(/I approve sending/));
  await user.click(
    screen.getByRole("button", { name: "Generate Analysis · paid API" }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "outcome uncertain",
  );
  expect(screen.getByRole("button", { name: "Cancel analysis" })).toBeEnabled();
  expect(
    vi.mocked(ai.action).mock.calls.filter((c) => c[1] === "generate"),
  ).toHaveLength(1);
});
it("ignores a late old-project result after unmount", async () => {
  let finish: (v: IntelligenceView) => void = () => {};
  vi.mocked(ai.action).mockReturnValueOnce(
    new Promise((resolve) => {
      finish = resolve as typeof finish;
    }),
  );
  const old = render(<ProjectIntelligence project="old" name="Old project" />);
  old.unmount();
  render(<ProjectIntelligence project="new" name="New project" />);
  await screen.findByText(/Approved baseline:/);
  await act(async () =>
    finish({
      ...view,
      versions: [],
      baselines: [
        {
          id: "foreign",
          title: "Foreign baseline",
          revision: 1,
          content: "{}",
          supersedes: null,
          source_version: null,
          source_hash: null,
          created_at: "",
          approved_at: "",
          active: 1,
        },
      ],
    }),
  );
  expect(screen.queryByText(/Foreign baseline/)).not.toBeInTheDocument();
  expect(
    screen.getAllByRole("heading", { name: "Project intelligence" }),
  ).toHaveLength(1);
});
