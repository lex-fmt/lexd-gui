; Indentation queries for Lex
;
; Lex is whitespace-sensitive: nested content inside a session, definition,
; verbatim block, or list item is indented relative to its parent. When the
; user presses Enter inside one of these blocks, Zed should preserve (not
; reset) the current indent level. The default behavior is usually correct
; for prose; we keep this query minimal and let Zed's plain-text indent
; heuristics handle continuation.
;
; If specific cases need explicit control, add @indent / @outdent here.

; List item bodies should retain their indent on newline.
(list_item) @indent
