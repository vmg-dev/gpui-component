use std::{
    rc::Rc,
    sync::LazyLock,
    time::{Duration, Instant},
};

use gpui::{
    AbsoluteLength, AnyElement, App, AppContext as _, ClickEvent, DefiniteLength, DismissEvent, DragMoveEvent,
    Edges, Empty, EventEmitter, FocusHandle, InteractiveElement as _, IntoElement, KeyBinding,
    MouseButton, MouseDownEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, RenderOnce,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, WindowControlArea,
    anchored, div, point,
    prelude::FluentBuilder as _, px,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    animation::cubic_bezier,
    ActiveTheme, FocusTrapElement as _, IconName, Placement, Sizable, StyledExt as _,
    WindowExt as _,
    actions::Cancel,
    button::{Button, ButtonVariants as _},
    dialog::overlay_color,
    h_flex,
    scroll::ScrollableElement as _,
    title_bar::TITLE_BAR_HEIGHT,
    v_flex,
};

const CONTEXT: &str = "Sheet";
pub(crate) static SHEET_ANIMATION_DURATION: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs_f64(0.15));
const SHEET_DISMISS_VELOCITY: f32 = 1400.0;

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Cancel, Some(CONTEXT))])
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SheetAnimationPhase {
    Entering(Instant),
    Closing(Instant),
}

#[derive(Clone, Copy, Debug, Default)]
struct SheetDrag;

impl Render for SheetDrag {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone, Copy, Debug)]
struct SheetDragState {
    dragging: bool,
    start_position: Point<Pixels>,
    current_offset: Pixels,
    last_offset: Pixels,
    last_sample_at: Instant,
    velocity: f32,
}

impl Default for SheetDragState {
    fn default() -> Self {
        Self {
            dragging: false,
            start_position: Point::default(),
            current_offset: px(0.0),
            last_offset: px(0.0),
            last_sample_at: Instant::now(),
            velocity: 0.0,
        }
    }
}

impl SheetDragState {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn outward_amount(offset: Pixels, placement: Placement) -> f32 {
        match placement {
            Placement::Right | Placement::Bottom => offset.max(px(0.0)).as_f32(),
            Placement::Left | Placement::Top => (-offset).max(px(0.0)).as_f32(),
        }
    }

    fn clamp_offset(delta: Pixels, placement: Placement) -> Pixels {
        match placement {
            Placement::Right | Placement::Bottom => delta.max(px(0.0)),
            Placement::Left | Placement::Top => delta.min(px(0.0)),
        }
    }

    fn begin_drag(&mut self, position: Point<Pixels>) {
        self.dragging = true;
        self.start_position = position;
        self.current_offset = px(0.0);
        self.last_offset = px(0.0);
        self.last_sample_at = Instant::now();
        self.velocity = 0.0;
    }

    fn update_drag(&mut self, position: Point<Pixels>, placement: Placement) {
        if !self.dragging {
            return;
        }

        let delta = match placement {
            Placement::Right | Placement::Left => position.x - self.start_position.x,
            Placement::Top | Placement::Bottom => position.y - self.start_position.y,
        };
        let now = Instant::now();
        let next_offset = Self::clamp_offset(delta, placement);
        let elapsed = now.duration_since(self.last_sample_at).as_secs_f32();

        if elapsed > 0.0 {
            self.velocity = (next_offset - self.last_offset).as_f32() / elapsed;
        }

        self.current_offset = next_offset;
        self.last_offset = next_offset;
        self.last_sample_at = now;
    }

    fn dismiss_progress(&self, placement: Placement, extent: Pixels) -> f32 {
        let extent = extent.max(px(1.0)).as_f32();
        (Self::outward_amount(self.current_offset, placement) / extent).clamp(0.0, 1.0)
    }

    fn should_dismiss(&self, placement: Placement, extent: Pixels) -> bool {
        let outward_offset = Self::outward_amount(self.current_offset, placement);
        let outward_velocity = Self::outward_amount(px(self.velocity), placement);
        outward_offset > extent.as_f32() * 0.35 || outward_velocity > SHEET_DISMISS_VELOCITY
    }
}

/// The settings for sheets.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SheetSettings {
    /// The margin top for the sheet, default is [`TITLE_BAR_HEIGHT`].
    pub margin_top: Pixels,
}

impl Default for SheetSettings {
    fn default() -> Self {
        Self {
            margin_top: TITLE_BAR_HEIGHT,
        }
    }
}

/// Sheet component that slides in from the side of the window.
#[derive(IntoElement)]
pub struct Sheet {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) placement: Placement,
    pub(crate) size: DefiniteLength,
    pub(crate) animation_phase: SheetAnimationPhase,
    resizable: bool,
    on_close: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
    title: Option<AnyElement>,
    footer: Option<AnyElement>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    overlay: bool,
    overlay_closable: bool,
}

impl Sheet {
    /// Creates a new Sheet.
    pub fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            placement: Placement::Right,
            size: DefiniteLength::Absolute(px(350.).into()),
            animation_phase: SheetAnimationPhase::Entering(Instant::now()),
            resizable: true,
            title: None,
            footer: None,
            style: StyleRefinement::default(),
            children: Vec::new(),
            overlay: true,
            overlay_closable: true,
            on_close: Rc::new(|_, _, _| {}),
        }
    }

    /// Sets the title of the sheet.
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self
    }

    /// Set the footer of the sheet.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Sets the size of the sheet, default is 350px.
    pub fn size(mut self, size: impl Into<DefiniteLength>) -> Self {
        self.size = size.into();
        self
    }

    /// Sets whether the sheet is resizable, default is `true`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Set whether the sheet should have an overlay, default is `true`.
    pub fn overlay(mut self, overlay: bool) -> Self {
        self.overlay = overlay;
        self
    }

    /// Set whether the sheet should be closable by clicking the overlay, default is `true`.
    pub fn overlay_closable(mut self, overlay_closable: bool) -> Self {
        self.overlay_closable = overlay_closable;
        self
    }

    /// Listen to the close event of the sheet.
    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Rc::new(on_close);
        self
    }
}

impl EventEmitter<DismissEvent> for Sheet {}
impl ParentElement for Sheet {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}
impl Styled for Sheet {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Sheet {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let window_paddings = crate::window_border::window_paddings(window);
        let size = window.viewport_size()
            - gpui::size(
                window_paddings.left + window_paddings.right,
                window_paddings.top + window_paddings.bottom,
            );
        let top = cx.theme().sheet.margin_top;
        let on_close = self.on_close.clone();
        let drag_state = window.use_keyed_state("sheet-drag", cx, |_, _| SheetDragState::default());
        let animation_duration = *SHEET_ANIMATION_DURATION;
        let ease_out = cubic_bezier(0.0, 0.0, 0.2, 1.0);
        let elapsed = match self.animation_phase {
            SheetAnimationPhase::Entering(started_at) | SheetAnimationPhase::Closing(started_at) => {
                started_at.elapsed().as_secs_f32()
            }
        };
        let progress = (elapsed / animation_duration.as_secs_f32()).clamp(0.0, 1.0);
        let easing = ease_out(progress);
        let is_closing = matches!(self.animation_phase, SheetAnimationPhase::Closing(_));
        let visibility = if is_closing { 1.0 - easing } else { easing };

        if progress < 1.0 {
            window.request_animation_frame();
        }

        let base_size = window.text_style().font_size;
        let rem_size = window.rem_size();
        let mut paddings = Edges::all(px(16.));
        if let Some(pl) = self.style.padding.left {
            paddings.left = pl.to_pixels(base_size, rem_size);
        }
        if let Some(pr) = self.style.padding.right {
            paddings.right = pr.to_pixels(base_size, rem_size);
        }
        if let Some(pt) = self.style.padding.top {
            paddings.top = pt.to_pixels(base_size, rem_size);
        }
        if let Some(pb) = self.style.padding.bottom {
            paddings.bottom = pb.to_pixels(base_size, rem_size);
        }

        if matches!(self.animation_phase, SheetAnimationPhase::Entering(_))
            && drag_state.read(cx).current_offset.abs() > px(0.5)
            && !drag_state.read(cx).dragging
        {
            drag_state.update(cx, |state, _| state.reset());
        }

        let sheet_extent = match self.placement {
            Placement::Right | Placement::Left => {
                self.size
                    .to_pixels(AbsoluteLength::Pixels(size.width), rem_size)
            }
            Placement::Top | Placement::Bottom => {
                self.size
                    .to_pixels(AbsoluteLength::Pixels(size.height), rem_size)
            }
        };
        let drag_offset = drag_state.read(cx).current_offset;
        let animated_offset = sheet_extent * (1.0 - visibility);
        let overlay_visibility =
            visibility * (1.0 - drag_state.read(cx).dismiss_progress(self.placement, sheet_extent));

        let sheet_panel = {
            let sheet_panel = v_flex()
                .id("sheet")
                .key_context(CONTEXT)
                .track_focus(&self.focus_handle)
                .focus_trap("sheet", &self.focus_handle)
                .on_action({
                    let on_close = self.on_close.clone();
                    move |_: &Cancel, window, cx| {
                        cx.propagate();

                        window.close_sheet(cx);
                        on_close(&ClickEvent::default(), window, cx);
                    }
                })
                .absolute()
                .occlude()
                .bg(cx.theme().background)
                .border_color(cx.theme().border)
                .shadow_xl()
                .refine_style(&self.style);

            let sheet_panel = if self.placement.is_horizontal() {
                sheet_panel.w(self.size)
            } else {
                sheet_panel.h(self.size)
            };

            let sheet_panel = match self.placement {
                Placement::Top => sheet_panel
                    .top(top - animated_offset + drag_offset)
                    .left_0()
                    .right_0()
                    .border_b_1(),
                Placement::Right => sheet_panel
                    .top(top)
                    .right(-animated_offset + drag_offset)
                    .bottom_0()
                    .border_l_1(),
                Placement::Bottom => sheet_panel
                    .bottom(-animated_offset + drag_offset)
                    .left_0()
                    .right_0()
                    .border_t_1(),
                Placement::Left => sheet_panel
                    .top(top)
                    .left(-animated_offset + drag_offset)
                    .bottom_0()
                    .border_r_1(),
            };

            sheet_panel
                .child(
                    // TitleBar
                    h_flex()
                        .id("sheet-drag-handle")
                        .justify_between()
                        .pl_4()
                        .pr_3()
                        .py_2()
                        .w_full()
                        .cursor_grab()
                        .font_semibold()
                        .on_mouse_down(MouseButton::Left, {
                            let drag_state = drag_state.clone();
                            move |event: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                                drag_state.update(cx, |state, _| {
                                    state.begin_drag(event.position);
                                });
                            }
                        })
                        .on_drag(SheetDrag, |drag, _, _, cx| {
                            cx.stop_propagation();
                            cx.new(|_| *drag)
                        })
                        .on_drag_move({
                            let drag_state = drag_state.clone();
                            let placement = self.placement;
                            move |event: &DragMoveEvent<SheetDrag>, window: &mut Window, cx: &mut App| {
                                drag_state.update(cx, |state, _| {
                                    state.update_drag(event.event.position, placement);
                                });
                                window.refresh();
                            }
                        })
                        .on_mouse_up(MouseButton::Left, {
                            let drag_state = drag_state.clone();
                            let placement = self.placement;
                            let on_close = on_close.clone();
                            move |_: &MouseUpEvent, window: &mut Window, cx: &mut App| {
                                let should_dismiss =
                                    drag_state.read(cx).should_dismiss(placement, sheet_extent);

                                drag_state.update(cx, |state, _| {
                                    state.dragging = false;
                                });

                                if should_dismiss {
                                    window.close_sheet(cx);
                                    on_close(&ClickEvent::default(), window, cx);
                                } else {
                                    drag_state.update(cx, |state, _| state.reset());
                                }

                                window.refresh();
                            }
                        })
                        .on_mouse_up_out(MouseButton::Left, {
                            let drag_state = drag_state.clone();
                            let placement = self.placement;
                            let on_close = on_close.clone();
                            move |_: &MouseUpEvent, window: &mut Window, cx: &mut App| {
                                let should_dismiss =
                                    drag_state.read(cx).should_dismiss(placement, sheet_extent);

                                drag_state.update(cx, |state, _| {
                                    state.dragging = false;
                                });

                                if should_dismiss {
                                    window.close_sheet(cx);
                                    on_close(&ClickEvent::default(), window, cx);
                                } else {
                                    drag_state.update(cx, |state, _| state.reset());
                                }

                                window.refresh();
                            }
                        })
                        .child(self.title.unwrap_or(div().into_any_element()))
                        .child(
                            Button::new("close")
                                .small()
                                .ghost()
                                .icon(IconName::Close)
                                .on_click(move |_, window, cx| {
                                    window.close_sheet(cx);
                                    on_close(&ClickEvent::default(), window, cx);
                                }),
                        ),
                )
                .child(
                    div().flex_1().overflow_hidden().child(
                        // Body
                        v_flex()
                            .size_full()
                            .overflow_y_scrollbar()
                            .pl(paddings.left)
                            .pr(paddings.right)
                            .children(self.children),
                    ),
                )
                .when_some(self.footer, |this, footer| {
                    // Footer
                    this.child(
                        h_flex()
                            .justify_between()
                            .px_4()
                            .py_3()
                            .w_full()
                            .child(footer),
                    )
                })
                .on_any_mouse_down(|_, _, cx| {
                    cx.stop_propagation();
                })
        };

        anchored()
            .position(point(window_paddings.left, window_paddings.top))
            .snap_to_window()
            .child(
                div()
                    .occlude()
                    .w(size.width)
                    .h(size.height)
                    .bg(overlay_color(self.overlay, cx).opacity(overlay_visibility))
                    .when(self.overlay, |this| {
                        this.window_control_area(WindowControlArea::Drag)
                            .on_any_mouse_down({
                                let on_close = self.on_close.clone();
                                move |event, window, cx| {
                                    if event.position.y < top {
                                        return;
                                    }

                                    cx.stop_propagation();
                                    if self.overlay_closable && event.button == MouseButton::Left {
                                        window.close_sheet(cx);
                                        on_close(&ClickEvent::default(), window, cx);
                                    }
                                }
                            })
                    })
                    .child(sheet_panel),
            )
    }
}
