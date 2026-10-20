// Acquire synchronously, before React can render disabled controls. Cloud settings
// share a scope across the simple and advanced panels for the same project.
const pending = new Set<string>();
export function beginOperation(scope: string): (() => void) | null {
  if (pending.has(scope)) return null;
  pending.add(scope);
  let released = false;
  return () => {
    if (!released) {
      pending.delete(scope);
      released = true;
    }
  };
}
