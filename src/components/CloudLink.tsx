import { useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
export function CloudLink({
  href,
  children,
}: {
  href: string;
  children: ReactNode;
}) {
  const [error, setError] = useState("");
  return (
    <>
      <a
        href={href}
        onClick={(e) => {
          e.preventDefault();
          setError("");
          void invoke("cloud_link", { url: href }).catch((e) =>
            setError(String(e)),
          );
        }}
      >
        {children}
      </a>
      {error && <span role="alert">{error}</span>}
    </>
  );
}
