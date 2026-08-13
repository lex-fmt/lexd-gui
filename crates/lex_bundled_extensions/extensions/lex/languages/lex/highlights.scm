; Highlight queries for Lex — Zed flavour.
;
; This file is intentionally separate from tree-sitter-lex's upstream
; queries/highlights.scm, which targets nvim-treesitter / TextMate scope
; conventions (@markup.heading, @markup.bold, …). Zed's theme uses a
; different, flat capture-name vocabulary (@title, @emphasis.strong, …),
; so the same scopes don't render. The structural capture choices below
; mirror lex-analysis/src/semantic_tokens.rs at CST granularity — the
; LSP's semantic tokens still override these for clients that request
; them, but a syntactic baseline must look right too.
;
; ---- Lex-specific dual capture ---------------------------------------------
; Every capture below is dual-tagged with a `.lex` suffix on the right.
; Zed tries the rightmost capture first and falls back leftward when the
; theme has no style for it. Result:
;
;   - With no theme override: rightmost @x.lex misses, falls back to @x,
;     which the base theme styles. Lex looks like other languages.
;   - With theme_overrides defining "x.lex": rightmost wins, only Lex
;     uses the override. Other languages capture only @x and stay on
;     the base theme.
;
; This is what makes themes/lex-monochrome.json a Lex-specific override
; rather than an editor-wide recolour.

; === Document Title ===
(document_title
  title: (line_content) @title @title.lex)

(document_subtitle
  subtitle: (line_content) @title @title.lex)

(document_title
  title: (line_content
    (list_marker) @punctuation.list_marker @punctuation.list_marker.lex))

; === Sessions ===
(session
  title: (line_content) @title @title.lex)

(session
  title: (line_content
    (list_marker) @punctuation.list_marker @punctuation.list_marker.lex))

; === Definitions ===
(definition
  subject: (subject_content) @property @property.lex)

; === Verbatim Blocks ===
(verbatim_block
  subject: (subject_content) @string.special @string.special.lex)

(verbatim_block
  (paragraph) @text.literal @text.literal.lex)
(verbatim_block
  (definition) @text.literal @text.literal.lex)
(verbatim_block
  (list) @text.literal @text.literal.lex)
(verbatim_block
  (verbatim_content) @text.literal @text.literal.lex)
(verbatim_block
  (session) @text.literal @text.literal.lex)

(verbatim_group_item
  subject: (subject_content) @string.special @string.special.lex)
(verbatim_group_item
  (paragraph) @text.literal @text.literal.lex)
(verbatim_group_item
  (definition) @text.literal @text.literal.lex)
(verbatim_group_item
  (list) @text.literal @text.literal.lex)
(verbatim_group_item
  (verbatim_content) @text.literal @text.literal.lex)
(verbatim_group_item
  (session) @text.literal @text.literal.lex)

; === Lists ===
(list_item
  (list_marker) @punctuation.list_marker @punctuation.list_marker.lex)

; === Annotations ===
(annotation_marker) @punctuation.special @punctuation.special.lex
(annotation_close) @punctuation.special @punctuation.special.lex
(annotation_header) @comment @comment.lex
(annotation_inline_text) @comment @comment.lex
(annotation_block
  (_) @comment @comment.lex)

; Verbatim closing metadata: the `:: label ::` line at the end of a
; verbatim block is structurally an annotation but visually it's part of
; the verbatim block. Render it as @text.literal so it doesn't pop out.
; Must come AFTER the generic annotation captures above.
(verbatim_block
  (annotation_marker) @text.literal @text.literal.lex)
(verbatim_block
  (annotation_close) @text.literal @text.literal.lex)
(verbatim_block
  (annotation_header) @text.literal @text.literal.lex)

; === Tables ===
; Table caption — emphasis to distinguish from regular definitions.
(definition
  subject: (subject_content) @emphasis @emphasis.lex
  (table_row))

; Header row — first table_row in a definition (bold).
(definition
  subject: (_) .
  (table_row
    (table_cell
      (text_content) @emphasis.strong @emphasis.strong.lex)))

; Pipe delimiters
(table_row
  (pipe_delimiter) @punctuation.delimiter @punctuation.delimiter.lex)

; Table separator rows — comment-toned (cosmetic, parser ignores them).
(table_separator_row) @comment @comment.lex

; === Inline formatting ===
(strong) @emphasis.strong @emphasis.strong.lex
(emphasis) @emphasis @emphasis.lex
(code_span) @text.literal @text.literal.lex
(math_span) @text.literal @text.literal.lex
(escape_sequence) @string.escape @string.escape.lex

; === References ===
(reference) @link_text @link_text.lex
(citation_reference) @link_text @link_text.lex
(annotation_reference) @link_text @link_text.lex
(url_reference) @link_uri @link_uri.lex
(file_reference) @link_uri @link_uri.lex
(session_reference) @link_text @link_text.lex
(tocome_reference) @constant @constant.lex
(number_reference) @link_text @link_text.lex
