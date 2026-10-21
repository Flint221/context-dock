import { render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { Modal } from "./Modal";

it("focuses the designated field after opening and restores the prior control on close", () => {
  const opener = document.createElement("button");
  opener.textContent = "Open editor";
  document.body.append(opener);
  opener.focus();
  const view = render(
    <Modal title="Edit project" close={vi.fn()}>
      <label>
        Project name
        <input data-autofocus />
      </label>
    </Modal>,
  );
  expect(screen.getByRole("textbox", { name: "Project name" })).toHaveFocus();
  view.unmount();
  expect(opener).toHaveFocus();
  opener.remove();
});
