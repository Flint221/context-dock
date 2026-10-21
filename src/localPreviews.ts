import {
  isLocalEndpoint,
  type Endpoint,
  type LocalProcess,
} from "./services/processes";

// This is a display heuristic, not an HTTP probe or permission to stop an app.
// Advanced mode keeps the complete native inventory available.
const nonWebPorts = new Set([
  22, 25, 53, 110, 135, 137, 138, 139, 143, 389, 445, 465, 587, 636, 993, 995,
  1433, 1521, 2375, 2376, 3306, 3389, 5432, 5672, 5900, 6379, 9222, 9229, 9230,
  11211, 27017, 27018, 27019,
]);
const webRuntime =
  /^(?:node(?:js)?|bun|deno|python(?:w|\d+(?:\.\d+)*)?|php(?:-cgi)?|ruby|dotnet|javaw?|nginx|caddy|httpd|uvicorn|gunicorn|vite|next)(?:\.exe)?$/i;
function commonPreviewPort(port: number) {
  return (
    [80, 443, 1234, 1420, 2015, 6006, 6007, 7000, 7070, 10000].includes(port) ||
    (port >= 3000 && port <= 5999) ||
    (port >= 8000 && port <= 9099)
  );
}
export function localPreviewEndpoints(process: LocalProcess): Endpoint[] {
  if (!process.canStop) return [];
  const runtime = webRuntime.test(process.name);
  const candidates = process.endpoints.filter(
    (e) =>
      e.protocol === "TCP" &&
      e.port > 0 &&
      isLocalEndpoint(e) &&
      !nonWebPorts.has(e.port) &&
      (runtime || commonPreviewPort(e.port)),
  );
  // Prefer IPv4 for the usual dual-stack localhost binding. Other loopback
  // addresses retain their own entries; the full bindings remain in Advanced.
  candidates.sort(
    (a, b) =>
      Number(a.address.includes(":")) - Number(b.address.includes(":")) ||
      a.port - b.port,
  );
  const seen = new Set<string>();
  return candidates.filter((e) => {
    const host = ["0.0.0.0", "127.0.0.1", "::", "::1"].includes(e.address)
      ? "localhost"
      : e.address;
    const key = `${host}:${e.port}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}
export function localPreviewUrl(endpoint: Endpoint, secure: boolean): string {
  const address =
    endpoint.address === "0.0.0.0"
      ? "127.0.0.1"
      : endpoint.address === "::"
        ? "::1"
        : endpoint.address;
  return `${secure ? "https" : "http"}://${address.includes(":") ? `[${address}]` : address}:${endpoint.port}/`;
}
