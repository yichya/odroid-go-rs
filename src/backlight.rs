use esp_idf_hal::gpio::Gpio14;
use esp_idf_hal::ledc::{
    config::TimerConfig, LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver, Resolution,
};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::units::Hertz;

pub struct Backlight<'d> {
    driver: LedcDriver<'d>,
    max_duty: u32,
}

impl<'d> Backlight<'d> {
    pub fn new<C: LedcChannel, T: LedcTimer + 'd>(
        channel: impl Peripheral<P = C> + 'd,
        timer: impl Peripheral<P = T> + 'd,
        pin: impl Peripheral<P = Gpio14> + 'd,
    ) -> anyhow::Result<Self>
    where
        C: LedcChannel<SpeedMode = <T as LedcTimer>::SpeedMode>,
    {
        let timer_config = TimerConfig::default()
            .frequency(Hertz(5_000))
            .resolution(Resolution::Bits13);

        let timer_driver = LedcTimerDriver::new(timer, &timer_config)?;
        let mut driver = LedcDriver::new(channel, timer_driver, pin)?;

        let max_duty = driver.get_max_duty();
        driver.set_duty(max_duty / 2)?;

        Ok(Self { driver, max_duty })
    }

    pub fn set(&mut self, percentage: u8) -> anyhow::Result<()> {
        let pct = percentage.min(100) as u32;
        let duty = self.max_duty * pct / 100;
        self.driver.set_duty(duty)?;
        Ok(())
    }
}
