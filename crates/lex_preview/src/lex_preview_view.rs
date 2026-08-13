use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    SharedString, Styled, Subscription, Task, WeakEntity, Window, div, rems,
};
use language::{Buffer, BufferEvent};
use lsp::{DEFAULT_LSP_REQUEST_TIMEOUT, LanguageServer};
use multi_buffer::MultiBuffer;
use project::Project;
use ui::prelude::*;
use workspace::item::Item;
use workspace::{Pane, Workspace};

use crate::html_tree::{HtmlNode, parse_html_document};
use crate::render_html::{RenderContext, render_nodes};
use crate::{OpenFollowingPreview, OpenPreview, OpenPreviewToTheSide};

/// The language server name registered by the bundled Lex extension.
const LEX_LANGUAGE_SERVER_NAME: &str = "lex-lsp";
/// The lex-lsp workspace command that serializes a document to another format.
const EXPORT_COMMAND: &str = "lex.export";
const REFRESH_DEBOUNCE: Duration = Duration::from_millis(200);

pub struct LexPreviewView {
    focus_handle: FocusHandle,
    project: Entity<Project>,
    buffer: Option<Entity<Buffer>>,
    document_dir: Option<PathBuf>,
    content: Option<Result<Arc<[HtmlNode]>, SharedString>>,
    _refresh: Task<()>,
    _buffer_subscription: Option<Subscription>,
    _workspace_subscription: Option<Subscription>,
    _project_subscription: Subscription,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LexPreviewMode {
    /// The preview always shows the contents of the provided editor.
    Default,
    /// The preview "follows" the last active editor of a lex file.
    Follow,
}

impl LexPreviewView {
    pub fn new(
        mode: LexPreviewMode,
        active_buffer: Entity<MultiBuffer>,
        workspace_handle: WeakEntity<Workspace>,
        project: Entity<Project>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let workspace_subscription = if mode == LexPreviewMode::Follow
                && let Some(workspace) = workspace_handle.upgrade()
            {
                Some(Self::subscribe_to_workspace(workspace, window, cx))
            } else {
                None
            };

            // The preview can open before the language server has started
            // (server startup waits on worktree trust); refresh once it comes
            // up so the pane doesn't stay stuck on the waiting message.
            let project_subscription = cx.subscribe_in(
                &project,
                window,
                |this: &mut Self, _, event: &project::Event, window, cx| {
                    if let project::Event::LanguageServerAdded(_, name, _) = event
                        && name.0.as_ref() == LEX_LANGUAGE_SERVER_NAME
                    {
                        this.refresh_preview(false, window, cx);
                    }
                },
            );

            let buffer = active_buffer.read_with(cx, |buffer, _cx| buffer.as_singleton());
            let subscription = buffer
                .as_ref()
                .map(|buffer| Self::create_buffer_subscription(buffer, window, cx));

            let mut this = Self {
                focus_handle: cx.focus_handle(),
                project,
                buffer,
                document_dir: None,
                content: None,
                _refresh: Task::ready(()),
                _buffer_subscription: subscription,
                _workspace_subscription: workspace_subscription,
                _project_subscription: project_subscription,
            };
            this.refresh_preview(false, window, cx);
            this
        })
    }

    fn subscribe_to_workspace(
        workspace: Entity<Workspace>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe_in(
            &workspace,
            window,
            move |this: &mut LexPreviewView, workspace, event: &workspace::Event, window, cx| {
                if let workspace::Event::ActiveItemChanged = event {
                    let workspace = workspace.read(cx);
                    if let Some(active_item) = workspace.active_item(cx)
                        && let Some(buffer) = active_item.downcast::<MultiBuffer>()
                        && Self::is_lex_file(&buffer, cx)
                    {
                        let Some(buffer) = buffer.read(cx).as_singleton() else {
                            return;
                        };
                        if this.buffer.as_ref() != Some(&buffer) {
                            this._buffer_subscription =
                                Some(Self::create_buffer_subscription(&buffer, window, cx));
                            this.buffer = Some(buffer);
                            this.refresh_preview(false, window, cx);
                        }
                    }
                }
            },
        )
    }

    fn create_buffer_subscription(
        buffer: &Entity<Buffer>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe_in(
            buffer,
            window,
            move |this, _buffer, event: &BufferEvent, window, cx| match event {
                BufferEvent::Edited { .. } => this.refresh_preview(true, window, cx),
                BufferEvent::Saved => this.refresh_preview(false, window, cx),
                _ => {}
            },
        )
    }

    fn refresh_preview(&mut self, debounce: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffer.clone() else {
            self.content = None;
            cx.notify();
            return;
        };

        self._refresh = cx.spawn_in(window, async move |this, cx| {
            if debounce {
                cx.background_executor().timer(REFRESH_DEBOUNCE).await;
            }

            let Ok((text, document_dir, server)) = this.update(cx, |this, cx| {
                let text = buffer.read(cx).text();
                let document_dir = buffer
                    .read(cx)
                    .file()
                    .and_then(|file| file.as_local())
                    .map(|file| file.abs_path(cx))
                    .and_then(|path| path.parent().map(Path::to_path_buf));
                let server = this.lex_language_server(&buffer, cx);
                (text, document_dir, server)
            }) else {
                return;
            };

            let Some(server) = server else {
                this.update(cx, |this, cx| {
                    this.set_content(
                        Err("Waiting for the Lex language server to start…".into()),
                        cx,
                    );
                })
                .ok();
                return;
            };

            let response = server
                .request::<lsp::request::ExecuteCommand>(
                    lsp::ExecuteCommandParams {
                        command: EXPORT_COMMAND.to_string(),
                        arguments: vec![
                            serde_json::Value::String("html".to_string()),
                            serde_json::Value::String(text),
                        ],
                        ..Default::default()
                    },
                    DEFAULT_LSP_REQUEST_TIMEOUT,
                )
                .await
                .into_response();

            let content = match response {
                Ok(Some(serde_json::Value::String(html))) => cx
                    .background_spawn(async move {
                        parse_html_document(&html)
                            .map(Arc::from)
                            .map_err(|error| SharedString::from(error.to_string()))
                    })
                    .await,
                Ok(_) => Err("The Lex language server returned no HTML".into()),
                Err(error) => Err(format!("Failed to render the document: {error}").into()),
            };

            this.update(cx, |this, cx| {
                this.document_dir = document_dir;
                this.set_content(content, cx);
            })
            .ok();
        });
    }

    fn lex_language_server(
        &self,
        buffer: &Entity<Buffer>,
        cx: &mut Context<Self>,
    ) -> Option<Arc<LanguageServer>> {
        let lsp_store = self.project.read(cx).lsp_store();
        lsp_store.update(cx, |lsp_store, cx| {
            buffer.update(cx, |buffer, cx| {
                lsp_store
                    .running_language_servers_for_local_buffer(buffer, cx)
                    .find(|(_, server)| server.name().0.as_ref() == LEX_LANGUAGE_SERVER_NAME)
                    .map(|(_, server)| server.clone())
            })
        })
    }

    fn set_content(&mut self, content: Result<Arc<[HtmlNode]>, SharedString>, cx: &mut Context<Self>) {
        self.content = Some(content);
        cx.notify();
    }

    fn find_existing_preview_item_idx(
        pane: &Pane,
        buffer: &Entity<MultiBuffer>,
        cx: &App,
    ) -> Option<usize> {
        let buffer_id = buffer.read(cx).as_singleton()?.entity_id();
        pane.items_of_type::<LexPreviewView>()
            .find(|view| {
                view.read(cx)
                    .buffer
                    .as_ref()
                    .is_some_and(|buffer| buffer.entity_id() == buffer_id)
            })
            .and_then(|view| pane.index_for_item(&view))
    }

    pub fn resolve_active_item_as_lex_buffer(
        workspace: &Workspace,
        cx: &mut Context<Workspace>,
    ) -> Option<Entity<MultiBuffer>> {
        workspace
            .active_item(cx)?
            .act_as::<MultiBuffer>(cx)
            .filter(|buffer| Self::is_lex_file(buffer, cx))
    }

    pub fn is_lex_file(buffer: &Entity<MultiBuffer>, cx: &App) -> bool {
        buffer
            .read(cx)
            .as_singleton()
            .and_then(|buffer| buffer.read(cx).file())
            .is_some_and(|file| {
                Path::new(file.file_name(cx))
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("lex"))
            })
    }

    fn create_lex_view(
        mode: LexPreviewMode,
        workspace: &mut Workspace,
        buffer: Entity<MultiBuffer>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Entity<LexPreviewView> {
        let workspace_handle = workspace.weak_handle();
        let project = workspace.project().clone();
        LexPreviewView::new(mode, buffer, workspace_handle, project, window, cx)
    }

    pub fn open_preview_in_pane(
        workspace: &mut Workspace,
        buffer: Entity<MultiBuffer>,
        pane: Entity<Pane>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        Self::activate_or_add_preview(workspace, buffer, pane, true, window, cx);
    }

    pub fn open_preview_to_the_side_of_pane(
        workspace: &mut Workspace,
        buffer: Entity<MultiBuffer>,
        origin_pane: Entity<Pane>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let target_pane = workspace.adjacent_pane_of(&origin_pane, window, cx);
        Self::activate_or_add_preview(workspace, buffer, target_pane, false, window, cx);
    }

    fn activate_or_add_preview(
        workspace: &mut Workspace,
        buffer: Entity<MultiBuffer>,
        pane: Entity<Pane>,
        focus: bool,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let existing_view_idx = Self::find_existing_preview_item_idx(pane.read(cx), &buffer, cx);
        if let Some(existing_view_idx) = existing_view_idx {
            pane.update(cx, |pane, cx| {
                pane.activate_item(existing_view_idx, focus, focus, window, cx);
            });
        } else {
            let view = Self::create_lex_view(LexPreviewMode::Default, workspace, buffer, window, cx);
            pane.update(cx, |pane, cx| {
                pane.add_item(Box::new(view), focus, focus, None, window, cx)
            });
        }
        cx.notify();
    }

    pub fn register(workspace: &mut Workspace, _window: &mut Window, _cx: &mut Context<Workspace>) {
        workspace.register_action(move |workspace, _: &OpenPreview, window, cx| {
            if let Some(buffer) = Self::resolve_active_item_as_lex_buffer(workspace, cx) {
                let pane = workspace.active_pane().clone();
                Self::open_preview_in_pane(workspace, buffer, pane, window, cx);
            }
        });

        workspace.register_action(move |workspace, _: &OpenPreviewToTheSide, window, cx| {
            if let Some(buffer) = Self::resolve_active_item_as_lex_buffer(workspace, cx) {
                let pane = workspace.active_pane().clone();
                Self::open_preview_to_the_side_of_pane(workspace, buffer, pane, window, cx);
            }
        });

        workspace.register_action(move |workspace, _: &OpenFollowingPreview, window, cx| {
            if let Some(buffer) = Self::resolve_active_item_as_lex_buffer(workspace, cx) {
                let view =
                    Self::create_lex_view(LexPreviewMode::Follow, workspace, buffer, window, cx);
                workspace.active_pane().update(cx, |pane, cx| {
                    pane.add_item(Box::new(view), true, true, None, window, cx)
                });
                cx.notify();
            }
        });
    }
}

impl Render for LexPreviewView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text_color = cx.theme().colors().text;
        let muted_color = cx.theme().colors().text_muted;
        let background = cx.theme().colors().editor_background;

        let body = match &self.content {
            Some(Ok(nodes)) => {
                let nodes = nodes.clone();
                let mut render_context = RenderContext::new(self.document_dir.clone(), cx);
                v_flex()
                    .gap_3()
                    .children(render_nodes(&nodes, &mut render_context, cx))
                    .into_any_element()
            }
            Some(Err(error)) => div()
                .text_color(muted_color)
                .child(error.clone())
                .into_any_element(),
            None => div()
                .text_color(muted_color)
                .child("No lex file selected")
                .into_any_element(),
        };

        v_flex()
            .id("LexPreview")
            .key_context("LexPreview")
            .track_focus(&self.focus_handle(cx))
            .size_full()
            .bg(background)
            .overflow_y_scroll()
            .child(
                div()
                    .w_full()
                    .max_w(rems(50.))
                    .mx_auto()
                    .p_8()
                    .text_color(text_color)
                    .child(body),
            )
    }
}

impl Focusable for LexPreviewView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<()> for LexPreviewView {}

impl Item for LexPreviewView {
    type Event = ();

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::Eye))
    }

    fn tab_content_text(&self, _detail: usize, cx: &App) -> SharedString {
        self.buffer
            .as_ref()
            .and_then(|buffer| buffer.read(cx).file())
            .map(|file| format!("Preview {}", file.file_name(cx)).into())
            .unwrap_or_else(|| "Lex Preview".into())
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("lex preview: open")
    }

    fn to_item_events(_event: &Self::Event, _f: &mut dyn FnMut(workspace::item::ItemEvent)) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs::FakeFs;
    use futures::StreamExt as _;
    use gpui::TestAppContext;
    use language::{FakeLspAdapter, Language, LanguageConfig, LanguageMatcher};
    use project::ProjectPath;
    use serde_json::json;
    use util::path;
    use util::rel_path::rel_path;
    use workspace::{AppState, Workspace};

    fn init_test(cx: &mut TestAppContext) -> std::sync::Arc<AppState> {
        cx.update(|cx| {
            let state = AppState::test(cx);
            editor::init(cx);
            crate::init(cx);
            state
        })
    }

    #[gpui::test]
    async fn preview_renders_html_from_lex_lsp(cx: &mut TestAppContext) {
        init_test(cx);

        let fs = FakeFs::new(cx.executor());
        fs.insert_tree(path!("/root"), json!({ "doc.lex": "Title\n\nHello world.\n" }))
            .await;
        let project = Project::test(fs, [path!("/root").as_ref()], cx).await;

        let language_registry = project.read_with(cx, |project, _| project.languages().clone());
        language_registry.add(std::sync::Arc::new(Language::new(
            LanguageConfig {
                name: "Lex".into(),
                matcher: std::sync::Arc::new(LanguageMatcher {
                    path_suffixes: vec!["lex".to_string()],
                    ..Default::default()
                }),
                ..Default::default()
            },
            None,
        )));
        let mut fake_servers = language_registry.register_fake_lsp(
            "Lex",
            FakeLspAdapter {
                name: LEX_LANGUAGE_SERVER_NAME,
                ..Default::default()
            },
        );

        let (workspace, cx) =
            cx.add_window_view(|window, cx| Workspace::test_new(project.clone(), window, cx));

        let worktree_id = project.read_with(cx, |project, cx| {
            project.worktrees(cx).next().expect("has worktree").read(cx).id()
        });
        workspace
            .update_in(cx, |workspace, window, cx| {
                workspace.open_path(
                    ProjectPath {
                        worktree_id,
                        path: rel_path("doc.lex").into(),
                    },
                    None,
                    true,
                    window,
                    cx,
                )
            })
            .await
            .expect("open doc.lex");

        // The toolbar registry (populated by crate::init) offers a preview
        // button for the active lex editor.
        workspace.update_in(cx, |workspace, _window, cx| {
            let item = workspace.active_item(cx).expect("active item");
            let button = crate::toolbar::render_toolbar_preview_button(
                item.as_ref(),
                workspace.weak_handle(),
                cx,
            );
            assert!(button.is_some(), "lex file should get a preview button");
        });

        let fake_server = fake_servers.next().await.expect("lex-lsp starts");
        fake_server.set_request_handler::<lsp::request::ExecuteCommand, _, _>(
            |params, _| async move {
                assert_eq!(params.command, EXPORT_COMMAND);
                assert_eq!(params.arguments[0], json!("html"));
                assert!(
                    params.arguments[1]
                        .as_str()
                        .is_some_and(|content| content.contains("Hello world.")),
                    "export must receive the buffer text"
                );
                Ok(Some(json!(
                    "<html><body><div class=\"lex-document\">\
                     <p class=\"lex-paragraph\">Hello <strong>world</strong>.</p>\
                     </div></body></html>"
                )))
            },
        );

        let preview = workspace.update_in(cx, |workspace, window, cx| {
            let buffer = LexPreviewView::resolve_active_item_as_lex_buffer(workspace, cx)
                .expect("active item is a lex buffer");
            let pane = workspace.active_pane().clone();
            LexPreviewView::open_preview_in_pane(workspace, buffer, pane, window, cx);
            workspace
                .active_pane()
                .read(cx)
                .items_of_type::<LexPreviewView>()
                .next()
                .expect("preview pane item exists")
        });
        cx.run_until_parked();

        // The preview pane itself is not previewable: no button for it.
        workspace.update_in(cx, |workspace, _window, cx| {
            let item = workspace.active_item(cx).expect("active item");
            let button = crate::toolbar::render_toolbar_preview_button(
                item.as_ref(),
                workspace.weak_handle(),
                cx,
            );
            assert!(button.is_none(), "preview pane must not get a button");
        });

        preview.read_with(cx, |preview, _| {
            let content = preview
                .content
                .as_ref()
                .expect("preview has content")
                .as_ref()
                .expect("content is not an error");
            assert!(!content.is_empty(), "parsed HTML should have nodes");
        });
    }

    #[gpui::test]
    async fn preview_shows_waiting_message_without_language_server(cx: &mut TestAppContext) {
        init_test(cx);

        let fs = FakeFs::new(cx.executor());
        fs.insert_tree(path!("/root"), json!({ "doc.lex": "Title\n" })).await;
        let project = Project::test(fs, [path!("/root").as_ref()], cx).await;

        let (workspace, cx) =
            cx.add_window_view(|window, cx| Workspace::test_new(project.clone(), window, cx));

        let worktree_id = project.read_with(cx, |project, cx| {
            project.worktrees(cx).next().expect("has worktree").read(cx).id()
        });
        workspace
            .update_in(cx, |workspace, window, cx| {
                workspace.open_path(
                    ProjectPath {
                        worktree_id,
                        path: rel_path("doc.lex").into(),
                    },
                    None,
                    true,
                    window,
                    cx,
                )
            })
            .await
            .expect("open doc.lex");

        let preview = workspace.update_in(cx, |workspace, window, cx| {
            let buffer = LexPreviewView::resolve_active_item_as_lex_buffer(workspace, cx)
                .expect("active item is a lex buffer");
            let pane = workspace.active_pane().clone();
            LexPreviewView::open_preview_in_pane(workspace, buffer, pane, window, cx);
            workspace
                .active_pane()
                .read(cx)
                .items_of_type::<LexPreviewView>()
                .next()
                .expect("preview pane item exists")
        });
        cx.run_until_parked();
        preview.read_with(cx, |preview, _| {
            let error = preview
                .content
                .as_ref()
                .expect("preview has content")
                .as_ref()
                .expect_err("no server means a waiting message");
            assert!(error.contains("language server"), "got: {error}");
        });
    }
}
