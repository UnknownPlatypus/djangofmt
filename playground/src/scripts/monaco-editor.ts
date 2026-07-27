import * as monaco from "monaco-editor/editor/editor.api.js";
import "monaco-editor/features/register.all.js";
import "monaco-editor/languages/definitions/html/register.js";
import "monaco-editor/languages/features/html/register.js";
import editorWorker from "monaco-editor/editor/editor.worker.js?worker";
import htmlWorker from "monaco-editor/languages/features/html/html.worker.js?worker";
import { parsePermalinkCode } from "./permalink";

const defaultTemplate = `\
{% extends "base.html" %}

{% block content %}
<div class="badly-formatted"><h1>Welcome {{ user.username }}</h1>
  </div><form method=""></form>
{% endblock %}
`;
const initialTemplate = parsePermalinkCode() ?? defaultTemplate;

// Setup monaco code editors
self.MonacoEnvironment = {
  getWorker(_: any, label: String) {
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
  const inputEditor = monaco.editor.create(inputContainer, { value: initialTemplate, ...monacoOptions });
  inputEditor.onDidChangeModelContent(() => {
    // Monaco editor does not trigger proper events so we create one here
    inputContainer.dispatchEvent(
      new CustomEvent("monaco-change", { detail: { value: inputEditor.getValue() }, bubbles: true }),
    );
  });
  (inputContainer as any).editor = inputEditor;

  const outputContainer = document.getElementById("monacoOutput") as HTMLElement;
  const outputEditor = monaco.editor.create(outputContainer, { readOnly: true, ...monacoOptions });
  (outputContainer as any).editor = outputEditor;

  return { inputEditor, outputEditor };
}

export { createEditors, monaco };
