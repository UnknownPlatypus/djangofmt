import AnsiToHtml from "ansi-to-html";
import init, { format, lint, type LintResult } from "../../pkg/djangofmt_wasm.js";
import { writeClipboardText } from "./clipboard";
import { createEditors, monaco } from "./monaco-editor";
import { openGithubIssue, savePermalinkToClipboard } from "./permalink";

window.monaco = monaco;
createEditors();

function formatCode(
  source: string,
  width: number,
  indent: number,
  mode: string,
): { formatted: string; duration: number } {
  try {
    const start = performance.now();
    const formatted = format(source, width, indent, mode.toLowerCase());
    const duration = performance.now() - start;
    return { formatted, duration };
  } catch (e) {
    return { formatted: `Error: ${e}`, duration: 0 };
  }
}

const ansiConverter = new AnsiToHtml({ escapeXML: true });

function lintCode(source: string, mode: string): { text: string; errorCount: number } {
  try {
    const result = lint(source, mode.toLowerCase()) as LintResult;
    const text = result.output ? ansiConverter.toHtml(result.output) : "✓ No lint issues found.";
    return { text, errorCount: result.error_count };
  } catch (e) {
    return { text: ansiConverter.toHtml(`Error: ${e}`), errorCount: 1 };
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
    openGithubIssue: typeof openGithubIssue;
  }
}

// Initialize WASM & expose some functions globally for Datastar expressions
init().then(() => {
  window.formatCode = formatCode;
  window.lintCode = lintCode;
  window.savePermalinkToClipboard = savePermalinkToClipboard;
  window.writeClipboardText = writeClipboardText;
  window.openGithubIssue = openGithubIssue;

  import(
    // @ts-expect-error
    "https://cdn.jsdelivr.net/gh/starfederation/datastar@1.0.0-RC.7/bundles/datastar.js"
  );
});
