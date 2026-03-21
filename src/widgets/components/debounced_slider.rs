use super::icon_slider::IconSlider;
use crate::store::ServiceStore;
use async_channel::Sender;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

/// A slider that stays in sync with a service via debounced bidirectional binding.
///
/// Internally wraps an IconSlider and handles:
/// - Store → Slider (with threshold-based debounce)
/// - Slider → Command (on user interaction)
/// - Optional data_changed callback for side effects (icon update, etc.)
pub struct DebouncedSlider<T, C>
where
    T: Clone + PartialEq + 'static,
    C: Send + 'static,
{
    pub icon_slider: IconSlider,
    _phantom: PhantomData<(T, C)>,
}

impl<T, C> DebouncedSlider<T, C>
where
    T: Clone + PartialEq + 'static,
    C: Send + 'static,
{
    pub fn new(
        icon_name: &str,
        min: f64,
        max: f64,
        step: f64,
        store: &ServiceStore<T>,
        tx: &Sender<C>,
        to_value: impl Fn(&T) -> f64 + 'static,
        to_cmd: impl Fn(f64) -> C + 'static,
        threshold: f64,
        data_changed: Option<impl Fn(&IconSlider, &T) + 'static>,
    ) -> Self {
        let icon_slider = IconSlider::new(icon_name, min, max, step);
        let is_updating = Rc::new(RefCell::new(false));
        let is_first = Rc::new(RefCell::new(true));

        // Store → Slider (debounced)
        let slider_c = icon_slider.clone();
        let is_updating_rx = is_updating.clone();
        let is_first_rx = is_first.clone();
        store.subscribe(move |data| {
            let value = to_value(data);
            let current = slider_c.slider.value();
            let diff = (current - value).abs();
            let first = *is_first_rx.borrow();

            if first || diff > threshold {
                *is_first_rx.borrow_mut() = false;
                *is_updating_rx.borrow_mut() = true;
                slider_c.set_value(value);
                *is_updating_rx.borrow_mut() = false;

                if let Some(ref cb) = data_changed {
                    cb(&slider_c, data);
                }
            }
        });

        // Slider → Command (guarded by is_updating)
        let tx_c = tx.clone();
        let is_updating_cmd = is_updating.clone();
        icon_slider.slider.connect_value_changed(move |s| {
            if *is_updating_cmd.borrow() {
                return;
            }
            let val = s.value();
            let _ = tx_c.try_send(to_cmd(val));
        });

        Self {
            icon_slider,
            _phantom: PhantomData,
        }
    }
}

impl<T, C> Clone for DebouncedSlider<T, C>
where
    T: Clone + PartialEq + 'static,
    C: Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            icon_slider: self.icon_slider.clone(),
            _phantom: PhantomData,
        }
    }
}
