## Typos
- [ ] emath-0.30.0/src/rect.rs. Documentation of `lerp_towards`.
    "Linearly towards" -> "Linearly interpolate towards"
- [ ] eframe-0.30.0/src/native/winit_integration.rs. Documentation of `UserEvent`. 
    "even" -> "event"
- [ ] egui-0.30.0/src/ui_builder.rs. Documentation of `max_rect`.
    "[`Ui`] will make room for it by expanding both `min_rect` and" end of sentence missing.
- [ ] egui-0.30.0/src/style.rs. Documentation of `text_styles`.
    "If you would like to overwrite app `text_styles`" end of sentence missing.
- [ ] egui-0.30.0/src/context.rs. Documentation of `show_viewport_immediate".
    "This means that the child viewport will not be repainted when the parent viewport is repainted, and vice versa." 
    This seems contradictory with the earlier sentences
    - "This is the easier type of viewport to use, but it is less performant at it requires both parent and child to repaint if any one of them needs repainting"
    - (from `show_viewport_deferred`) "The downside is that it will require the parent viewport (the caller) to repaint anytime the child is repainted, and vice versa."