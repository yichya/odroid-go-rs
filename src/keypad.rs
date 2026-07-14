use esp_idf_hal::adc::{attenuation::DB_11, AdcChannelDriver, AdcConfig, AdcDriver};
use esp_idf_hal::gpio::{
    Gpio0, Gpio13, Gpio27, Gpio32, Gpio33, Gpio34, Gpio35, Gpio36, Gpio39, Input, PinDriver, Pull,
};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::sys::{
    adc_atten_t_ADC_ATTEN_DB_11, adc_bits_width_t_ADC_WIDTH_BIT_12, adc_unit_t_ADC_UNIT_1,
    esp_adc_cal_characteristics_t, esp_adc_cal_characterize, esp_adc_cal_raw_to_voltage,
};

pub const KEY_UP: u16 = 1 << 0;
pub const KEY_RIGHT: u16 = 1 << 1;
pub const KEY_DOWN: u16 = 1 << 2;
pub const KEY_LEFT: u16 = 1 << 3;
pub const KEY_SELECT: u16 = 1 << 4;
pub const KEY_START: u16 = 1 << 5;
pub const KEY_A: u16 = 1 << 6;
pub const KEY_B: u16 = 1 << 7;
pub const KEY_MENU: u16 = 1 << 8;
pub const KEY_VOLUME: u16 = 1 << 9;

const JOY_THRESHOLD_HIGH: u16 = 3072;
const JOY_THRESHOLD_LOW: u16 = 1024;

pub struct Keypad<'d> {
    a: PinDriver<'d, Gpio32, Input>,
    b: PinDriver<'d, Gpio33, Input>,
    select: PinDriver<'d, Gpio27, Input>,
    start: PinDriver<'d, Gpio39, Input>,
    menu: PinDriver<'d, Gpio13, Input>,
    volume: PinDriver<'d, Gpio0, Input>,
    adc: AdcDriver<'d, esp_idf_hal::adc::ADC1>,
    adc_x: AdcChannelDriver<'d, { DB_11 }, Gpio34>,
    adc_y: AdcChannelDriver<'d, { DB_11 }, Gpio35>,
    adc_bat: AdcChannelDriver<'d, { DB_11 }, Gpio36>,
    adc_cal: esp_adc_cal_characteristics_t,
    // C-style two-sample vertical counter debounce state
    state: u16,
    cnt0: u16,
    cnt1: u16,
}

impl<'d> Keypad<'d> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        adc1: impl Peripheral<P = esp_idf_hal::adc::ADC1> + 'd,
        a: impl Peripheral<P = Gpio32> + 'd,
        b: impl Peripheral<P = Gpio33> + 'd,
        select: impl Peripheral<P = Gpio27> + 'd,
        start: impl Peripheral<P = Gpio39> + 'd,
        menu: impl Peripheral<P = Gpio13> + 'd,
        volume: impl Peripheral<P = Gpio0> + 'd,
        joy_x: impl Peripheral<P = Gpio34> + 'd,
        joy_y: impl Peripheral<P = Gpio35> + 'd,
        bat: impl Peripheral<P = Gpio36> + 'd,
    ) -> anyhow::Result<Self> {
        // ADC first — so digital pin setup isn't clobbered
        let adc = AdcDriver::new(adc1, &AdcConfig::new())?;
        let adc_x = AdcChannelDriver::new(joy_x)?;
        let adc_y = AdcChannelDriver::new(joy_y)?;
        let adc_bat = AdcChannelDriver::new(bat)?;

        // Digital buttons after ADC
        let mut a_pin = PinDriver::input(a)?;
        a_pin.set_pull(Pull::Up)?;
        let mut b_pin = PinDriver::input(b)?;
        b_pin.set_pull(Pull::Up)?;
        let mut select_pin = PinDriver::input(select)?;
        select_pin.set_pull(Pull::Up)?;
        let start_pin = PinDriver::input(start)?;
        let mut menu_pin = PinDriver::input(menu)?;
        menu_pin.set_pull(Pull::Up)?;
        let volume_pin = PinDriver::input(volume)?;

        let mut kp = Self {
            a: a_pin,
            b: b_pin,
            select: select_pin,
            start: start_pin,
            menu: menu_pin,
            volume: volume_pin,
            adc,
            adc_x,
            adc_y,
            adc_bat,
            adc_cal: Default::default(),
            state: 0,
            cnt0: 0,
            cnt1: 0,
        };

        // SAFETY: esp_adc_cal_characterize fills kp.adc_cal from eFuse-calibrated
        // ADC characteristics. kp.adc_cal is valid for the lifetime of Keypad.
        // All args are constants for ADC1_CH0 with 11dB attenuation.
        unsafe {
            let cal_type = esp_adc_cal_characterize(
                adc_unit_t_ADC_UNIT_1,
                adc_atten_t_ADC_ATTEN_DB_11,
                adc_bits_width_t_ADC_WIDTH_BIT_12,
                1100,
                &mut kp.adc_cal,
            );
            let cal_name = match cal_type {
                0 => "efuse VREF",
                1 => "efuse two-point",
                2 => "default VREF",
                _ => "unknown",
            };
            log::info!("Battery ADC calibration: {} ({})", cal_name, cal_type);
        }

        // Initialize state to current reading to avoid false edge on first poll
        kp.state = kp.sample_raw()?;

        Ok(kp)
    }

    fn sample_raw(&mut self) -> anyhow::Result<u16> {
        let mut mask = 0u16;

        if self.a.is_low() {
            mask |= KEY_A;
        }
        if self.b.is_low() {
            mask |= KEY_B;
        }
        if self.select.is_low() {
            mask |= KEY_SELECT;
        }
        if self.start.is_low() {
            mask |= KEY_START;
        }
        if self.menu.is_low() {
            mask |= KEY_MENU;
        }
        if self.volume.is_low() {
            mask |= KEY_VOLUME;
        }

        let x: u16 = self.adc.read_raw(&mut self.adc_x)?;
        let y: u16 = self.adc.read_raw(&mut self.adc_y)?;

        if x > JOY_THRESHOLD_HIGH {
            mask |= KEY_LEFT;
        } else if x > JOY_THRESHOLD_LOW {
            mask |= KEY_RIGHT;
        }
        if y > JOY_THRESHOLD_HIGH {
            mask |= KEY_UP;
        } else if y > JOY_THRESHOLD_LOW {
            mask |= KEY_DOWN;
        }

        Ok(mask)
    }

    pub fn debug_read(&mut self) -> anyhow::Result<()> {
        let a = self.a.is_low();
        let b = self.b.is_low();
        let sel = self.select.is_low();
        let start = self.start.is_low();
        let menu = self.menu.is_low();
        let vol = self.volume.is_low();
        let x: u16 = self.adc.read_raw(&mut self.adc_x)?;
        let y: u16 = self.adc.read_raw(&mut self.adc_y)?;
        log::info!(
            "KP: A={} B={} SEL={} STA={} MENU={} VOL={} X={} Y={}",
            a,
            b,
            sel,
            start,
            menu,
            vol,
            x,
            y
        );
        Ok(())
    }

    pub fn read_battery_mv(&mut self) -> anyhow::Result<u32> {
        let raw = self.adc.read_raw(&mut self.adc_bat)?;
        // SAFETY: adc_cal was initialized by esp_adc_cal_characterize. raw value
        // is from the same ADC unit/attenuation as the calibration struct.
        let voltage = unsafe { esp_adc_cal_raw_to_voltage(raw as u32, &self.adc_cal) };
        Ok(voltage * 2) // 2:1 voltage divider
    }

    #[allow(dead_code)]
    pub fn read_battery_raw(&mut self) -> anyhow::Result<u16> {
        let raw = self.adc.read_raw(&mut self.adc_bat)?;
        Ok(raw)
    }

    /// Exact port of C keypad_debounce — vertical counter algorithm.
    /// Returns `(stable_state, edge_changes)`.
    pub fn poll(&mut self) -> anyhow::Result<(u16, u16)> {
        let sample = self.sample_raw()?;

        let delta = sample ^ self.state;
        // Original C 4-sample vertical counter (Gray code: 00→01→11→10→00→...)
        self.cnt1 = (self.cnt1 ^ self.cnt0) & delta;
        self.cnt0 = !self.cnt0 & delta;
        // Toggle when counter completes full cycle (back to 00)
        let toggle = delta & !(self.cnt0 | self.cnt1);
        self.state ^= toggle;

        Ok((self.state, toggle))
    }
}
