<script lang="ts">
    // Single-line CodeMirror input with {{var}} highlight, autocomplete and hover.
    // Drop-in for <input type="text">: wrapper gets the daisyUI input classes.
    import { untrack } from "svelte";
    import { EditorView, keymap, placeholder as cmPlaceholder } from "@codemirror/view";
    import { Compartment, EditorState, Prec } from "@codemirror/state";
    import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
    import { completionStatus } from "@codemirror/autocomplete";
    import { varExtensions } from "$lib/vars/var-extensions";

    interface Props {
        value: string;
        placeholder?: string;
        disabled?: boolean;
        oninput?: (value: string) => void;
        onblur?: () => void;
        onfocus?: () => void;
        onenter?: () => void;
        class?: string;
    }

    let {
        value,
        placeholder = "",
        disabled = false,
        oninput,
        onblur,
        onfocus,
        onenter,
        class: className = "",
    }: Props = $props();

    let container: HTMLDivElement;
    let view: EditorView | undefined;

    // Join pasted / typed newlines into one line.
    const singleLine = EditorState.transactionFilter.of((tr) => {
        if (!tr.docChanged || tr.newDoc.lines === 1) return tr;
        const text = tr.newDoc.toString().replace(/\r?\n/g, " ");
        return [tr, { changes: { from: 0, to: tr.newDoc.length, insert: text }, sequential: true }];
    });

    const enterKey = Prec.highest(
        keymap.of([
            {
                key: "Enter",
                run: (v) => {
                    // Let the completion list take Enter first.
                    if (completionStatus(v.state) === "active") return false;
                    onenter?.();
                    return true;
                },
            },
            // Ctrl+Enter is the global send hotkey; do not insert a line here.
            { key: "Mod-Enter", run: () => true },
        ]),
    );

    const inputTheme = EditorView.theme({
        "&": { width: "100%", fontSize: "inherit", background: "transparent" },
        "&.cm-focused": { outline: "none" },
        ".cm-scroller": {
            fontFamily: "inherit",
            lineHeight: "inherit",
            overflowX: "auto",
            overflowY: "hidden",
            scrollbarWidth: "none",
        },
        ".cm-content": { padding: "0", caretColor: "currentColor" },
        ".cm-line": { padding: "0" },
        ".cm-placeholder": { opacity: "0.5" },
    });

    const editableConf = new Compartment();

    function editable(on: boolean) {
        return [EditorView.editable.of(on), EditorState.readOnly.of(!on)];
    }

    $effect(() => {
        const initial = untrack(() => value);
        const isDisabled = untrack(() => disabled);
        view = new EditorView({
            parent: container,
            state: EditorState.create({
                doc: initial,
                extensions: [
                    enterKey,
                    singleLine,
                    history(),
                    keymap.of([...defaultKeymap, ...historyKeymap]),
                    cmPlaceholder(untrack(() => placeholder)),
                    editableConf.of(editable(!isDisabled)),
                    varExtensions(),
                    inputTheme,
                    EditorView.updateListener.of((u) => {
                        if (u.docChanged) oninput?.(u.state.doc.toString());
                        if (u.focusChanged) {
                            if (u.view.hasFocus) onfocus?.();
                            else onblur?.();
                        }
                    }),
                ],
            }),
        });
        return () => {
            view?.destroy();
            view = undefined;
        };
    });

    // Outside value change (form reset, sync from code) -> replace the doc.
    $effect(() => {
        const next = value;
        if (!view) return;
        const current = view.state.doc.toString();
        if (next === current) return;
        const head = Math.min(view.state.selection.main.head, next.length);
        view.dispatch({
            changes: { from: 0, to: current.length, insert: next },
            selection: { anchor: head },
        });
    });

    $effect(() => {
        const on = !disabled;
        view?.dispatch({ effects: editableConf.reconfigure(editable(on)) });
    });
</script>

<div
    bind:this={container}
    class="var-input flex items-center min-w-0 {className}"
    class:opacity-50={disabled}
></div>
