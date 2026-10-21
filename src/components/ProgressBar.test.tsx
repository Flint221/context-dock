import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { ProgressBar } from "./ProgressBar";
import { OperationStatus } from "./OperationStatus";

it("keeps unknown work indeterminate and exposes measured progress as it arrives", () => {
  const view = render(<ProgressBar label="Indexing versions" />);
  expect(screen.getByRole("progressbar")).not.toHaveAttribute("aria-valuenow");
  view.rerender(
    <ProgressBar
      label="Indexing versions"
      value={40}
      detail="2 of 5 indexed"
    />,
  );
  expect(
    screen.getByRole("progressbar", { name: "Indexing versions" }),
  ).toHaveAttribute("aria-valuenow", "40");
  expect(screen.getByRole("progressbar")).toHaveAttribute(
    "aria-valuetext",
    "2 of 5 indexed",
  );
  view.rerender(
    <ProgressBar
      label="Indexing versions"
      value={80}
      detail="4 of 5 indexed"
    />,
  );
  expect(screen.getByRole("progressbar")).toHaveAttribute(
    "aria-valuenow",
    "80",
  );
});
it("never exposes invalid or out-of-range percentages to assistive technology", () => {
  const view = render(<ProgressBar label="Work" value={NaN} />);
  expect(screen.getByRole("progressbar")).not.toHaveAttribute("aria-valuenow");
  view.rerender(<ProgressBar label="Work" value={Infinity} />);
  expect(screen.getByRole("progressbar")).not.toHaveAttribute("aria-valuenow");
  view.rerender(<ProgressBar label="Work" value={120} />);
  expect(screen.getByRole("progressbar")).toHaveAttribute(
    "aria-valuenow",
    "100",
  );
  view.rerender(<ProgressBar label="Work" value={-10} />);
  expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "0");
});
it("shows completion only after confirmation and removes feedback when the action returns to idle", () => {
  const view = render(
    <OperationStatus
      busy
      message="Saving settings…"
      success="Settings saved."
    />,
  );
  expect(screen.getByRole("status")).toHaveTextContent("Saving settings…");
  expect(screen.getByRole("progressbar")).not.toHaveAttribute("aria-valuenow");
  view.rerender(
    <OperationStatus
      busy={false}
      message="Saving settings…"
      success="Settings saved."
    />,
  );
  expect(screen.getByRole("status")).toHaveTextContent("Settings saved.");
  expect(screen.getByRole("progressbar")).toHaveAttribute(
    "aria-valuenow",
    "100",
  );
  view.rerender(<OperationStatus busy={false} message="Saving settings…" />);
  expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  expect(screen.queryByRole("status")).not.toBeInTheDocument();
});
