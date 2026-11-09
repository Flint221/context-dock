import { Cloud, FolderTree } from "lucide-react";
export type ProjectTab = "files" | "cloud";
const tabs = [
  { id: "files", label: "Files & tracking", Icon: FolderTree },
  { id: "cloud", label: "Drive publishing", Icon: Cloud },
] as const;
export function ProjectTabs({
  tab,
  select,
}: {
  tab: ProjectTab;
  select: (tab: ProjectTab) => void;
}) {
  return (
    <div
      className="project-tabs"
      role="tablist"
      aria-label="Project sections"
      onKeyDown={(e) => {
        const buttons = Array.from(
          e.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]'),
        );
        const index = buttons.indexOf(
          document.activeElement as HTMLButtonElement,
        );
        const next =
          e.key === "ArrowRight"
            ? (index + 1) % buttons.length
            : e.key === "ArrowLeft"
              ? (index + buttons.length - 1) % buttons.length
              : e.key === "Home"
                ? 0
                : e.key === "End"
                  ? buttons.length - 1
                  : -1;
        if (next >= 0) {
          e.preventDefault();
          buttons[next].focus();
        }
      }}
    >
      {tabs.map(({ id, label, Icon }) => (
        <button
          key={id}
          id={`project-tab-${id}`}
          role="tab"
          aria-selected={tab === id}
          aria-controls={`project-panel-${id}`}
          tabIndex={tab === id ? 0 : -1}
          onClick={() => select(id)}
        >
          <Icon size={18} />
          {label}
        </button>
      ))}
    </div>
  );
}
