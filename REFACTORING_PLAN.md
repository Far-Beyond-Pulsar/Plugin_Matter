# Plugin_Matter Refactoring Plan - WGPUI-Component Integration

## Phase 2.1: UI Modernization

### Goals
1. Replace custom UI with WGPUI-Component widgets
2. Follow Blueprint Editor's patterns exactly
3. Use proper theme colors via `cx.theme()`
4. Implement trait-based modular tool system

---

## Step 1: Toolbar Refactoring

### Current State
- Custom `render_toolbar()` function returning divs
- Hardcoded colors (rgb(0x3c3c3c), etc.)
- No proper button components

### Target State (Following Blueprint Pattern)
```rust
use ui::{
    button::{Button, ButtonVariants as _},
    h_flex, ActiveTheme, Icon, IconName,
};

pub struct ToolbarRenderer;

impl ToolbarRenderer {
    pub fn render(panel: &MatterEditorPanel, cx: &mut Context<MatterEditorPanel>) -> impl IntoElement {
        let active_tool = panel.document.tool_state.active_tool;
        let can_undo = panel.document.history.can_undo();
        let can_redo = panel.document.history.can_redo();
        
        h_flex()
            .w_full()
            .h(px(48.0))
            .px_4()
            .gap_3()
            .items_center()
            .bg(cx.theme().sidebar.opacity(0.98))
            .border_b_1()
            .border_color(cx.theme().border.opacity(0.8))
            .shadow_sm()
            // Undo/Redo group
            .child(h_flex().gap_1p5().items_center()
                .child(Button::new("toolbar-undo")
                    .icon(IconName::Undo)
                    .disabled(!can_undo)
                    .tooltip("Undo (Cmd+Z)")
                    .on_click(cx.listener(|panel, _, _, cx| {
                        let _ = panel.document.history.undo();
                        cx.notify();
                    })))
                .child(Button::new("toolbar-redo")
                    .icon(IconName::Redo)
                    .disabled(!can_redo)
                    .tooltip("Redo (Cmd+Shift+Z)")
                    .on_click(cx.listener(|panel, _, _, cx| {
                        let _ = panel.document.history.redo();
                        cx.notify();
                    })))
            )
            .child(toolbar_separator(cx))
            // Tools group
            .child(h_flex().gap_1p5().items_center()
                .child(tool_button("Paint", IconName::Brush, ActiveTool::Paint, active_tool, cx))
                .child(tool_button("Erase", IconName::Eraser, ActiveTool::Erase, active_tool, cx))
                .child(tool_button("Fill", IconName::PaintBucket, ActiveTool::Fill, active_tool, cx))
                .child(tool_button("Eyedropper", IconName::Eyedropper, ActiveTool::Eyedropper, active_tool, cx))
                .child(tool_button("Pan", IconName::Hand, ActiveTool::Pan, active_tool, cx))
            )
    }
}

fn tool_button(
    label: &str,
    icon: IconName,
    tool: ActiveTool,
    active: ActiveTool,
    cx: &Context<MatterEditorPanel>
) -> Button {
    let btn = Button::new(format!("tool-{:?}", tool))
        .icon(icon)
        .tooltip(format!("{} ({})", label, tool.hotkey()))
        .on_click(cx.listener(move |panel, _, _, cx| {
            panel.document.tool_state.active_tool = tool;
            cx.notify();
        }));
    
    if active == tool {
        btn.primary()
    } else {
        btn
    }
}

fn toolbar_separator(cx: &Context<MatterEditorPanel>) -> impl IntoElement {
    div()
        .h(px(24.0))
        .w(px(1.0))
        .bg(cx.theme().border.opacity(0.5))
}
```

---

## Step 2: Panel System Refactoring

### Current State
- Function-based `render_layer_panel()` and `render_color_panel()`
- Not using Panel trait or workspace integration

### Target State
```rust
// Separate panel modules
mod panels {
    pub mod layers;
    pub mod properties;
    pub mod canvas;
}

// Each panel implements proper structure
pub struct LayersPanel {
    document: Entity<Document>,
}

impl LayersPanel {
    pub fn render(&self, cx: &mut Context<Self>) -> impl IntoElement {
        use ui::{v_flex, Button, IconName};
        
        v_flex()
            .size_full()
            .p_2()
            .gap_2()
            .bg(cx.theme().sidebar)
            .child(
                // Header with title and new layer button
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_sm().font_semibold().child("Layers"))
                    .child(
                        Button::new("new-layer")
                            .icon(IconName::Plus)
                            .compact()
                            .tooltip("New Layer")
                            .on_click(cx.listener(|panel, _, win, cx| {
                                panel.create_layer(win, cx);
                            }))
                    )
            )
            .child(/* layer list */)
    }
}
```

---

## Step 3: Tool Trait System

### Design
```rust
pub trait Tool: Send + Sync {
    /// Tool name (for display)
    fn name(&self) -> &str;
    
    /// Tool icon
    fn icon(&self) -> IconName;
    
    /// Keyboard shortcut
    fn hotkey(&self) -> &str;
    
    /// Handle mouse down event
    fn on_mouse_down(&mut self, event: &MouseDownEvent, state: &mut ToolState, doc: &mut Document) -> Result<()>;
    
    /// Handle mouse move event
    fn on_mouse_move(&mut self, event: &MouseMoveEvent, state: &mut ToolState, doc: &mut Document) -> Result<()>;
    
    /// Handle mouse up event
    fn on_mouse_up(&mut self, event: &MouseUpEvent, state: &mut ToolState, doc: &mut Document) -> Result<Option<Box<dyn Command>>>;
    
    /// Render tool cursor overlay
    fn render_cursor(&self, pos: Point<Pixels>, state: &ToolState) -> Option<impl IntoElement>;
    
    /// Render tool-specific settings panel
    fn render_settings(&self, state: &ToolState, cx: &mut Context<Self>) -> impl IntoElement;
}

// Individual tool implementations
pub struct PaintTool {
    current_stroke: Option<Stroke>,
}

impl Tool for PaintTool {
    fn name(&self) -> &str { "Paint Brush" }
    fn icon(&self) -> IconName { IconName::Brush }
    fn hotkey(&self) -> &str { "B" }
    
    fn on_mouse_down(&mut self, event: &MouseDownEvent, state: &mut ToolState, doc: &mut Document) -> Result<()> {
        // Start new stroke
        self.current_stroke = Some(Stroke::new(state.foreground_bytes()));
        Ok(())
    }
    
    fn on_mouse_up(&mut self, event: &MouseUpEvent, state: &mut ToolState, doc: &mut Document) -> Result<Option<Box<dyn Command>>> {
        // Finalize and return command
        if let Some(stroke) = self.current_stroke.take() {
            let tiles = stroke.rasterize(/* ... */);
            Ok(Some(Box::new(PaintStrokeCommand::new(/* ... */))))
        } else {
            Ok(None)
        }
    }
    
    // ...
}
```

---

## Step 4: Theme Integration

### Replace All Hardcoded Colors

**Before:**
```rust
.bg(rgb(0x1e1e1e))
.text_color(rgb(0x999999))
.border_color(rgb(0x3c3c3c))
```

**After:**
```rust
.bg(cx.theme().background)
.text_color(cx.theme().muted_foreground)
.border_color(cx.theme().border)
```

### Theme Colors Available
- `background`, `foreground`
- `primary`, `secondary`, `accent`
- `success`, `warning`, `danger`, `info`
- `muted`, `muted_foreground`
- `border`, `ring`
- `sidebar`, `panel`

---

## Implementation Order

1. ✅ Create toolbar.rs with ToolbarRenderer
2. ✅ Create panels/layers.rs with proper Panel structure
3. ✅ Create panels/properties.rs for color/brush settings
4. ✅ Create tools/trait.rs with Tool trait
5. ✅ Implement tools/paint.rs, tools/erase.rs, etc.
6. ✅ Update panel.rs to use new components
7. ✅ Replace all hardcoded colors with theme
8. ✅ Test and verify visual consistency

---

## Benefits

- **Consistent UI**: Matches engine's visual language
- **Theme Support**: Automatically adapts to theme changes
- **Modular Tools**: Easy to add new tools as plugins
- **Maintainable**: Standard patterns across codebase
- **Professional**: Uses battle-tested UI components
