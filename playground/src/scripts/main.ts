import AnsiToHtml from "ansi-to-html";
import init, { format, lint, type LintResult } from "../../pkg/djangofmt_wasm.js";
import { writeClipboardText } from "./clipboard";
import { createEditors, monaco } from "./monaco-editor";
import { savePermalinkToClipboard } from "./permalink";

const { outputEditor } = createEditors();
window.monaco = monaco;

/**
 * Format source code and update the UI with the result and duration.
 *
 * Formats `source` using the provided `width`, `indent`, and `mode`, writes the formatted text into the global `outputEditor`, and updates the DOM element with id "format-duration" with the time taken. On error, sets the output editor content to `Error: <error>`.
 *
 * @param source - The source code to format
 * @param width - Maximum line width used by the formatter
 * @param indent - Number of spaces per indentation level
 * @param mode - Formatting mode (case-insensitive) passed to the formatter
 */
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

/**
 * Run the linter on the given source and render its output into the page.
 *
 * The function writes the linter's formatted output (converted from ANSI to HTML)
 * into the element with id "lint-output".
 *
 * @param source - The source code to lint
 * @returns The number of lint errors found; returns `1` if linting fails due to an internal error
 */
function lintCode(source: string): number {
  try {
    const result = lint(source) as LintResult;
    const lintOutput = document.getElementById("lint-output") as HTMLPreElement;
    lintOutput.innerHTML = result.output
      ? ansiConverter.toHtml(result.output)
      : "<span class=\"text-success\">✓ No lint issues found.</span>";
    return result.error_count;
  } catch (e) {
    return 1;
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
    "https://cdn.jsdelivr.net/gh/starfederation/datastar@1.0.0-RC.7/bundles/datastar.js"
  );
});