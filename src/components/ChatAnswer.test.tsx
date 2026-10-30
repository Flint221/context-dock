import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { ChatAnswer } from "./ChatAnswer";

it("formats basic Markdown while preserving literal citations and code", () => {
  const { container } = render(
    <ChatAnswer
      text={
        "## Findings\n\n**Working** and *pending*. [1][4]\n\n- First\n- Second\n\n1. Check\n2. Retry\n\n> A quote\n\nUse `**literal**`.\n\n```text\n<script>example</script>\n**unchanged**\n```"
      }
    />,
  );
  expect(screen.getByRole("heading", { name: "Findings" })).toBeInTheDocument();
  expect(screen.getByText("Working").tagName).toBe("STRONG");
  expect(screen.getByText("pending").tagName).toBe("EM");
  expect(screen.getByText(/\[1\]\[4\]/)).toBeInTheDocument();
  expect(screen.getAllByRole("listitem")).toHaveLength(4);
  expect(container.querySelector("blockquote")).toHaveTextContent("A quote");
  expect(screen.getByText("**literal**").tagName).toBe("CODE");
  expect(container.querySelector("pre code")).toHaveTextContent(
    "<script>example</script> **unchanged**",
  );
  expect(container.querySelector("script")).toBeNull();
});

it("renders HTML as text and never creates links or remote image elements", () => {
  const { container } = render(
    <ChatAnswer
      text={
        '<img src="https://example.com/track" onerror="alert(1)">\n\n<script>alert(1)</script>\n\n[Website](https://example.com) ![Image description](https://example.com/image.png) [Unsafe](javascript:alert%281%29) <https://example.com>'
      }
    />,
  );
  expect(container).toHaveTextContent('<img src="https://example.com/track"');
  expect(container).toHaveTextContent("<script>alert(1)</script>");
  expect(screen.getByText("Website")).toBeInTheDocument();
  expect(screen.getByText("Image description")).toBeInTheDocument();
  expect(container.querySelector("a, img, script, iframe")).toBeNull();
});

it("keeps unfinished streamed Markdown readable and formats it when completed", () => {
  const { rerender } = render(<ChatAnswer text="A **partial" />);
  expect(screen.getByText("A **partial")).toBeInTheDocument();
  rerender(<ChatAnswer text="A **partial answer** [1][4]" />);
  expect(screen.getByText("partial answer").tagName).toBe("STRONG");
  expect(screen.getByText(/\[1\]\[4\]/)).toBeInTheDocument();
  rerender(<ChatAnswer text={"```text\n**unfinished code"} />);
  expect(screen.getByText("**unfinished code").tagName).toBe("CODE");
});
