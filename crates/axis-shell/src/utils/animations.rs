use libadwaita::prelude::*;
use gtk4_layer_shell::{Edge, LayerShell};
use std::time::Instant;

pub struct SlideAnimator;

impl SlideAnimator {
    /// A manual, time-based animator with easing curve.
    /// Independent of the GTK frame clock, making it ideal for Layer-Shell windows.
    pub fn slide_margin<W: IsA<gtk4::Window> + LayerShell + Clone + 'static>(
        window: &W,
        edge: Edge,
        target: i32,
        duration_ms: u32,
    ) {
        let start_margin = window.margin(edge);
        if start_margin == target {
            return;
        }

        let start_time = Instant::now();
        let duration = std::time::Duration::from_millis(duration_ms as u64);
        let window_c = window.clone();

        // 60 FPS Ziel (ca. 16ms)
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            let elapsed = start_time.elapsed();
            let progress = elapsed.as_secs_f64() / duration.as_secs_f64();
            
            if progress >= 1.0 {
                window_c.set_margin(edge, target);
                return gtk4::glib::ControlFlow::Break;
            }

            // Ease Out Cubic Formel: 1 - (1 - x)^3
            let eased = 1.0 - (1.0 - progress).powi(3);
            
            let current = start_margin + ((target - start_margin) as f64 * eased) as i32;
            window_c.set_margin(edge, current);
            
            gtk4::glib::ControlFlow::Continue
        });
    }
}
