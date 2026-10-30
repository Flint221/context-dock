import { memo } from "react";
import Markdown, { type Components } from "react-markdown";

// Answers are read-only: links and images never create navigation or requests.
const components: Components = {
  a: ({ children }) => <span>{children}</span>,
  img: ({ alt }) => <span>{alt ?? ""}</span>,
};

export const ChatAnswer = memo(function ChatAnswer({ text }: { text: string }) {
  return (
    <div className="chat-markdown">
      <Markdown components={components}>{text}</Markdown>
    </div>
  );
});
