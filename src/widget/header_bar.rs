// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::cosmic_theme::{Density, Spacing};
use crate::{Element, theme, widget};
use apply::Apply;
use derive_setters::Setters;
use iced::Length;
use iced_core::event::{self, Event};
use iced_core::layout::Limits;
use iced_core::widget::tree::Tree;
use iced_core::{
    Clipboard, Layout, Point, Rectangle, Shell, Size, Vector, Widget, layout, mouse, overlay,
    renderer,
};
use std::borrow::Cow;

#[must_use]
pub fn header_bar<'a, Message>() -> HeaderBar<'a, Message> {
    HeaderBar {
        title: Cow::Borrowed(""),
        on_close: None,
        on_drag: None,
        on_maximize: None,
        on_minimize: None,
        on_right_click: None,
        start: Vec::new(),
        center: Vec::new(),
        end: Vec::new(),
        density: None,
        focused: false,
        maximized: false,
        sharp_corners: false,
        is_ssd: false,
        on_double_click: None,
        is_condensed: false,
        transparent: false,
        header_width: 0.0,
    }
}

#[derive(Setters)]
pub struct HeaderBar<'a, Message> {
    /// Defines the title of the window
    #[setters(skip)]
    title: Cow<'a, str>,

    /// A message emitted when the close button is pressed.
    #[setters(strip_option)]
    on_close: Option<Message>,

    /// A message emitted when dragged.
    #[setters(strip_option)]
    on_drag: Option<Message>,

    /// A message emitted when the maximize button is pressed.
    #[setters(strip_option)]
    on_maximize: Option<Message>,

    /// A message emitted when the minimize button is pressed.
    #[setters(strip_option)]
    on_minimize: Option<Message>,

    /// A message emitted when the header is double clicked,
    /// usually used to maximize the window.
    #[setters(strip_option)]
    on_double_click: Option<Message>,

    /// A message emitted when the header is right clicked.
    #[setters(strip_option)]
    on_right_click: Option<Message>,

    /// Elements packed at the start of the headerbar.
    #[setters(skip)]
    start: Vec<Element<'a, Message>>,

    /// Elements packed in the center of the headerbar.
    #[setters(skip)]
    center: Vec<Element<'a, Message>>,

    /// Elements packed at the end of the headerbar.
    #[setters(skip)]
    end: Vec<Element<'a, Message>>,

    /// Controls the density of the headerbar.
    #[setters(strip_option)]
    density: Option<Density>,

    /// Focused state of the window
    focused: bool,

    /// Maximized state of the window
    maximized: bool,

    /// Whether the corners of the window should be sharp
    sharp_corners: bool,

    /// HeaderBar used for server-side decorations
    is_ssd: bool,

    /// Whether the headerbar should be compact
    is_condensed: bool,

    /// Whether the headerbar should be transparent
    transparent: bool,

    /// The width of the header bar area in logical pixels, used for progressive collapse.
    header_width: f32,
}

impl<'a, Message: Clone + 'static> HeaderBar<'a, Message> {
    /// Defines the title of the window
    #[must_use]
    pub fn title(mut self, title: impl Into<Cow<'a, str>> + 'a) -> Self {
        self.title = title.into();
        self
    }

    /// Pushes an element to the start region.
    #[must_use]
    pub fn start(mut self, widget: impl Into<Element<'a, Message>> + 'a) -> Self {
        self.start.push(widget.into());
        self
    }

    /// Pushes an element to the center region.
    #[must_use]
    pub fn center(mut self, widget: impl Into<Element<'a, Message>> + 'a) -> Self {
        self.center.push(widget.into());
        self
    }

    /// Pushes an element to the end region.
    #[must_use]
    pub fn end(mut self, widget: impl Into<Element<'a, Message>> + 'a) -> Self {
        self.end.push(widget.into());
        self
    }

    #[must_use]
    #[inline]
    pub fn build(self) -> HeaderBarWidget<'a, Message> {
        self.into_widget()
    }
}

/// The header bar widget with custom layout that guarantees window controls
/// are never pushed off-screen.
///
/// Children: `[start, center, end, close, maximize, minimize]`
///
/// Layout priority:
/// - Close
/// - Maximize
/// - Minimize
/// - end section
/// - start sesction
/// - center/title — gets whatever space remains.
pub struct HeaderBarWidget<'a, Message> {
    /// Children: `[start, center, end, close, maximize, minimize]`.
    children: Vec<Element<'a, Message>>,
    /// The total height of the header bar including padding.
    height: f32,
    /// The padding [top, right, bottom, left].
    padding: [u16; 4],
    /// Whether the header bar is focused.
    focused: bool,
    /// Whether the window has sharp corners.
    sharp_corners: bool,
    /// Whether the header bar background is transparent.
    transparent: bool,
    /// Spacing between window control buttons.
    controls_spacing: u16,
    /// Spacing between items in the end region.
    end_spacing: u16,
}

impl<'a, Message: Clone + 'static> HeaderBar<'a, Message> {
    #[allow(clippy::too_many_lines)]
    fn into_widget(mut self) -> HeaderBarWidget<'a, Message> {
        let Spacing {
            space_xxxs,
            space_xxs,
            ..
        } = theme::spacing();

        // Take ownership of the regions to be packed.
        let start = std::mem::take(&mut self.start);
        let center = std::mem::take(&mut self.center);
        let end = std::mem::take(&mut self.end);

        // Padding depending on density and maximized state.
        let padding = match self.density.unwrap_or_else(crate::config::header_size) {
            Density::Compact => {
                if self.maximized {
                    [4, 8, 4, 8]
                } else {
                    [3, 7, 4, 7]
                }
            }
            _ => {
                if self.maximized {
                    [8, 8, 8, 8]
                } else {
                    [7, 7, 8, 7]
                }
            }
        };

        let w = self.header_width;
        // Title is hidden first when the window is too narrow (before window controls collapse).
        let show_title = !self.title.is_empty() && !self.is_condensed && (w == 0.0 || w >= 300.0);

        // Build the four section elements.
        let start_element: Element<'a, Message> = widget::row::with_children(start)
            .spacing(space_xxxs)
            .align_y(iced::Alignment::Center)
            .into();

        let center_element: Element<'a, Message> = if !center.is_empty() {
            widget::row::with_children(center)
                .spacing(space_xxxs)
                .align_y(iced::Alignment::Center)
                .into()
        } else if show_title {
            self.title_widget()
        } else {
            // Empty placeholder — will get zero or remaining width.
            widget::horizontal_space().width(Length::Shrink).into()
        };

        // Custom end elements (without window controls).
        let end_element: Element<'a, Message> = widget::row::with_children(end)
            .spacing(space_xxs)
            .align_y(iced::Alignment::Center)
            .into();

        // Individual window control buttons as separate children.
        let (close_element, maximize_element, minimize_element) = self.window_control_elements();

        let actual_padding = if self.is_ssd { [0, 8, 0, 8] } else { padding };
        // cosmic-comp's IcedElement buffer is 36px tall, making the height larger is fine since it
        // gets clamped.
        let height = 32.0 + padding[0] as f32 + padding[2] as f32;

        HeaderBarWidget {
            // Children: [start, center, end, close, maximize, minimize]
            children: vec![
                start_element,
                center_element,
                end_element,
                close_element,
                maximize_element,
                minimize_element,
            ],
            height,
            padding: actual_padding,
            focused: self.focused,
            sharp_corners: self.sharp_corners,
            transparent: self.transparent,
            controls_spacing: space_xxs,
            end_spacing: space_xxs,
        }
    }

    /// Converts the headerbar builder into an Iced element.
    pub fn view(self) -> Element<'a, Message> {
        let on_drag = self.on_drag.clone();
        let on_double_click = self.on_double_click.clone();
        let on_right_click = self.on_right_click.clone();

        let mut mouse_area = self
            .into_widget()
            .apply(Element::from)
            .apply(widget::mouse_area);

        if let Some(message) = on_drag {
            mouse_area = mouse_area.on_drag(message);
        }
        if let Some(message) = on_double_click {
            mouse_area = mouse_area.on_double_press(message);
        }
        if let Some(message) = on_right_click {
            mouse_area = mouse_area.on_right_press(message);
        }

        mouse_area.into()
    }

    fn title_widget(&mut self) -> Element<'a, Message> {
        let mut title = Cow::default();
        std::mem::swap(&mut title, &mut self.title);

        widget::text::heading(title)
            .ellipsize(iced_core::text::Ellipsize::End(
                iced_core::text::EllipsizeHeightLimit::Lines(1),
            ))
            .apply(widget::container)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    /// Creates individual window control button elements.
    fn window_control_elements(
        &mut self,
    ) -> (
        Element<'a, Message>,
        Element<'a, Message>,
        Element<'a, Message>,
    ) {
        macro_rules! icon {
            ($name:expr, $size:expr, $on_press:expr) => {{
                let icon = {
                    widget::icon::from_name($name)
                        .apply(widget::button::icon)
                        .padding(8)
                };

                icon.class(crate::theme::Button::HeaderBar)
                    .selected(self.focused)
                    .icon_size($size)
                    .on_press($on_press)
            }};
        }

        let w = self.header_width;
        let empty = || -> Element<'a, Message> {
            widget::horizontal_space().width(Length::Fixed(0.0)).into()
        };

        let close: Element<'a, Message> = self
            .on_close
            .take()
            .map(|m| -> Element<'a, Message> { icon!("window-close-symbolic", 16, m).into() })
            .unwrap_or_else(empty);

        let maximize: Element<'a, Message> = self
            .on_maximize
            .take()
            .filter(|_| w == 0.0 || w >= 200.0)
            .map(|m| -> Element<'a, Message> {
                if self.maximized {
                    icon!("window-restore-symbolic", 16, m).into()
                } else {
                    icon!("window-maximize-symbolic", 16, m).into()
                }
            })
            .unwrap_or_else(empty);

        let minimize: Element<'a, Message> = self
            .on_minimize
            .take()
            .filter(|_| w == 0.0 || w >= 250.0)
            .map(|m| -> Element<'a, Message> { icon!("window-minimize-symbolic", 16, m).into() })
            .unwrap_or_else(empty);

        (close, maximize, minimize)
    }
}

impl<Message: Clone + 'static> Widget<Message, crate::Theme, crate::Renderer>
    for HeaderBarWidget<'_, Message>
{
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(self.children.as_mut_slice());
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    /// Custom layout that guarantees the close button is never clipped.
    fn layout(&self, tree: &mut Tree, renderer: &crate::Renderer, limits: &Limits) -> layout::Node {
        let pad = self.padding;
        let pad_h = pad[1] as f32 + pad[3] as f32;
        let pad_v = pad[0] as f32 + pad[2] as f32;
        let actual_height = self.height.min(limits.max().height);
        let content_height = actual_height - pad_v;
        let ctrl_spacing = self.controls_spacing as f32;
        let end_spacing = self.end_spacing as f32;

        let total_width = limits
            .resolve(Length::Fill, Length::Fixed(actual_height), Size::ZERO)
            .width;
        let available_width = (total_width - pad_h).max(0.0);

        // Reserve spacing for the 2 section gaps: [start] [center] [end+controls]
        let mut remaining = available_width;

        // Helper to measure a child and subtract from remaining.
        let measure = |child_idx: usize,
                       tree: &mut Tree,
                       renderer: &crate::Renderer,
                       children: &[Element<'_, Message>],
                       remaining: &mut f32|
         -> layout::Node {
            let lim = Limits::new(Size::ZERO, Size::new((*remaining).max(0.0), content_height));
            let node = children[child_idx].as_widget().layout(
                &mut tree.children[child_idx],
                renderer,
                &lim,
            );
            *remaining -= node.size().width;
            node
        };

        // Close button (highest priority)
        let mut close_node = measure(3, tree, renderer, &self.children, &mut remaining);
        let close_w = close_node.size().width;

        // Maximize
        if close_w > 0.0 {
            remaining -= ctrl_spacing;
        }
        let mut max_node = measure(4, tree, renderer, &self.children, &mut remaining);
        let max_w = max_node.size().width;

        // Minimize
        if max_w > 0.0 {
            remaining -= ctrl_spacing;
        }
        let mut min_node = measure(5, tree, renderer, &self.children, &mut remaining);
        let min_w = min_node.size().width;

        let ctrl_gap_1 = if min_w > 0.0 && (max_w > 0.0 || close_w > 0.0) {
            ctrl_spacing
        } else {
            0.0
        };
        let ctrl_gap_2 = if max_w > 0.0 && close_w > 0.0 {
            ctrl_spacing
        } else {
            0.0
        };
        let controls_total = min_w + ctrl_gap_1 + max_w + ctrl_gap_2 + close_w;

        // custom end elements
        let end_gap = if controls_total > 0.0 {
            end_spacing
        } else {
            0.0
        };
        remaining -= end_gap;
        let mut end_node = measure(2, tree, renderer, &self.children, &mut remaining);
        let end_w = end_node.size().width;
        let actual_end_gap = if end_w > 0.0 && controls_total > 0.0 {
            end_spacing
        } else {
            0.0
        };
        if actual_end_gap < end_gap {
            remaining += end_gap - actual_end_gap;
        }

        // start section
        let mut start_node = measure(0, tree, renderer, &self.children, &mut remaining);
        let start_w = start_node.size().width;

        // center sectino
        let center_limits = Limits::new(Size::ZERO, Size::new(remaining.max(0.0), content_height));
        let mut center_node =
            self.children[1]
                .as_widget()
                .layout(&mut tree.children[1], renderer, &center_limits);

        let start_y = pad[0] as f32;
        let vert_center = |node: &layout::Node| -> f32 {
            start_y + (content_height - node.size().height).max(0.0) / 2.0
        };

        let left_x = pad[3] as f32;
        start_node.move_to_mut(Point::new(left_x, vert_center(&start_node)));
        let center_x = left_x + start_w;
        center_node.move_to_mut(Point::new(center_x, vert_center(&center_node)));
        let right_edge = total_width - pad[1] as f32;
        let close_x = right_edge - close_w;
        close_node.move_to_mut(Point::new(close_x, vert_center(&close_node)));
        let max_x = close_x - ctrl_gap_2 - max_w;
        max_node.move_to_mut(Point::new(max_x, vert_center(&max_node)));
        let min_x = max_x - ctrl_gap_1 - min_w;
        min_node.move_to_mut(Point::new(min_x, vert_center(&min_node)));
        let end_x = min_x - actual_end_gap - end_w;
        end_node.move_to_mut(Point::new(end_x, vert_center(&end_node)));

        let parent_size = Size::new(total_width, actual_height);
        layout::Node::with_children(
            parent_size,
            vec![
                start_node,
                center_node,
                end_node,
                close_node,
                max_node,
                min_node,
            ],
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut crate::Renderer,
        theme: &crate::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        // Compute the header bar container style for background and text/icon colors.
        let bounds = layout.bounds();
        let appearance = crate::theme::Container::HeaderBar {
            focused: self.focused,
            sharp_corners: self.sharp_corners,
            transparent: self.transparent,
        };
        let container_style =
            <crate::Theme as iced::widget::container::Catalog>::style(theme, &appearance);

        // Draw the background.
        iced_core::renderer::Renderer::fill_quad(
            renderer,
            renderer::Quad {
                bounds,
                border: container_style.border,
                shadow: container_style.shadow,
            },
            container_style
                .background
                .unwrap_or(iced::Background::Color(iced::Color::TRANSPARENT)),
        );

        // Propagate text and icon colors from the container style to children,
        // matching the behavior of iced's Container widget.
        let child_style = renderer::Style {
            icon_color: container_style.icon_color.unwrap_or(style.icon_color),
            text_color: container_style.text_color.unwrap_or(style.text_color),
            scale_factor: style.scale_factor,
        };

        // Draw each child section.
        for ((child, state), c_layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child.as_widget().draw(
                state,
                renderer,
                theme,
                &child_style,
                c_layout,
                cursor,
                viewport,
            );
        }
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &crate::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        let mut status = event::Status::Ignored;
        for ((child, child_state), c_layout) in self
            .children
            .iter_mut()
            .zip(&mut state.children)
            .zip(layout.children())
        {
            let child_status = child.as_widget_mut().on_event(
                child_state,
                event.clone(),
                c_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
            if child_status == event::Status::Captured {
                status = event::Status::Captured;
            }
        }
        status
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &crate::Renderer,
    ) -> mouse::Interaction {
        for ((child, child_state), c_layout) in self
            .children
            .iter()
            .zip(&state.children)
            .zip(layout.children())
        {
            let interaction = child.as_widget().mouse_interaction(
                child_state,
                c_layout,
                cursor,
                viewport,
                renderer,
            );
            if interaction != mouse::Interaction::None {
                return interaction;
            }
        }
        mouse::Interaction::None
    }

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &crate::Renderer,
        operation: &mut dyn iced_core::widget::Operation<()>,
    ) {
        for ((child, child_state), c_layout) in self
            .children
            .iter()
            .zip(&mut state.children)
            .zip(layout.children())
        {
            child
                .as_widget()
                .operate(child_state, c_layout, renderer, operation);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &crate::Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, crate::Theme, crate::Renderer>> {
        overlay::from_children(
            self.children.as_mut_slice(),
            state,
            layout,
            renderer,
            translation,
        )
    }

    fn drag_destinations(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        renderer: &crate::Renderer,
        dnd_rectangles: &mut iced_core::clipboard::DndDestinationRectangles,
    ) {
        for ((child, child_state), c_layout) in self
            .children
            .iter()
            .zip(&state.children)
            .zip(layout.children())
        {
            child
                .as_widget()
                .drag_destinations(child_state, c_layout, renderer, dnd_rectangles);
        }
    }

    #[cfg(feature = "a11y")]
    fn a11y_nodes(
        &self,
        layout: Layout<'_>,
        state: &Tree,
        p: mouse::Cursor,
    ) -> iced_accessibility::A11yTree {
        use iced_accessibility::A11yTree;
        A11yTree::join(
            self.children
                .iter()
                .zip(layout.children())
                .zip(state.children.iter())
                .map(|((c, c_layout), state)| c.as_widget().a11y_nodes(c_layout, state, p)),
        )
    }
}

impl<'a, Message: Clone + 'static> From<HeaderBar<'a, Message>> for Element<'a, Message> {
    fn from(headerbar: HeaderBar<'a, Message>) -> Self {
        headerbar.view()
    }
}

impl<'a, Message: Clone + 'static> From<HeaderBarWidget<'a, Message>> for Element<'a, Message> {
    fn from(headerbar: HeaderBarWidget<'a, Message>) -> Self {
        Element::new(headerbar)
    }
}
