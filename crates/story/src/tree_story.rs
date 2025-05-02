use std::{collections::HashMap, rc::Rc};

use gpui::{
    div, impl_internal_actions, prelude::FluentBuilder, relative, App, AppContext, ClickEvent,
    Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render, SharedString,
    Styled, Window,
};

use gpui_component::{
    badge::Badge,
    blue_500,
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    divider::Divider,
    h_flex,
    popup_menu::PopupMenuExt,
    sidebar::{
        Sidebar, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem,
        SidebarToggleButton,
    },
    switch::Switch,
    tree::{Tree, TreeItem},
    v_flex, white, ActiveTheme, Icon, IconName, Side, Sizable,
};
use serde::Deserialize;

use crate::section;

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct SelectCompany(SharedString);

impl_internal_actions!(sidebar_story, [SelectCompany]);

#[derive(Clone, Deserialize)]
pub struct File {
    name: SharedString,
    children: Vec<File>,
}

impl File {
    fn new(name: SharedString) -> Self {
        Self {
            name,
            children: vec![],
        }
    }

    fn children(mut self, children: impl Into<Vec<File>>) -> Self {
        self.children = children.into();
        self
    }
}

impl TreeItem for File {
    fn label(&self) -> SharedString {
        self.name.clone()
    }

    fn children(&self) -> Vec<Rc<dyn TreeItem>> {
        self.children
            .iter()
            .map(|child| Rc::new(child.clone()) as Rc<dyn TreeItem>)
            .collect::<Vec<_>>()
    }
}

pub struct TreeStory {
    focus_handle: FocusHandle,
    files: Vec<File>,
    file_tree: Entity<Tree>,
}

impl TreeStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let files = vec![File {
            name: "root".into(),
            children: vec![
                File::new("src".into()).children(vec![
                    File::new("lib.rs".into()),
                    File::new("tree".into()).children(vec![
                        File::new("mod.rs".into()),
                        File::new("tree_item.rs".into()),
                    ]),
                ]),
                File::new("examples".into()).children(vec![
                    File::new("hello.rs".into()),
                    File::new("simple.rs".into()),
                ]),
                File::new("tests".into()).children(vec![File::new("test.rs".into())]),
                File::new("Cargo.toml".into()),
                File::new("README.md".into()),
            ],
        }];

        let file_tree = cx.new({
            let files = files.clone();
            move |_| Tree::new(files)
        });

        Self {
            file_tree,
            files,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl super::Story for TreeStory {
    fn title() -> &'static str {
        "Tree"
    }

    fn description() -> &'static str {
        "A tree component that displays hierarchical data structures."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl Focusable for TreeStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TreeStory {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .gap_1()
            .child(section("A file tree").child(self.file_tree.clone()))
    }
}
