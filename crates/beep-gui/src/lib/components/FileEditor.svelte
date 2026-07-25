<script lang="ts">
    import { untrack, mount } from "svelte";
    import {
        EditorView,
        keymap,
        lineNumbers,
        highlightActiveLine,
        highlightActiveLineGutter,
        GutterMarker,
        gutter,
    } from "@codemirror/view";
    import { EditorState, Prec, StateEffect, StateField } from "@codemirror/state";
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
    import RunGutterButton from "./RunGutterButton.svelte";

    // --- Run gutter ---

    const setRunMarkers = StateEffect.define<number[]>();
    const runMarkerField = StateField.define<Set<number>>({
        create() { return new Set(); },
        update(set, tr) {
            for (const e of tr.effects) {
                if (e.is(setRunMarkers)) return new Set(e.value);
            }
            return set;
        },
    });

    class RunGutterMarker extends GutterMarker {
        private _pos: number;
        private _onClick: (pos: number, e: MouseEvent) => void;
        constructor(pos: number, onClick: (pos: number, e: MouseEvent) => void) {
            super();
            this._pos = pos;
            this._onClick = onClick;
        }
        toDOM() {
            const span = document.createElement("span");
            mount(RunGutterButton, {
                target: span,
                props: { pos: this._pos, onClick: this._onClick },
            });
            return span;
        }
        eq(other: RunGutterMarker) { return this._pos === other._pos; }
    }

    interface Props {
        value: string;
        language?: "text" | "json" | "html" | "xml" | "http";
        onchange?: (value: string) => void;
        oncursorchange?: (pos: number) => void;
        initialCursorPos?: number;
        class?: string;
        wrapLines?: boolean;
        runMarkers?: number[];
        onRunMarkerClick?: (pos: number, event: MouseEvent) => void;
    }

    let {
        value,
        language = "text",
        onchange,
        oncursorchange,
        initialCursorPos,
        class: className = "",
        wrapLines = true,
        runMarkers,
        onRunMarkerClick,
    }: Props = $props();

    let container: HTMLDivElement;
    let view: EditorView | undefined;
    let _skipCursorSync = false;

    function createEditor(
        initialValue: string,
        lang: "text" | "json" | "html" | "xml" | "http",
        wrap: boolean,
    ) {
        const runGutter = gutter({
            class: "cm-run-gutter",
            lineMarker(view, line) {
                const positions = view.state.field(runMarkerField, false);
                if (!positions || positions.size === 0) return null;
                const lineFrom = line.from;
                for (const pos of positions) {
                    if (pos >= lineFrom && pos < lineFrom + line.length) {
                        return new RunGutterMarker(pos, (p, e) => onRunMarkerClick?.(p, e));
                    }
                }
                return null;
            },
            initialSpacer: null,
        });

        const extensions = [
            runMarkerField,
            runGutter,
            lineNumbers(),
            highlightActiveLine(),
            highlightActiveLineGutter(),
            ...(wrap ? [EditorView.lineWrapping] : []),
            history(),
            bracketMatching(),
            syntaxHighlighting(monokaiHighlight),
            monokaiTheme,
            keymap.of([...defaultKeymap, ...historyKeymap]),
            Prec.highest(keymap.of([{ key: "Mod-Enter", run: () => true }])),
            EditorView.updateListener.of((update) => {
                if (update.docChanged && onchange) {
                    onchange(update.state.doc.toString());
                }
                // Track cursor position changes
                if (update.selectionSet && oncursorchange) {
                    _skipCursorSync = true;
                    const pos = update.state.selection.main.head;
                    oncursorchange(pos);
                }
            }),
            EditorView.theme({
                "&": { height: "100%" },
                ".cm-scroller": { overflow: "auto" },
                ".cm-run-gutter": { width: "16px", textAlign: "center" },
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

        return editor;
    }

    $effect(() => {
        if (!container) return;
        const initVal = untrack(() => value);

        view = createEditor(initVal, language, wrapLines);

        view.focus();

        return () => {
            view?.destroy();
            view = undefined;
        };
    });

    $effect(() => {
        const currentValue = value;
        const pos = initialCursorPos;
        if (!view) return;

        // Update content if changed
        if (view.state.doc.toString() !== currentValue) {
            view.dispatch({
                changes: {
                    from: 0,
                    to: view.state.doc.length,
                    insert: currentValue,
                },
            });
        }

        // Cursor position
        if (pos !== undefined && !_skipCursorSync) {
            const clampedPos = pos < 0 ? 0 : Math.min(pos, view.state.doc.length);
            view.dispatch({
                selection: { anchor: clampedPos, head: clampedPos },
                scrollIntoView: true,
            });
        }
        _skipCursorSync = false;
    });

    // Sync runMarkers to CodeMirror state field
    $effect(() => {
        const markers = runMarkers;
        const editor = view;
        if (!editor) return;
        editor.dispatch({
            effects: setRunMarkers.of(markers ?? []),
        });
    });
</script>

<div bind:this={container} class={className}></div>
