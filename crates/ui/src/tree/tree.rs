use std::{rc::Rc, sync::Arc};

use futures_util::SinkExt;
use gpui::{
    div, AnyElement, App, ClickEvent, Context, ElementId, Entity, IntoElement, ParentElement,
    Render, RenderOnce, SharedString, Styled, Task, Window,
};

use crate::{h_flex, v_flex, Icon};

/// A tree component.
pub struct Tree {
    items: Vec<InnerTreeItem>,
    selected_index: Option<usize>,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected_index: None,
        }
    }

    pub fn set_items(
        &mut self,
        items: Vec<Rc<dyn TreeItem>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.items.clear();
        // Convert nested items to flat list
        for item in items {
            build_items(&mut self.items, item, 0);
        }

        cx.notify();
    }
}

fn build_items(items: &mut Vec<InnerTreeItem>, item: Rc<dyn TreeItem>, depth: usize) {
    items.push(InnerTreeItem::new(item.clone(), None, depth));
    for sub_item in item.children() {
        build_items(items, sub_item, depth + 1);
    }
}

pub trait TreeItem {
    fn id(&self) -> ElementId;
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
    fn on_click(&self) -> Option<Arc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>;
}

impl Render for Tree {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_1().children(self.items.clone())
    }
}

#[derive(Clone, IntoElement)]
struct InnerTreeItem {
    item: Rc<dyn TreeItem>,
    parent: Option<Rc<dyn TreeItem>>,
    depth: usize,
    on_click: Option<Arc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl InnerTreeItem {
    pub fn new(item: Rc<dyn TreeItem>, parent: Option<Rc<dyn TreeItem>>, depth: usize) -> Self {
        Self {
            item,
            parent,
            depth,
            on_click: None,
        }
    }
}

impl RenderOnce for InnerTreeItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex().gap_1().child(self.item.label())
    }
}
