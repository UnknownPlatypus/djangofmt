export async function writeClipboardText(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text.trim());
  } catch (error) {
    console.error((error as Error).message);
  }
}
