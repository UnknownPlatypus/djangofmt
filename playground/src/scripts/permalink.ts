import { createPatch } from "diff";
import LZString from "lz-string";
import { writeClipboardText } from "./clipboard";

function createPermalink(code: string, mode: string, width: number, indent: number): string {
  const compressed = LZString.compressToEncodedURIComponent(code);
  const params = new URLSearchParams({ mode: mode, width: String(width), indent: String(indent), code: compressed });
  return `${window.location.origin}${window.location.pathname}?${params.toString()}`;
}

function savePermalinkToClipboard(code: string, mode: string, width: number, indent: number): void {
  const url = createPermalink(code, mode, width, indent);
  writeClipboardText(url);
}

function parsePermalinkCode(): string | null {
  const params = new URLSearchParams(window.location.search);
  const codeParam = params.get("code");

  if (!codeParam) return null;

  try {
    const code = LZString.decompressFromEncodedURIComponent(codeParam);
    if (!code) return null;

    return code;
  } catch (error) {
    console.error("Failed to parse permalink:", error);
    return null;
  }
}

function openGithubIssue(source: string, mode: string, width: number, indent: number, formatted: string) {
  const fullDiff = createPatch("template", source, formatted, "", "", { context: 3 });
  // Strip header lines and @@ hunk markers, keep only +/- lines and context
  const diff = fullDiff.split("\n").slice(4).filter((line) => !line.startsWith("@@")).join("\n").trim();
  const link = createPermalink(source, mode, width, indent);
  const body = `### Description

<!-- Describe what is not working. -->

### Playground diff

\`\`\`diff
${diff}
\`\`\`

[Playground link](${link})`;

  const url = new URL("https://github.com/unknownplatypus/djangofmt/issues/new");
  url.searchParams.set("body", body);
  window.open(url, "_blank");
}

export { openGithubIssue, parsePermalinkCode, savePermalinkToClipboard };
