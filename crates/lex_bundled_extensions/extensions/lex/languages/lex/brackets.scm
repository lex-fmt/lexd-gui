; Bracket-matching queries for Lex
;
; Lex's inline pairs (**bold**, _emphasis_, `code`, $math$) are modelled as
; flat nodes in the grammar without separate open/close tokens, so they
; don't fit Zed's @open/@close shape. Annotation markers (:: ... ::) use the
; same token on both sides, which the matcher can't disambiguate either.
;
; Left intentionally empty until the grammar exposes distinct delimiter
; nodes that can be paired.
