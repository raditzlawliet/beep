<script lang="ts">
    // Zed-style Go To Line/Column (Ctrl+G). Jumps as you type (preview);
    // Enter keeps it, Esc / click outside goes back to where you were.
    import { untrack } from "svelte";
    import { editorCursor, parseLineCol } from "$lib/editor-cursor.svelte";

    let input: HTMLInputElement | undefined = $state();
    let text = $state("");
    let snap: unknown = null;
    let start = $state<{ line: number; col: number; lines: number } | null>(null);

    $effect(() => {
        if (!editorCursor.gotoOpen) return;
        // Read once on open; the preview moves the cursor and must not reset this.
        untrack(() => {
            snap = editorCursor.target?.snapshot() ?? null;
            start = editorCursor.current;
            text = "";
        });
        queueMicrotask(() => input?.focus());
    });

    function preview() {
        const target = editorCursor.target;
        if (!target) return;
        const pos = parseLineCol(text);
        if (pos) target.goto(pos.line, pos.col, false);
        else target.restore(snap, false);
    }

    function confirm() {
        const target = editorCursor.target;
        const pos = parseLineCol(text);
        editorCursor.closeGoto();
        if (!target) return;
        if (pos) target.goto(pos.line, pos.col, true);
        else target.restore(snap, true);
    }

    function cancel() {
        const target = editorCursor.target;
        editorCursor.closeGoto();
        target?.restore(snap, true);
    }

    function onkeydown(e: KeyboardEvent) {
        if (e.key === "Enter") {
            e.preventDefault();
            confirm();
        } else if (e.key === "Escape") {
            e.preventDefault();
            cancel();
        }
    }
</script>

{#if editorCursor.gotoOpen}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <dialog class="modal items-start bg-transparent!" open onclick={cancel}>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
            class="modal-box p-0 w-96 mt-[10vh] border border-base-content/10"
            onclick={(e: MouseEvent) => e.stopPropagation()}
        >
            <input
                bind:this={input}
                bind:value={text}
                oninput={preview}
                {onkeydown}
                class="w-full bg-transparent px-3 py-2 text-sm font-mono outline-none"
                placeholder={start ? `${start.line}:${start.col}` : "Line:Column"}
                spellcheck="false"
                aria-label="Go to line and column"
            />
            {#if start}
                <div class="px-3 py-1 text-xs opacity-50 border-t border-base-content/10">
                    Current Line: {start.line} of {start.lines} (column {start.col})
                </div>
            {/if}
        </div>
    </dialog>
{/if}
