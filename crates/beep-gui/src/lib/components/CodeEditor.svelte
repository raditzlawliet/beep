<script lang="ts">
    import { untrack } from "svelte";
    import {
        EditorView,
        keymap,
        lineNumbers,
        highlightActiveLine,
        highlightActiveLineGutter,
    } from "@codemirror/view";
    import { EditorState } from "@codemirror/state";
    import {
        defaultKeymap,
        history,
        historyKeymap,
    } from "@codemirror/commands";
    import { json } from "@codemirror/lang-json";
    import { html } from "@codemirror/lang-html";
    import { xml } from "@codemirror/lang-xml";
    import { syntaxHighlighting, bracketMatching } from "@codemirror/language";
    import { monokaiHighlight, monokaiTheme } from "./styles/monokai";

    interface Props {
        value: string;
        language?: "text" | "json" | "html" | "xml";
        onchange?: (value: string) => void;
        class?: string;
        readonly?: boolean;
        wrapLines?: boolean;
    }

    let {
        value,
        language = "text",
        onchange,
        class: className = "",
        readonly = false,
        wrapLines = true,
    }: Props = $props();

    let container: HTMLDivElement;
    let view: EditorView | undefined;

    function createEditor(
        initialValue: string,
        lang: "text" | "json" | "html" | "xml",
        wrap: boolean,
    ) {
        const extensions = [
            lineNumbers(),
            highlightActiveLine(),
            highlightActiveLineGutter(),
            ...(wrap ? [EditorView.lineWrapping] : []),
            history(),
            bracketMatching(),
            syntaxHighlighting(monokaiHighlight),
            monokaiTheme,
            keymap.of([...defaultKeymap, ...historyKeymap]),
            EditorView.updateListener.of((update) => {
                if (update.docChanged && onchange) {
                    onchange(update.state.doc.toString());
                }
            }),
            EditorView.theme({
                "&": { height: "100%" },
                ".cm-scroller": { overflow: "auto" },
            }),
        ];

        if (lang === "json") {
            extensions.push(json());
        } else if (lang === "html") {
            extensions.push(html());
        } else if (lang === "xml") {
            extensions.push(xml());
        }

        const state = EditorState.create({
            doc: initialValue,
            extensions,
        });

        const editor = new EditorView({
            state,
            parent: container,
        });

        if (readonly) {
            editor.contentDOM.setAttribute("contenteditable", "false");
        }

        return editor;
    }

    // Create editor on mount or when language changes
    $effect(() => {
        if (!container) return;
        const initVal = untrack(() => value);

        view = createEditor(initVal, language, wrapLines);

        return () => {
            view?.destroy();
            view = undefined;
        };
    });

    // Sync external value changes into the editor
    $effect(() => {
        const currentValue = value;
        if (!view) return;
        if (view.state.doc.toString() === currentValue) return;

        view.dispatch({
            changes: {
                from: 0,
                to: view.state.doc.length,
                insert: currentValue,
            },
        });
    });
</script>

<div bind:this={container} class={className}></div>
