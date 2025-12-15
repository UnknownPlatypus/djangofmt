import "https://cdn.jsdelivr.net/gh/starfederation/datastar@1.0.0-RC.6/bundles/datastar.js";
import init, { format } from "../../pkg/djangofmt_wasm.js";
import { writeClipboardText } from "./clipboard";
import { createEditors, monaco } from "./monaco-editor";
import { savePermalinkToClipboard } from "./permalink";

// Initialize Monaco editors
const { inputEditor, outputEditor } = createEditors();

// Initialize WASM and set up formatting
async function initWasm() {
  await init();

  function runFormat() {
    const source = inputEditor.getValue();
    const lineLength = parseInt(
      (document.querySelector("[data-bind='width']") as HTMLInputElement)
        ?.value,
    );
    const indentWidth = parseInt(
      (document.querySelector("[data-bind='indent']") as HTMLInputElement)
        ?.value,
    );
    const profile = (
      document.querySelector("[data-bind='mode']") as HTMLSelectElement
    )?.value?.toLowerCase();
    const footer = document.querySelector("#format-duration") as HTMLDivElement;

    try {
      const start = performance.now();
      const formatted = format(source, lineLength, indentWidth, profile);
      const duration = performance.now() - start;
      outputEditor.setValue(formatted);
      footer.textContent = `Formatted in ${duration.toFixed(1)}ms!`;
    } catch (e) {
      outputEditor.setValue(`Error: ${e}`);
    }
  }

  inputEditor.onDidChangeModelContent(runFormat);
  document.getElementById("controls")?.addEventListener("click", runFormat);
  runFormat();
}

await initWasm();

// Expose functions globally for Datastar expressions
window.monaco = monaco;
window.savePermalinkToClipboard = savePermalinkToClipboard;
window.writeClipboardText = writeClipboardText;

declare global {
  interface Window {
    monaco: typeof monaco;
    writeClipboardText: typeof writeClipboardText;
    savePermalinkToClipboard: typeof savePermalinkToClipboard;
  }
}
