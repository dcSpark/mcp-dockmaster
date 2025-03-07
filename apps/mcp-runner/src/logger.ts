// Enable debug logging
const DEBUG = true;

export function debugLog(...args: any[]) {
  if (DEBUG) {
    console.error(`[DEBUG ${new Date().toISOString()}]`, ...args);
  }
}
