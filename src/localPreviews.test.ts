import { expect, it } from "vitest";
import { localPreviewEndpoints, localPreviewUrl } from "./localPreviews";
import type { Endpoint, LocalProcess } from "./services/processes";

const endpoint = (
  port: number,
  address = "127.0.0.1",
  protocol: "TCP" | "UDP" = "TCP",
): Endpoint => ({ port, address, protocol, scope: 0 });
const process = (
  name: string,
  endpoints: Endpoint[],
  canStop = true,
): LocalProcess => ({
  name,
  endpoints,
  canStop,
  pid: 123,
  parentPid: 1,
  path: "C:\\Apps\\server.exe",
  started: "birth",
  startedAt: null,
  protection: "",
});
it("finds runtime custom ports and common frontend ports without including databases, debuggers or UDP", () => {
  const custom = endpoint(45001);
  expect(localPreviewEndpoints(process("node.exe", [custom]))).toEqual([
    custom,
  ]);
  expect(
    localPreviewEndpoints(
      process("custom-site.exe", [endpoint(5173, "0.0.0.0")]),
    ),
  ).toHaveLength(1);
  for (const e of [
    endpoint(5432),
    endpoint(6379),
    endpoint(9229),
    endpoint(5173, "192.168.1.20"),
    endpoint(8080, "127.0.0.1", "UDP"),
  ]) {
    expect(localPreviewEndpoints(process("node.exe", [e]))).toEqual([]);
  }
  expect(
    localPreviewEndpoints(process("background-agent.exe", [custom])),
  ).toEqual([]);
  expect(
    localPreviewEndpoints(process("System", [endpoint(80)], false)),
  ).toEqual([]);
});
it("collapses ordinary dual-stack bindings without changing the original inventory or other loopback hosts", () => {
  const endpoints = [
    endpoint(5173, "::"),
    endpoint(5173),
    endpoint(5173, "0.0.0.0"),
    endpoint(5173, "127.0.0.2"),
  ];
  const original = structuredClone(endpoints);
  const result = localPreviewEndpoints(process("node.exe", endpoints));
  expect(result).toEqual([endpoint(5173), endpoint(5173, "127.0.0.2")]);
  expect(endpoints).toEqual(original);
});
it("copies the local address used by the native preview and reverses HTTP/HTTPS", () => {
  expect(localPreviewUrl(endpoint(5173, "0.0.0.0"), false)).toBe(
    "http://127.0.0.1:5173/",
  );
  expect(localPreviewUrl(endpoint(8000, "::"), true)).toBe(
    "https://[::1]:8000/",
  );
  expect(localPreviewUrl(endpoint(8000, "::"), false)).toBe(
    "http://[::1]:8000/",
  );
});
