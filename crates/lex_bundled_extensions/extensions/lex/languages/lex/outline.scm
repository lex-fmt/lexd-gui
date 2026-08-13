; Outline queries for Lex
;
; Drives Zed's symbol picker, breadcrumbs, and document outline. The grammar's
; structural hierarchy is: document_title > session > (nested session | definition).
; Verbatim blocks are not surfaced — they are content, not structure.

; Document title — top-level entry. The title text is what users see.
(document_title
  title: (line_content) @name) @item

; Sessions — every session title is an outline entry. Indentation in the
; outline mirrors the CST's session nesting (sessions can contain sessions).
(session
  title: (line_content) @name) @item

; Definitions — each definition subject is an outline entry. Tables are
; technically definitions in the CST; users still benefit from seeing the
; table caption in the outline.
(definition
  subject: (subject_content) @name) @item
