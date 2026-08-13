//! Native preview pane for Lex documents.
//!
//! The pane asks the lex-lsp language server (shipped by the bundled Lex
//! extension) to export the buffer as HTML via the `lex.export` workspace
//! command, parses the bounded HTML subset lex-babel emits, and renders it
//! as gpui elements — no webview involved. The preview refreshes as the
//! buffer is edited.

use gpui::{App, actions};
use workspace::Workspace;

mod html_tree;
pub mod lex_preview_view;
mod render_html;

actions!(
    lex,
    [
        /// Opens a preview of the current lex file.
        OpenPreview,
        /// Opens a preview of the current lex file in a split pane.
        OpenPreviewToTheSide,
        /// Opens a lex preview that follows the active editor.
        OpenFollowingPreview
    ]
);

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            return;
        };
        lex_preview_view::LexPreviewView::register(workspace, window, cx);
    })
    .detach();
}
