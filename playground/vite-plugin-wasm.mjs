import { spawn } from "node:child_process";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const cratesDir = path.join(root, "crates");

function buildWasm(logger) {
  return new Promise((resolve) => {
    logger.info("[wasm] rebuilding djangofmt_wasm…");
    const child = spawn("just", ["playground-wasm-build"], { cwd: root, stdio: "inherit" });
    child.on("error", (err) => {
      logger.error(`[wasm] failed to spawn just: ${err.message}`);
      resolve(false);
    });
    child.on("exit", (code) => {
      if (code !== 0) logger.error(`[wasm] just playground-wasm-build exited with code ${code}`);
      resolve(code === 0);
    });
  });
}

export default function wasmPack() {
  return {
    name: "djangofmt-wasm-pack",
    apply: "serve",
    configureServer(server) {
      let building = false;
      let pending = false;
      const rebuild = async () => {
        if (building) {
          pending = true;
          return;
        }
        building = true;
        const ok = await buildWasm(server.config.logger);
        building = false;
        if (ok) (server.hot ?? server.ws).send({ type: "full-reload" });
        if (pending) {
          pending = false;
          rebuild();
        }
      };
      server.watcher.add(cratesDir);
      const onRustChange = (file) => {
        if (file.startsWith(cratesDir) && file.endsWith(".rs")) rebuild();
      };
      for (const event of ["add", "change", "unlink"]) {
        server.watcher.on(event, onRustChange);
      }
    },
  };
}
