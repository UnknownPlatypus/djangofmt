import init, { format, lint } from "../../pkg/djangofmt_wasm.js";
import { writeClipboardText } from "./clipboard";
import { createEditors, monaco } from "./monaco-editor";
import { savePermalinkToClipboard } from "./permalink";

// Initialize Monaco editors
const { outputEditor } = createEditors();
window.monaco = monaco;

function formatCode(
  source: string,
  width: number,
  indent: number,
  mode: string,
) {
  const footer = document.querySelector("#format-duration") as HTMLDivElement;

  try {
    const start = performance.now();
    const formatted = format(source, width, indent, mode.toLowerCase());
    const duration = performance.now() - start;
    outputEditor.setValue(formatted);
    footer.textContent = `Formatted in ${duration.toFixed(1)}ms!`;
  } catch (e) {
    outputEditor.setValue(`Error: ${e}`);
  }
}

function lintCode(source: string): string {
  try {
    return lint(source);
  } catch (e) {
    return `Lint error: ${e}`;
  }
}

declare global {
  interface Window {
    monaco: typeof monaco;
    formatCode: typeof formatCode;
    lintCode: typeof lintCode;
    writeClipboardText: typeof writeClipboardText;
    savePermalinkToClipboard: typeof savePermalinkToClipboard;
  }
}

// Initialize WASM & expose some functions globally for Datastar expressions
init().then(() => {
  window.formatCode = formatCode;
  window.lintCode = lintCode;
  window.savePermalinkToClipboard = savePermalinkToClipboard;
  window.writeClipboardText = writeClipboardText;

  import(
    // @ts-expect-error
    "https://cdn.jsdelivr.net/gh/starfederation/datastar@1.0.0-RC.6/bundles/datastar.js"
  );
});
