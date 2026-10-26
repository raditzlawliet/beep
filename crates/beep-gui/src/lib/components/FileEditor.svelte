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
    import { Compartment, EditorState, Prec, StateEffect, StateField } from "@codemirror/state";
    import type { Extension } from "@codemirror/state";
    import {
        defaultKeymap,
        history,
        historyKeymap,
    } from "@codemirror/commands";
    import { json } from "@codemirror/lang-json";
    import { html } from "@codemirror/lang-html";
    import { xml } from "@codemirror/lang-xml";
    import { javascript } from "@codemirror/lang-javascript";
    import { syntaxHighlighting, bracketMatching } from "@codemirror/language";
    import { monokaiHighlight, monokaiTheme } from "./styles/monokai";
    import { httpLanguage } from "./styles/http-lang";
    import { varExtensions } from "$lib/vars/var-extensions";
    import { editorCursor, type GotoTarget } from "$lib/editor-cursor.svelte";
    import RunGutterButton from "./RunGutterButton.svelte";

    // --- Run gutter ---

    const setRunMarkers = StateEffect.define<number[]>();
    const runMarkerField = StateField.define<Set<number>>({
        create() { return new Set(); },
        update(set, tr) {
            for (const e of tr.effects) {
                if (e.is(setRunMarkers)) return new Set(e.value);
            }
            // Keep markers on their lines until the reparse sends fresh ones.
            if (!tr.docChanged || set.size === 0) return set;
            return new Set([...set].map((pos) => tr.changes.mapPos(pos, 1)));
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
        language?: "text" | "json" | "html" | "xml" | "http" | "javascript";
        onchange?: (value: string) => void;
        oncursorchange?: (pos: number) => void;
        initialCursorPos?: number;
        class?: string;
        wrapLines?: boolean;
        runMarkers?: number[];
        onRunMarkerClick?: (pos: number, event: MouseEvent) => void;
        /** {{var}} highlight, autocomplete and hover (http-file code view). */
        vars?: boolean;
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
        vars = false,
    }: Props = $props();

    let container: HTMLDivElement;
    let view: EditorView | undefined;
    let _skipCursorSync = false;

    // CodeMirror joins lines with LF. Keep the file's own line ending so a CRLF file
    // does not look edited on open (no false dirty / temp tab promotion).
    function eolOf(text: string): string {
        return text.includes("\r\n") ? "\r\n" : "\n";
    }

    function docText(state: EditorState): string {
        return state.doc.sliceString(0, state.doc.length, eolOf(value));
    }

    // Language lives in a compartment so a late switch (auto-detect) keeps cursor and undo.
    const languageConf = new Compartment();
    const varsConf = new Compartment();

    $effect(() => {
        const on = vars;
        untrack(() => view)?.dispatch({ effects: varsConf.reconfigure(on ? varExtensions({ js: true }) : []) });
    });

    function languageExtension(lang: "text" | "json" | "html" | "xml" | "http" | "javascript"): Extension {
        if (lang === "json") return json();
        if (lang === "html") return html();
        if (lang === "xml") return xml();
        if (lang === "javascript") return javascript();
        if (lang === "http") return httpLanguage();
        return [];
    }

    $effect(() => {
        const lang = language;
        untrack(() => view)?.dispatch({ effects: languageConf.reconfigure(languageExtension(lang)) });
    });

    function createEditor(
        initialValue: string,
        lang: "text" | "json" | "html" | "xml" | "http" | "javascript",
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
            // Redraw when markers are set without a doc change (gutter only reruns on doc/viewport).
            lineMarkerChange: (update) =>
                update.transactions.some((tr) => tr.effects.some((e) => e.is(setRunMarkers))),
            initialSpacer: null,
        });

        const extensions = [
            varsConf.of(untrack(() => vars) ? varExtensions({ js: true }) : []),
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
                if (update.docChanged || update.selectionSet) publishCursor(update.state);
                if (update.docChanged && onchange) {
                    onchange(docText(update.state));
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

        extensions.push(languageConf.of(languageExtension(lang)));

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

    // Status bar line:col + Go To Line (Ctrl+G). Only the active tab's editor is mounted.
    function publishCursor(state: EditorState) {
        const head = state.selection.main.head;
        const line = state.doc.lineAt(head);
        editorCursor.set({ line: line.number, col: head - line.from + 1, lines: state.doc.lines });
    }

    const gotoTarget: GotoTarget = {
        goto(lineNo, col, focus) {
            if (!view) return;
            const doc = view.state.doc;
            const line = doc.line(Math.min(Math.max(lineNo, 1), doc.lines));
            const c = Math.min(Math.max(col ?? 1, 1), line.length + 1);
            const pos = line.from + c - 1;
            view.dispatch({
                selection: { anchor: pos },
                effects: EditorView.scrollIntoView(pos, { y: "center" }),
            });
            if (focus) view.focus();
        },
        snapshot() {
            return view ? { selection: view.state.selection, scrollTop: view.scrollDOM.scrollTop } : null;
        },
        restore(snap, focus) {
            if (!view || !snap) return;
            const s = snap as { selection: EditorState["selection"]; scrollTop: number };
            view.dispatch({ selection: s.selection });
            view.scrollDOM.scrollTop = s.scrollTop;
            if (focus) view.focus();
        },
    };

    $effect(() => {
        if (!container) return;
        const initVal = untrack(() => value);

        view = createEditor(initVal, untrack(() => language), wrapLines);
        publishCursor(view.state);
        editorCursor.setTarget(gotoTarget);

        view.focus();

        return () => {
            if (editorCursor.target === gotoTarget) {
                editorCursor.setTarget(null);
                editorCursor.set(null);
            }
            view?.destroy();
            view = undefined;
        };
    });

    $effect(() => {
        const currentValue = value;
        const pos = initialCursorPos;
        if (!view) return;

        // Update content if changed. Compare with line endings normalized (the doc is
        // always LF), so mixed / lone-CR endings alone never count as an edit.
        if (view.state.doc.toString() !== currentValue.replace(/\r\n?/g, "\n")) {
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
