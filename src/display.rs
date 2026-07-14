use core::borrow::Borrow;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{Gpio21, Output, PinDriver};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::spi::SpiDeviceDriver;
use esp_idf_hal::spi::SpiDriver;
use log::info;

use crate::gbuf::{self, FrameBuffer};

struct IliCmd {
    cmd: u8,
    data: &'static [u8],
    delay_ms: u16,
}

const ILI9341_INIT_CMDS: &[IliCmd] = &[
    IliCmd {
        cmd: 0x01,
        data: &[],
        delay_ms: 100,
    },
    IliCmd {
        cmd: 0xCF,
        data: &[0x00, 0xC3, 0x30],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xED,
        data: &[0x64, 0x03, 0x12, 0x81],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xE8,
        data: &[0x85, 0x00, 0x78],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xCB,
        data: &[0x39, 0x2C, 0x00, 0x34, 0x02],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xF7,
        data: &[0x20],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xEA,
        data: &[0x00, 0x00],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xC0,
        data: &[0x1B],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xC1,
        data: &[0x12],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xC5,
        data: &[0x32, 0x3C],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xC7,
        data: &[0x91],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0x36,
        data: &[0xA8],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0x3A,
        data: &[0x55],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xB1,
        data: &[0x00, 0x1B],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xB6,
        data: &[0x0A, 0xA2],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xF6,
        data: &[0x01, 0x30],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xF2,
        data: &[0x00],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0x26,
        data: &[0x01],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xE0,
        data: &[
            0x0F, 0x31, 0x2B, 0x0C, 0x0E, 0x08, 0x4E, 0xF1, 0x37, 0x07, 0x10, 0x03, 0x0E, 0x09,
            0x00,
        ],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0xE1,
        data: &[
            0x00, 0x0E, 0x14, 0x03, 0x11, 0x07, 0x31, 0xC1, 0x48, 0x08, 0x0F, 0x0C, 0x31, 0x36,
            0x0F,
        ],
        delay_ms: 0,
    },
    IliCmd {
        cmd: 0x11,
        data: &[],
        delay_ms: 100,
    },
    IliCmd {
        cmd: 0x29,
        data: &[],
        delay_ms: 100,
    },
];

pub struct Display<'d, T>
where
    T: Borrow<SpiDriver<'d>> + 'd,
{
    spi: SpiDeviceDriver<'d, T>,
    dc: PinDriver<'d, Gpio21, Output>,
}

impl<'d, T> Display<'d, T>
where
    T: Borrow<SpiDriver<'d>> + 'd,
{
    pub fn new(
        spi_device: SpiDeviceDriver<'d, T>,
        dc: impl Peripheral<P = Gpio21> + 'd,
    ) -> anyhow::Result<Self> {
        let dc_pin = PinDriver::output(dc)?;

        let mut display = Self {
            spi: spi_device,
            dc: dc_pin,
        };

        display.init()?;
        Ok(display)
    }

    fn init(&mut self) -> anyhow::Result<()> {
        for cmd in ILI9341_INIT_CMDS {
            self.cmd_data(cmd.cmd, cmd.data)?;
            if cmd.delay_ms > 0 {
                FreeRtos::delay_ms(cmd.delay_ms as u32);
            }
        }
        info!("ILI9341 initialized");
        Ok(())
    }

    fn cmd_data(&mut self, cmd: u8, data: &[u8]) -> anyhow::Result<()> {
        self.dc.set_low()?;
        self.spi.write(&[cmd])?;

        if !data.is_empty() {
            self.dc.set_high()?;
            self.spi.write(data)?;
        }

        Ok(())
    }

    pub fn update(&mut self, fb: &FrameBuffer) -> anyhow::Result<()> {
        let x_end: u16 = (gbuf::DISPLAY_WIDTH - 1) as u16;
        let y_end: u16 = (gbuf::DISPLAY_HEIGHT - 1) as u16;

        self.cmd_data(0x2A, &[0, 0, (x_end >> 8) as u8, x_end as u8])?;

        self.cmd_data(0x2B, &[0, 0, (y_end >> 8) as u8, y_end as u8])?;

        self.dc.set_low()?;
        self.spi.write(&[0x2C])?;

        self.dc.set_high()?;
        self.spi.write(fb.as_u8_slice())?;

        Ok(())
    }
}
