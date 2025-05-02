use std::{rc::Rc, sync::Arc};

use futures_util::SinkExt;
use gpui::{
    div, prelude::FluentBuilder as _, px, rems, AnyElement, App, ClickEvent, Context, ElementId,
    Entity, InteractiveElement as _, IntoElement, ParentElement, Render, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Task, Window,
};

use crate::{h_flex, v_flex, ActiveTheme as _, Icon};

/// A tree component.
pub struct Tree {
    items: Vec<InnerTreeItem>,
    selected_index: Option<usize>,
}

impl Tree {
    pub fn new<I>(items: impl IntoIterator<Item = I>) -> Self
    where
        I: TreeItem + 'static,
    {
        let mut this = Self {
            items: Vec::new(),
            selected_index: None,
        };

        this.prepare_items(items);
        this
    }

    pub fn set_items<I>(
        &mut self,
        items: impl IntoIterator<Item = I>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) where
        I: TreeItem + 'static,
    {
        self.prepare_items(items);
        cx.notify();
    }

    fn prepare_items<I>(&mut self, items: impl IntoIterator<Item = I>)
    where
        I: TreeItem + 'static,
    {
        self.items.clear();
        // Convert nested items to flat list
        for item in items {
            build_items(&mut self.items, Rc::new(item), 0);
        }
    }
}

fn build_items(items: &mut Vec<InnerTreeItem>, item: Rc<dyn TreeItem>, depth: usize) {
    items.push(InnerTreeItem::new(items.len(), item.clone(), None, depth));
    for sub_item in item.children() {
        build_items(items, sub_item, depth + 1);
    }
}

pub trait TreeItem {
    fn label(&self) -> SharedString;
    fn icon(&self) -> Option<Icon> {
        None
    }

    fn disabled(&self) -> bool {
        false
    }

    fn children(&self) -> Vec<Rc<dyn TreeItem>> {
        vec![]
    }

    fn on_click(&self) -> Option<Arc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>> {
        None
    }
}

impl Render for Tree {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .p_4()
            .border_1()
            .border_color(cx.theme().border)
            .gap_1()
            .children(self.items.clone())
    }
}

#[derive(Clone, IntoElement)]
struct InnerTreeItem {
    id: ElementId,
    item: Rc<dyn TreeItem>,
    parent: Option<Rc<dyn TreeItem>>,
    depth: usize,
    selected: bool,
    on_click: Option<Arc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl InnerTreeItem {
    pub fn new(
        index: usize,
        item: Rc<dyn TreeItem>,
        parent: Option<Rc<dyn TreeItem>>,
        depth: usize,
    ) -> Self {
        Self {
            id: ElementId::Integer(index as u64),
            item,
            parent,
            depth,
            selected: false,
            on_click: None,
        }
    }

    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl RenderOnce for InnerTreeItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .id(self.id)
            .gap_1()
            .child(self.item.label())
            .px_3()
            .pl(rems(0.75) + rems(1.0 * self.depth as f32))
            .hover(|this| this.bg(cx.theme().list_hover))
            .when(self.selected, |this| this.bg(cx.theme().list_active))
            .when_some(self.item.on_click(), |this, on_click| {
                this.on_click(move |e, window, cx| {
                    on_click(&e, window, cx);
                })
            })
    }
}
