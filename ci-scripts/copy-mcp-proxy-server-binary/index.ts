// Parse command line arguments to determine build type
const buildTypeIndex = Deno.args.indexOf("--build-type");
let buildType = "release"; // Default to release

if (buildTypeIndex !== -1 && buildTypeIndex < Deno.args.length - 1) {
  buildType = Deno.args[buildTypeIndex + 1];
}

console.log(`Using build type: ${buildType}`);

// Get target triple from environment or use default
const targetTriple = Deno.build.target;

// Paths
const workspaceRoot = new URL("../../", import.meta.url).pathname.replace(
  /^\/([A-Z]:)/,
  "$1",
);
const targetDir = `${workspaceRoot}apps/mcp-proxy-server/target/${buildType}`;
const sidecarsDir = `${workspaceRoot}apps/mcp-dockmaster/src-tauri/sidecars`;

// Binary name based on platform and target triple
const binaryName = "mcp-proxy-server";

const sourcePath = `${targetDir}/${binaryName}${Deno.build.os === "windows" ? ".exe" : ""}`;
const targetPath =
  Deno.build.os === "windows"
    ? `${sidecarsDir}/${binaryName}-${targetTriple}.exe`
    : `${sidecarsDir}/${binaryName}-${targetTriple}`;

// Ensure sidecars directory exists
try {
  await Deno.mkdir(sidecarsDir, { recursive: true });
} catch (error) {
  if (!(error instanceof Deno.errors.AlreadyExists)) {
    throw error;
  }
}

// Copy the binary
try {
  await Deno.copyFile(sourcePath, targetPath);
  console.log(
    `Successfully copied ${binaryName} from ${sourcePath} to ${targetPath}`,
  );
} catch (error) {
  console.error("Error copying binary:", error);
  Deno.exit(1);
}
