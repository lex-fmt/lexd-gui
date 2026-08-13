//! A registry of toolbar preview providers.
//!
//! Upstream Zed hardcodes which file types get the quick-action-bar preview
//! (eye) button. Rather than growing that match arm for every Lex-owned
//! preview, the upstream fallback makes a single call into
//! [`render_toolbar_preview_button`], and any crate that offers a preview
//! registers a [`ToolbarPreviewProvider`] here at init. The button built here
//! mirrors the upstream one: click opens the preview in the item's pane,
//! alt-click opens it in a split.

use std::rc::Rc;

use gpui::{Action, AnyElement, App, Entity, Global, Keystroke, Modifiers, Window};
use ui::{Tooltip, prelude::*, text_for_keystroke};
use workspace::item::ItemHandle;
use workspace::{Pane, Workspace};

pub trait ToolbarPreviewProvider: 'static {
    fn button_id(&self) -> &'static str;
    fn tooltip_text(&self) -> &'static str;
    /// The action shown in the tooltip, so its keybinding is displayed.
    fn open_action(&self) -> Box<dyn Action>;
    fn supports(&self, item: &dyn ItemHandle, cx: &App) -> bool;
    fn open(
        &self,
        workspace: &mut Workspace,
        item: &dyn ItemHandle,
        pane: Entity<Pane>,
        to_the_side: bool,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    );
}

#[derive(Default)]
struct ToolbarPreviewProviders(Vec<Rc<dyn ToolbarPreviewProvider>>);

impl Global for ToolbarPreviewProviders {}

pub fn register_toolbar_preview_provider(provider: impl ToolbarPreviewProvider, cx: &mut App) {
    cx.default_global::<ToolbarPreviewProviders>()
        .0
        .push(Rc::new(provider));
}

/// Builds the toolbar preview button for the first registered provider that
/// supports `active_item`, or `None` if no provider does. Called by the
/// upstream quick action bar as its fallback after the built-in file types.
pub fn render_toolbar_preview_button(
    active_item: &dyn ItemHandle,
    workspace: gpui::WeakEntity<Workspace>,
    cx: &mut App,
) -> Option<AnyElement> {
    let provider = cx
        .try_global::<ToolbarPreviewProviders>()?
        .0
        .iter()
        .find(|provider| provider.supports(active_item, cx))?
        .clone();

    let alt_click = Keystroke {
        key: "click".into(),
        modifiers: Modifiers::alt(),
        ..Default::default()
    };
    let tooltip_text = provider.tooltip_text();
    let open_action = provider.open_action();

    let button = IconButton::new(provider.button_id(), IconName::Eye)
        .icon_size(IconSize::Small)
        .style(ButtonStyle::Subtle)
        .tooltip(move |_window, cx| {
            Tooltip::with_meta(
                tooltip_text,
                Some(open_action.as_ref()),
                format!(
                    "{} to open in a split",
                    text_for_keystroke(&alt_click.modifiers, &alt_click.key, cx)
                ),
                cx,
            )
        })
        .on_click({
            let active_item = active_item.boxed_clone();
            move |_, window, cx| {
                let Some(workspace) = workspace.upgrade() else {
                    return;
                };
                workspace.update(cx, |workspace, cx| {
                    let Some(pane) = workspace.pane_for(active_item.as_ref()) else {
                        return;
                    };
                    let to_the_side = window.modifiers().alt;
                    provider.open(
                        workspace,
                        active_item.as_ref(),
                        pane,
                        to_the_side,
                        window,
                        cx,
                    );
                });
            }
        });

    Some(button.into_any_element())
}
