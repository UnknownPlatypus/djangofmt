import * as monaco from "monaco-editor/esm/vs/editor/editor.api";
import "monaco-editor/min/vs/editor/editor.main.css";
import "monaco-editor/esm/vs/editor/editor.all.js";
import "monaco-editor/esm/vs/basic-languages/html/html.contribution";
import "monaco-editor/esm/vs/language/html/monaco.contribution";
// @ts-expect-error - Vite worker import
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
// @ts-expect-error - Vite worker import
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import { parsePermalinkCode } from "./permalink";

const defaultTemplate = `{% extends "base.html" %}\n\n{% block content %}\n<div class="badly-formatted"><h1>Welcome {{ user.username }}</h1>\n  </div>\n{% endblock %}\n`;
const initialTemplate = parsePermalinkCode() ?? defaultTemplate;

// Setup monaco code editors
self.MonacoEnvironment = {
  getWorker(_, label) {
    if (label === "html") {
      return new htmlWorker();
    }
    return new editorWorker();
  },
};

const monacoOptions = {
  language: "html",
  automaticLayout: true,
  minimap: { enabled: false },
  fontSize: 14,
  roundedSelection: false,
  scrollBeyondLastLine: false,
  contextmenu: true,
  theme: "vs-dark",
};

function createEditors() {
  // Setup input and output editors
  const inputContainer = document.getElementById("monacoInput") as HTMLElement;
  const inputEditor = monaco.editor.create(inputContainer, {
    value: initialTemplate,
    ...monacoOptions,
  });
  (inputContainer as any).editor = inputEditor;

  const outputContainer = document.getElementById(
    "monacoOutput",
  ) as HTMLElement;
  const outputEditor = monaco.editor.create(outputContainer, {
    readOnly: true,
    ...monacoOptions,
  });
  (outputContainer as any).editor = outputEditor;

  return { inputEditor, outputEditor };
}

export { monaco, createEditors };
