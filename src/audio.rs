use esp_idf_hal::sys::*;

pub struct Audio {
    initialized: bool,
    sample_rate: u32,
}

impl Audio {
    pub fn new() -> Self {
        Self {
            initialized: false,
            sample_rate: 44100,
        }
    }

    pub fn init(&mut self, sample_rate: u32) -> anyhow::Result<()> {
        if self.initialized {
            anyhow::bail!("Audio already initialized");
        }

        let i2s_config = i2s_driver_config_t {
            mode: (i2s_mode_t_I2S_MODE_MASTER
                | i2s_mode_t_I2S_MODE_TX
                | i2s_mode_t_I2S_MODE_DAC_BUILT_IN) as i2s_mode_t,
            sample_rate,
            bits_per_sample: i2s_bits_per_sample_t_I2S_BITS_PER_SAMPLE_16BIT,
            channel_format: i2s_channel_fmt_t_I2S_CHANNEL_FMT_RIGHT_LEFT,
            communication_format: i2s_comm_format_t_I2S_COMM_FORMAT_I2S_MSB,
            intr_alloc_flags: 0,
            __bindgen_anon_1: i2s_driver_config_t__bindgen_ty_1 { dma_desc_num: 10 },
            __bindgen_anon_2: i2s_driver_config_t__bindgen_ty_2 {
                dma_frame_num: 1024,
            },
            use_apll: false,
            tx_desc_auto_clear: false,
            fixed_mclk: 0,
            mclk_multiple: i2s_mclk_multiple_t_I2S_MCLK_MULTIPLE_256,
            bits_per_chan: i2s_bits_per_chan_t_I2S_BITS_PER_CHAN_DEFAULT,
        };

        // SAFETY: I2S port 0 is not used by any other thread. Null queue handle
        // means no ISR queue is needed since we use blocking i2s_write.
        let ret = unsafe {
            i2s_driver_install(i2s_port_t_I2S_NUM_0, &i2s_config, 0, core::ptr::null_mut())
        };
        if ret != 0 {
            anyhow::bail!("i2s_driver_install failed: {}", ret);
        }

        // SAFETY: I2S port 0 is initialized. Null pin config uses default GPIO25/26 for DAC built-in.
        let ret = unsafe { i2s_set_pin(i2s_port_t_I2S_NUM_0, core::ptr::null()) };
        if ret != 0 {
            anyhow::bail!("i2s_set_pin failed: {}", ret);
        }

        self.sample_rate = sample_rate;
        self.initialized = true;

        let mut silence = [0i16; 128];
        for _ in 0..8 {
            self.submit_raw_i16(&mut silence, 32, 0);
        }

        Ok(())
    }

    pub fn shutdown(&mut self) {
        if !self.initialized {
            return;
        }
        // SAFETY: I2S port 0 is initialized. ESP-IDF reference counts the driver
        // — this only uninstalls if we're the last user.
        unsafe {
            i2s_set_dac_mode(i2s_dac_mode_t_I2S_DAC_CHANNEL_DISABLE);
            i2s_driver_uninstall(i2s_port_t_I2S_NUM_0);
        }
        self.initialized = false;
    }

    pub fn submit(&mut self, buf: &mut [i16], n_frames: usize, volume_pct: u8) {
        if !self.initialized {
            return;
        }
        self.submit_raw_i16(buf, n_frames, volume_pct);
    }

    pub fn pause_output(&mut self) {
        if !self.initialized {
            return;
        }
        // SAFETY: I2S port 0 is initialized, DMA buffer owned by the driver.
        // No concurrent access during pause.
        unsafe {
            i2s_zero_dma_buffer(i2s_port_t_I2S_NUM_0);
        }
        let mut silence = [0i16; 32];
        self.submit_raw_i16(&mut silence, 16, 0);
    }

    pub fn resume_output(&mut self) {}

    fn submit_raw_i16(&mut self, buf: &mut [i16], n_frames: usize, volume_pct: u8) {
        let vol = volume_pct.min(100) as f32 / 100.0;
        let n = n_frames.min(buf.len() / 2);
        if n == 0 {
            return;
        }

        for i in 0..n {
            let idx = i * 2;
            let left = buf[idx] as i32;
            let right = buf[idx + 1] as i32;
            let sample = (left + right) >> 1;
            let normalized = sample as f32 / 0x8000u32 as f32;
            let range = 254.0 * normalized * vol;

            let mut dac0: i32;
            let mut dac1: i32;
            if range > 127.0 {
                dac1 = (range - 127.0) as i32;
                dac0 = 127;
            } else if range < -127.0 {
                dac1 = (range + 127.0) as i32;
                dac0 = -127;
            } else {
                dac1 = 0;
                dac0 = range as i32;
            }

            dac0 += 0x80;
            dac1 = 0x80 - dac1;
            dac0 <<= 8;
            dac1 <<= 8;
            buf[idx] = dac1 as i16;
            buf[idx + 1] = dac0 as i16;
        }

        let to_write = 2 * n * core::mem::size_of::<i16>();
        let mut written: usize = 0;
        // SAFETY: buf is &mut [i16] owned by caller. i2s_write copies into
        // DMA buffers and blocks until the transfer completes (portMAX_DELAY).
        // The pointer is valid for the call duration.
        unsafe {
            i2s_write(
                i2s_port_t_I2S_NUM_0,
                buf.as_ptr() as *const core::ffi::c_void,
                to_write,
                &mut written,
                !0u32,
            );
        }
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        self.shutdown();
    }
}
