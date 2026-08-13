//! Native preview pane for Lex documents.
//!
//! The pane asks the lex-lsp language server (shipped by the bundled Lex
//! extension) to export the buffer as HTML via the `lex.export` workspace
//! command, parses the bounded HTML subset lex-babel emits, and renders it
//! as gpui elements — no webview involved. The preview refreshes as the
//! buffer is edited.

use gpui::{Action, App, actions};
use multi_buffer::MultiBuffer;
use workspace::Workspace;
use workspace::item::ItemHandle;

mod html_tree;
pub mod lex_preview_view;
mod render_html;
pub mod toolbar;

use crate::lex_preview_view::LexPreviewView;

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
    toolbar::register_toolbar_preview_provider(LexToolbarPreviewProvider, cx);
    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            return;
        };
        lex_preview_view::LexPreviewView::register(workspace, window, cx);
    })
    .detach();
}

struct LexToolbarPreviewProvider;

impl toolbar::ToolbarPreviewProvider for LexToolbarPreviewProvider {
    fn button_id(&self) -> &'static str {
        "toggle-lex-preview"
    }

    fn tooltip_text(&self) -> &'static str {
        "Preview Lex"
    }

    fn open_action(&self) -> Box<dyn Action> {
        Box::new(OpenPreview)
    }

    fn supports(&self, item: &dyn ItemHandle, cx: &App) -> bool {
        item.act_as::<MultiBuffer>(cx)
            .is_some_and(|buffer| LexPreviewView::is_lex_file(&buffer, cx))
    }

    fn open(
        &self,
        workspace: &mut Workspace,
        item: &dyn ItemHandle,
        pane: gpui::Entity<workspace::Pane>,
        to_the_side: bool,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Workspace>,
    ) {
        let Some(buffer) = item.act_as::<MultiBuffer>(cx) else {
            return;
        };
        if to_the_side {
            LexPreviewView::open_preview_to_the_side_of_pane(workspace, buffer, pane, window, cx);
        } else {
            LexPreviewView::open_preview_in_pane(workspace, buffer, pane, window, cx);
        }
    }
}
