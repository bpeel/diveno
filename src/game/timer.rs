#[derive(Clone, Copy)]
pub struct Timer {
    #[cfg(target_arch = "wasm32")]
    start_time: f64,
    #[cfg(not(target_arch = "wasm32"))]
    start_time: std::time::Instant,
}

impl Timer {
    #[cfg(target_arch = "wasm32")]
    fn now() -> f64 {
        web_sys::window().and_then(|w| {
            w.performance().map(|p| p.now())
        }).unwrap_or(0.0)
    }

    pub fn new() -> Timer {
        let start_time = {
            #[cfg(target_arch = "wasm32")]
            {
                Timer::now()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::time::Instant::now()
            }
        };

        Timer {
            start_time
        }
    }

    pub fn elapsed(&self) -> f32 {
        #[cfg(target_arch = "wasm32")]
        {
            (Timer::now() - self.start_time) as f32 / 1000.0
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start_time.elapsed().as_millis() as f32 / 1000.0
        }
    }
}
