import LZString from "lz-string";
import { writeClipboardText } from "./clipboard";

function savePermalinkToClipboard(
  code: string,
  mode: string,
  width: number,
  indent: number,
): void {
  const compressed = LZString.compressToEncodedURIComponent(code);
  const params = new URLSearchParams({
    mode: mode,
    width: String(width),
    indent: String(indent),
    code: compressed,
  });
  const url = `${window.location.origin}${window.location.pathname}?${params.toString()}`;
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

export { savePermalinkToClipboard, parsePermalinkCode };
