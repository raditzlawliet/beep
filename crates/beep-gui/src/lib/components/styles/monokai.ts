import { EditorView } from "@codemirror/view";
import { HighlightStyle } from "@codemirror/language";
import { tags } from "@lezer/highlight";
import { lineComment } from "@codemirror/commands";

export const monokaiHighlight = HighlightStyle.define([
  { tag: tags.keyword, color: "#F92672" },
  { tag: tags.comment, color: "#75715E", fontStyle: "italic" },
  { tag: tags.string, color: "#E6DB74" },
  { tag: tags.number, color: "#AE81FF" },
  { tag: tags.typeName, color: "#66D9EF" },
  { tag: tags.function(tags.variableName), color: "#A6E22E" },
  { tag: tags.className, color: "#A6E22E" },
  { tag: tags.operator, color: "#F92672" },
  { tag: tags.regexp, color: "#E6DB74" },
  { tag: tags.escape, color: "#AE81FF" },
  { tag: tags.labelName, color: "#E6DB74" },
  { tag: tags.namespace, color: "#F92672" },
  { tag: tags.meta, color: "#75715E" },
  { tag: tags.propertyName, color: "#66D9EF" },
  { tag: tags.definition(tags.variableName), color: "#FD971F" },
  { tag: tags.url, color: "#E6DB74" },
]);

export const monokaiTheme = EditorView.theme(
  {
    "&": {
      // backgroundColor: "#272822",
      // color: "#F8F8F2",
      fontSize: "0.75rem",
    },
    ".cm-line": {
      lineHeight: "1.125rem",
    },
    // ".cm-content": {
    //     caretColor: "#F8F8F0",
    // },
    // "&.cm-focused .cm-cursor": {
    //     borderLeftColor: "#F8F8F0",
    // },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection":
      {
        backgroundColor:
          "color-mix(in srgb, var(--color-primary) 50%, transparent)",
      },
    ".cm-activeLine": {
      backgroundColor: "#3E3D32",
    },
    ".cm-gutters": {
      // backgroundColor: "#272822",
      // color: "#75715E",
      color: "color-mix(in srgb, var(--color-base-content) 50%, transparent)",
      backgroundColor: "var(--color-base-200)",
      borderRight:
        "1px solid color-mix(in srgb, var(--color-base-content) 10%, transparent)",
    },
    ".cm-gutterElement": {
      paddingLeft: "8px !important",
      paddingRight: "8px !important",
    },
    ".cm-activeLineGutter": {
      backgroundColor: "#3E3D32",
    },
  },
  { dark: true },
);
