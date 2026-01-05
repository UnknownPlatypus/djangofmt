import AnsiToHtml from "ansi-to-html";
import init, { format, lint, type LintResult } from "../../pkg/djangofmt_wasm.js";
import { writeClipboardText } from "./clipboard";
import { createEditors, monaco } from "./monaco-editor";
import { savePermalinkToClipboard } from "./permalink";

const { outputEditor } = createEditors();
window.monaco = monaco;

function formatCode(source: string, width: number, indent: number, mode: string) {
  const footer = document.getElementById("format-duration") as HTMLDivElement;

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

const ansiConverter = new AnsiToHtml({ escapeXML: true });

function lintCode(source: string, mode: string): number {
  const lintOutput = document.getElementById("lint-output") as HTMLPreElement;
  try {
    const result = lint(source, mode) as LintResult;
    lintOutput.innerHTML = result.output
      ? ansiConverter.toHtml(result.output)
      : "<span class=\"text-success\">✓ No lint issues found.</span>";
    return result.error_count;
  } catch (e) {
    lintOutput.textContent = `Error: ${e}`;
    console.error("Lint error:", e);
    return 1;
  }
}

declare global {
  interface Window {
    MonacoEnvironment?: monaco.Environment;
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
    "https://cdn.jsdelivr.net/gh/starfederation/datastar@1.0.0-RC.7/bundles/datastar.js"
  );
});
