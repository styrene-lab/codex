import { build } from "esbuild";
import { copyFile, mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const out = resolve(root, "../../assets/vendor");
await mkdir(out, { recursive: true });
await build({ legalComments: "none", entryPoints: [resolve(root, "src/index.tsx")], bundle: true, minify: true, format: "iife", target: "es2020", outfile: resolve(out, "excalidraw.bundle.js"), define: { "process.env.NODE_ENV": '"production"' }, loader: { ".woff2": "dataurl" } });
await copyFile(resolve(root, "node_modules/@excalidraw/excalidraw/dist/prod/index.css"), resolve(out, "excalidraw.css"));
